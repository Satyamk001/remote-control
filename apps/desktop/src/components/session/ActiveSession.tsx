import { useState, useEffect, useRef } from "react";
import { Button } from "../ui/button";
import { DiagnosticsPanel } from "../diagnostics/DiagnosticsPanel";
import { 
  Monitor, 
  Volume2, 
  VolumeX, 
  ClipboardPaste, 
  Maximize, 
  Minimize, 
  X, 
  Activity, 
  WifiOff, 
  Check,
  Upload
} from "lucide-react";
import { invoke, listen } from "@/lib/ipc";
import { BouncingLoader } from "../ui/bouncing-loader";
import { motion, AnimatePresence } from "framer-motion";

export function ActiveSession({ 
  deviceId, 
  streamUrl, 
  audioUrl,
  onDisconnect 
}: { 
  deviceId: string; 
  streamUrl: string; 
  audioUrl: string;
  onDisconnect: () => void; 
}) {
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [connectionState, setConnectionState] = useState<"connected" | "reconnecting" | "failed">("connected");
  const [isAudioMuted, setIsAudioMuted] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [monitorIndex, setMonitorIndex] = useState(0);
  const [showToolbar, setShowToolbar] = useState(true);
  const [clipboardCopied, setClipboardCopied] = useState(false);
  // Visible diagnostic of the video decode pipeline so a black canvas is
  // explainable without devtools. Cleared once the first frame paints.
  const [videoStatus, setVideoStatus] = useState<string>("Waiting for video…");
  const [firstFramePainted, setFirstFramePainted] = useState(false);
  
  const toolbarTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  // Intrinsic resolution of the decoded remote frames, updated as frames arrive.
  const videoSizeRef = useRef<{ w: number; h: number }>({ w: 0, h: 0 });
  // Mirrors isAudioMuted for the long-lived audio effect to read live.
  const mutedRef = useRef<boolean>(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [uploading, setUploading] = useState(false);

  // Map a viewport cursor position to normalized 0..65535 absolute coordinates
  // over the remote desktop, accounting for `object-contain` letterboxing of the
  // canvas. Returns null when the cursor is outside the visible video content
  // area (i.e. over the black letterbox bars), so we don't inject off-screen
  // moves.
  const toRemoteCoords = (clientX: number, clientY: number): { x: number; y: number } | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;

    const rect = canvas.getBoundingClientRect();
    const natW = videoSizeRef.current.w || canvas.width;
    const natH = videoSizeRef.current.h || canvas.height;
    if (natW === 0 || natH === 0 || rect.width === 0 || rect.height === 0) return null;

    // object-contain: scale uniformly to fit, then center.
    const scale = Math.min(rect.width / natW, rect.height / natH);
    const dispW = natW * scale;
    const dispH = natH * scale;
    const offsetX = rect.left + (rect.width - dispW) / 2;
    const offsetY = rect.top + (rect.height - dispH) / 2;

    const relX = clientX - offsetX;
    const relY = clientY - offsetY;

    // Outside the actual video content -> ignore.
    if (relX < 0 || relY < 0 || relX > dispW || relY > dispH) return null;

    const normX = Math.round((relX / dispW) * 65535);
    const normY = Math.round((relY / dispH) * 65535);
    return {
      x: Math.max(0, Math.min(65535, normX)),
      y: Math.max(0, Math.min(65535, normY)),
    };
  };

  // WebCodecs H.264 decode pipeline.
  //
  // The host streams length-prefixed Annex-B access units over `streamUrl`
  // (`[u8 keyframe_flag][u32 le len][payload]`). We read the response body as a
  // stream, reassemble each access unit, feed it to a VideoDecoder, and paint
  // decoded frames onto the canvas. This replaces the previous <img> MJPEG path
  // and gives real H.264 decoding (WebView2/Chromium 94+).
  useEffect(() => {
    if (!streamUrl) return;
    const AnyWin = window as any;
    if (typeof AnyWin.VideoDecoder === "undefined") {
      console.error("WebCodecs VideoDecoder unavailable in this runtime.");
      setVideoStatus("WebCodecs VideoDecoder unavailable in this WebView.");
      return;
    }

    let cancelled = false;
    const abort = new AbortController();
    let decoder: any = null;
    let configured = false;
    // Decode-latency tracking (approximate, FIFO): submit time of the most
    // recent chunk, and a throttle so we only report ~once/sec.
    let lastSubmitMs = 0;
    let lastReportMs = 0;

    const ctx = canvasRef.current?.getContext("2d") ?? null;

    const paint = (frame: any) => {
      try {
        const canvas = canvasRef.current;
        const w = frame.displayWidth || frame.codedWidth;
        const h = frame.displayHeight || frame.codedHeight;
        if (canvas && (canvas.width !== w || canvas.height !== h)) {
          canvas.width = w;
          canvas.height = h;
        }
        videoSizeRef.current = { w, h };
        if (ctx) ctx.drawImage(frame, 0, 0);
        if (!firstFramePainted) setFirstFramePainted(true);

        // Report decode latency at most once per second.
        const nowMs = performance.now();
        if (lastSubmitMs > 0 && nowMs - lastReportMs > 1000) {
          lastReportMs = nowMs;
          const micros = Math.max(0, Math.round((nowMs - lastSubmitMs) * 1000));
          invoke("report_decode_latency", { micros }).catch(() => {});
        }
      } finally {
        frame.close();
      }
    };

    const ensureDecoder = () => {
      if (decoder) return;
      decoder = new AnyWin.VideoDecoder({
        output: paint,
        error: (e: any) => {
          console.error("VideoDecoder error:", e);
          setVideoStatus("Decoder error: " + (e?.message || e));
        },
      });
    };

    const run = async () => {
      try {
        setVideoStatus("Connecting to video stream…");
        const resp = await fetch(streamUrl, { signal: abort.signal });
        if (!resp.body) { setVideoStatus("Video stream has no body."); return; }
        const reader = resp.body.getReader();
        let totalBytes = 0;
        let sawFirstKey = false;

        // Rolling buffer across chunk boundaries.
        let buffer = new Uint8Array(0);
        const append = (chunk: Uint8Array) => {
          const merged = new Uint8Array(buffer.length + chunk.length);
          merged.set(buffer, 0);
          merged.set(chunk, buffer.length);
          buffer = merged;
        };

        while (!cancelled) {
          const { done, value } = await reader.read();
          if (done) {
            // Host ended the stream. Clear the frozen last frame and tell the user.
            if (ctx && canvasRef.current) {
              ctx.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
            }
            setFirstFramePainted(false);
            setVideoStatus("Remote stream ended (host disconnected).");
            break;
          }
          if (value) { append(value); totalBytes += value.length;
            if (!sawFirstKey) setVideoStatus(`Receiving stream (${totalBytes} bytes), waiting for keyframe…`);
          }

          // Parse as many complete [flag][len][payload] units as available.
          while (buffer.length >= 5) {
            const flag = buffer[0];
            const len =
              buffer[1] | (buffer[2] << 8) | (buffer[3] << 16) | (buffer[4] << 24);
            if (len < 0 || len > 50_000_000) {
              // Corrupt framing; drop the connection.
              return;
            }
            if (buffer.length < 5 + len) break; // wait for more bytes
            const payload = buffer.slice(5, 5 + len);
            buffer = buffer.slice(5 + len);

            const isKey = flag === 1;
            ensureDecoder();

            // Configure on the first keyframe. avc1.42e01f = Baseline L3.1;
            // the decoder reads actual params from the in-band SPS/PPS.
            if (!configured) {
              if (!isKey) continue; // must start at a keyframe
              sawFirstKey = true;
              try {
                decoder.configure({
                  codec: "avc1.42e01f",
                  optimizeForLatency: true,
                });
                configured = true;
                setVideoStatus("Keyframe received, decoding…");
              } catch (e: any) {
                console.error("configure() failed:", e);
                setVideoStatus("Decoder configure failed: " + (e?.message || e));
                return;
              }
            }

            try {
              const chunk = new AnyWin.EncodedVideoChunk({
                type: isKey ? "key" : "delta",
                timestamp: performance.now() * 1000,
                data: payload,
              });
              lastSubmitMs = performance.now();
              decoder.decode(chunk);
            } catch (e) {
              console.error("decode() failed:", e);
            }
          }
        }
      } catch (e: any) {
        if (!cancelled) {
          console.error("H.264 stream error:", e);
          setVideoStatus("Stream error: " + (e?.message || e));
        }
      }
    };

    run();

    return () => {
      cancelled = true;
      abort.abort();
      try {
        if (decoder && decoder.state !== "closed") decoder.close();
      } catch {
        /* ignore */
      }
    };
  }, [streamUrl]);

  // Web Audio playback of the host's system audio.
  //
  // The host first sends a format header ("RCAF" + u32 LE sample_rate + u16 LE
  // channels), then a continuous stream of interleaved f32 LE PCM. We parse the
  // header to configure the AudioContext (sample rate) and channel
  // deinterleaving instead of assuming a fixed format, then schedule small
  // AudioBuffers back-to-back for gapless playback. Muting is handled host-side
  // (it stops sending) and locally here as a fast path.
  useEffect(() => {
    if (!audioUrl) return;

    let cancelled = false;
    const abort = new AbortController();
    const AudioCtx = (window as any).AudioContext || (window as any).webkitAudioContext;
    if (!AudioCtx) {
      console.error("Web Audio API unavailable.");
      return;
    }

    let audioCtx: any = null;
    let sampleRate = 48000;
    let channels = 2;
    let playHead = 0;

    const schedule = (interleaved: Float32Array) => {
      if (mutedRef.current || !audioCtx) return;
      const frames = Math.floor(interleaved.length / channels);
      if (frames === 0) return;
      const buffer = audioCtx.createBuffer(channels, frames, sampleRate);
      for (let ch = 0; ch < channels; ch++) {
        const out = buffer.getChannelData(ch);
        for (let i = 0; i < frames; i++) out[i] = interleaved[i * channels + ch];
      }
      const src = audioCtx.createBufferSource();
      src.buffer = buffer;
      src.connect(audioCtx.destination);
      const now = audioCtx.currentTime;
      if (playHead < now) playHead = now;
      src.start(playHead);
      playHead += buffer.duration;
    };

    const run = async () => {
      try {
        const resp = await fetch(audioUrl, { signal: abort.signal });
        if (!resp.body) return;
        const reader = resp.body.getReader();

        let headerParsed = false;
        let leftover = new Uint8Array(0);

        while (!cancelled) {
          const { done, value } = await reader.read();
          if (done) break;
          if (!value) continue;

          let merged = new Uint8Array(leftover.length + value.length);
          merged.set(leftover, 0);
          merged.set(value, leftover.length);

          // Parse the 10-byte "RCAF" header before any PCM.
          if (!headerParsed) {
            if (merged.length < 10) {
              leftover = merged;
              continue;
            }
            const magicOk =
              merged[0] === 0x52 && merged[1] === 0x43 && merged[2] === 0x41 && merged[3] === 0x46; // "RCAF"
            if (magicOk) {
              const dv = new DataView(merged.buffer, merged.byteOffset, 10);
              sampleRate = dv.getUint32(4, true) || 48000;
              channels = dv.getUint16(8, true) || 2;
              merged = merged.slice(10);
            } else {
              console.warn("Audio header magic missing; assuming 48kHz/2ch.");
            }
            audioCtx = new AudioCtx({ sampleRate });
            playHead = audioCtx.currentTime;
            headerParsed = true;
          }

          const frameBytes = 4 * channels; // one interleaved frame
          const usable = merged.length - (merged.length % frameBytes);
          if (usable > 0) {
            const slice = merged.slice(0, usable);
            const floats = new Float32Array(slice.buffer, slice.byteOffset, usable / 4);
            schedule(floats);
          }
          leftover = merged.slice(usable);
        }
      } catch (e) {
        if (!cancelled) console.error("Audio stream error:", e);
      }
    };

    run();

    return () => {
      cancelled = true;
      abort.abort();
      if (audioCtx) audioCtx.close().catch(() => {});
    };
  }, [audioUrl]);

  // Auto-hide toolbar on mouse inactivity
  const handleUserActivity = () => {
    setShowToolbar(true);
    if (toolbarTimeoutRef.current) {
      clearTimeout(toolbarTimeoutRef.current);
    }
    toolbarTimeoutRef.current = setTimeout(() => {
      setShowToolbar(false);
    }, 3500);
  };

  useEffect(() => {
    handleUserActivity();
    return () => {
      if (toolbarTimeoutRef.current) clearTimeout(toolbarTimeoutRef.current);
    };
  }, []);

  // Listen for WebRTC state and bind input capture
  useEffect(() => {
    const unlisten = listen("connection_state_changed", (event: any) => {
      const state = event.payload?.state;
      if (state) setConnectionState(state);
    });

    // Only forward input that happens over the remote video surface — never
    // when interacting with our own UI chrome (toolbar, HUD, overlays). Without
    // this, clicking a toolbar button also injects a click into the host
    // desktop (on loopback that opens File Explorer / taskbar items).
    const isOverUiChrome = (target: EventTarget | null): boolean => {
      let el = target as HTMLElement | null;
      while (el) {
        if (el.dataset && el.dataset.rcUi === "1") return true;
        el = el.parentElement;
      }
      return false;
    };

    const handleMouseMove = (e: MouseEvent) => {
      handleUserActivity();
      if (isOverUiChrome(e.target)) return;
      const coords = toRemoteCoords(e.clientX, e.clientY);
      if (!coords) return;
      invoke("send_input_event", {
        event: { MouseMove: { x: coords.x, y: coords.y } }
      }).catch(() => {}); // silent catch for high frequency mouse stream
    };

    const handleMouseDown = (e: MouseEvent) => {
      handleUserActivity();
      if (isOverUiChrome(e.target)) return;
      invoke("send_input_event", { 
        event: { MouseButton: { button: e.button, down: true } } 
      }).catch(console.error);
    };

    const handleMouseUp = (e: MouseEvent) => {
      handleUserActivity();
      if (isOverUiChrome(e.target)) return;
      invoke("send_input_event", { 
        event: { MouseButton: { button: e.button, down: false } } 
      }).catch(console.error);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F11") return;
      invoke("send_input_event", { 
        event: { KeyPress: { key_code: e.keyCode, down: true } } 
      }).catch(console.error);
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === "F11") return;
      invoke("send_input_event", { 
        event: { KeyPress: { key_code: e.keyCode, down: false } } 
      }).catch(console.error);
    };

    const handleWheel = (e: WheelEvent) => {
      handleUserActivity();
      if (isOverUiChrome(e.target)) return;
      // Normalize wheel deltas to notch counts. deltaY > 0 means scrolling down,
      // which corresponds to a negative wheel delta on Windows.
      const dy = e.deltaY > 0 ? -1 : e.deltaY < 0 ? 1 : 0;
      const dx = e.deltaX > 0 ? 1 : e.deltaX < 0 ? -1 : 0;
      if (dx === 0 && dy === 0) return;
      invoke("send_input_event", {
        event: { Scroll: { delta_x: dx, delta_y: dy } }
      }).catch(() => {});
    };

    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("wheel", handleWheel, { passive: true });
    document.addEventListener("fullscreenchange", handleFullscreenChange);

    return () => {
      unlisten.then(f => f());
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("wheel", handleWheel);
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, []);

  const handleSwitchMonitor = () => {
    const nextIndex = monitorIndex === 0 ? 1 : 0;
    setMonitorIndex(nextIndex);
    invoke("switch_monitor", { index: nextIndex }).catch(console.error);
  };

  const handleToggleAudio = () => {
    const next = !isAudioMuted;
    setIsAudioMuted(next);
    mutedRef.current = next;
    invoke("toggle_audio", { muted: next }).catch(console.error);
  };

  const handleToggleFullscreen = async () => {
    // WebView2 does not reliably support the HTML Fullscreen API, so drive the
    // native Tauri window instead.
    try {
      const { appWindow } = await import("@tauri-apps/api/window");
      const now = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!now);
      setIsFullscreen(!now);
    } catch (e) {
      console.error("Fullscreen toggle failed:", e);
    }
  };

  const handleFilePicked = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-selecting the same file later
    if (!file) return;
    try {
      setUploading(true);
      const buf = await file.arrayBuffer();
      // Tauri IPC serializes number[] for Vec<u8>.
      const data = Array.from(new Uint8Array(buf));
      await invoke("send_file", { filename: file.name, data });
    } catch (err) {
      console.error("File upload failed:", err);
    } finally {
      setUploading(false);
    }
  };

  const handleSyncClipboard = async () => {
    try {
      const text = await navigator.clipboard.readText();
      await invoke("sync_clipboard", { text });
      setClipboardCopied(true);
      setTimeout(() => setClipboardCopied(false), 2000);
    } catch (err) {
      console.error("Failed to read clipboard:", err);
    }
  };

  const handleDisconnect = () => {
    invoke("terminate_host_session").catch(console.error);
    onDisconnect();
  };

  return (
    <div 
      className="fixed inset-0 bg-black flex items-center justify-center overflow-hidden select-none cursor-crosshair"
      onMouseMove={handleUserActivity}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* Video Streaming Canvas (WebCodecs H.264 decode target) */}
      <div className={`w-full h-full flex items-center justify-center transition-opacity duration-500 ${connectionState !== "connected" ? "opacity-30 blur-sm" : "opacity-100"}`}>
        <canvas
          ref={canvasRef}
          className="w-full h-full object-contain pointer-events-none"
          aria-label={`Video Stream from ${deviceId}`}
        />
      </div>

      {/* Video pipeline status (hidden once the first frame paints) */}
      {!firstFramePainted && (
        <div className="absolute inset-0 z-30 flex items-center justify-center pointer-events-none">
          <div className="bg-black/70 border border-border/50 rounded-lg px-4 py-3 text-center max-w-md">
            <p className="text-sm font-mono text-amber-300">{videoStatus}</p>
            <p className="text-[10px] text-muted-foreground mt-1">Video pipeline diagnostic</p>
          </div>
        </div>
      )}

      {/* Top Floating Status Pill */}
      <div data-rc-ui="1" className="absolute top-4 right-4 z-40 flex items-center gap-2 pointer-events-auto">
        <button
          onClick={() => setShowDiagnostics(!showDiagnostics)}
          className="flex items-center gap-2 bg-background/80 hover:bg-background/95 backdrop-blur-md border border-border/60 rounded-full px-3 py-1.5 text-xs font-mono text-foreground shadow-lg transition-all active:scale-95"
          title="Toggle telemetry HUD"
        >
          <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="font-semibold text-emerald-400">60 FPS</span>
          <span className="text-muted-foreground">·</span>
          <span className="text-muted-foreground">&lt; 35ms RTT</span>
          <Activity className="h-3.5 w-3.5 text-primary ml-1" />
        </button>
      </div>

      {/* Reconnecting Overlay */}
      <AnimatePresence>
        {connectionState === "reconnecting" && (
          <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md"
          >
            <div className="flex flex-col items-center space-y-6 text-center max-w-sm p-6">
              <BouncingLoader />
              <div className="space-y-2">
                <p className="text-xl font-bold tracking-widest text-primary animate-pulse uppercase">
                  Re-establishing Tunnel
                </p>
                <p className="text-xs text-muted-foreground">
                  P2P media stream interrupted. Attempting ICE restart negotiation...
                </p>
              </div>
            </div>
          </motion.div>
        )}

        {/* Failed Connection Overlay */}
        {connectionState === "failed" && (
          <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md p-6"
          >
            <div className="bg-card/80 border border-destructive/40 rounded-2xl p-8 max-w-md w-full text-center space-y-6 shadow-2xl">
              <div className="w-14 h-14 rounded-full bg-destructive/10 border border-destructive/20 text-destructive flex items-center justify-center mx-auto">
                <WifiOff className="w-7 h-7" />
              </div>
              <div>
                <h3 className="text-xl font-bold text-foreground">Connection Terminated</h3>
                <p className="text-xs text-muted-foreground mt-2">
                  The WebRTC peer connection to {deviceId.substring(0, 12)}... could not be maintained.
                </p>
              </div>
              <div className="flex gap-3">
                <Button 
                  onClick={handleDisconnect} 
                  variant="default" 
                  className="flex-1"
                >
                  Return to Dashboard
                </Button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Diagnostics HUD Panel */}
      {showDiagnostics && (
        <div data-rc-ui="1" className="z-50 pointer-events-auto">
          <DiagnosticsPanel />
        </div>
      )}

      {/* Auto-Hiding Floating Control Dock */}
      <AnimatePresence>
        {showToolbar && (
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            transition={{ duration: 0.2 }}
            data-rc-ui="1"
            className="absolute bottom-6 left-1/2 -translate-x-1/2 z-40 pointer-events-auto flex items-center gap-1.5 bg-background/85 border border-border/60 rounded-full px-3.5 py-1.5 backdrop-blur-lg shadow-2xl transition-all"
            onMouseEnter={() => {
              if (toolbarTimeoutRef.current) clearTimeout(toolbarTimeoutRef.current);
            }}
          >
            {/* Monitor Switcher */}
            <Button 
              variant="ghost" 
              size="icon" 
              className="text-foreground hover:bg-secondary rounded-full h-9 w-9 relative"
              onClick={handleSwitchMonitor} 
              title={`Switch Display (Current: Display ${monitorIndex + 1})`}
            >
              <Monitor className="h-4 w-4" />
              <span className="absolute -top-1 -right-1 text-[9px] font-bold bg-primary text-primary-foreground rounded-full w-4 h-4 flex items-center justify-center">
                {monitorIndex + 1}
              </span>
            </Button>

            {/* Audio Toggle */}
            <Button 
              variant="ghost" 
              size="icon" 
              className={`rounded-full h-9 w-9 ${isAudioMuted ? "text-muted-foreground hover:bg-secondary" : "text-foreground hover:bg-secondary"}`}
              onClick={handleToggleAudio}
              title={isAudioMuted ? "Unmute Audio" : "Mute Audio"}
            >
              {isAudioMuted ? <VolumeX className="h-4 w-4 text-muted-foreground" /> : <Volume2 className="h-4 w-4" />}
            </Button>

            {/* Clipboard Sync */}
            <Button 
              variant="ghost" 
              size="icon" 
              className="text-foreground hover:bg-secondary rounded-full h-9 w-9"
              onClick={handleSyncClipboard}
              title="Sync Local Clipboard to Remote"
            >
              {clipboardCopied ? <Check className="h-4 w-4 text-emerald-400" /> : <ClipboardPaste className="h-4 w-4" />}
            </Button>

            {/* File Upload (viewer -> host) */}
            <input
              ref={fileInputRef}
              type="file"
              className="hidden"
              onChange={handleFilePicked}
            />
            <Button
              variant="ghost"
              size="icon"
              className="text-foreground hover:bg-secondary rounded-full h-9 w-9"
              onClick={() => fileInputRef.current?.click()}
              disabled={uploading}
              title="Send a file to the remote machine"
            >
              <Upload className={`h-4 w-4 ${uploading ? "animate-pulse text-primary" : ""}`} />
            </Button>

            <div className="w-px h-5 bg-border/60 mx-1" />

            {/* Fullscreen Toggle */}
            <Button 
              variant="ghost" 
              size="icon" 
              className="text-foreground hover:bg-secondary rounded-full h-9 w-9"
              onClick={handleToggleFullscreen}
              title={isFullscreen ? "Exit Fullscreen" : "Enter Fullscreen (F11)"}
            >
              {isFullscreen ? <Minimize className="h-4 w-4" /> : <Maximize className="h-4 w-4" />}
            </Button>

            <div className="w-px h-5 bg-border/60 mx-1" />

            {/* Disconnect Button */}
            <Button 
              variant="destructive" 
              size="icon" 
              className="rounded-full h-9 w-9 bg-red-600 hover:bg-red-700 shadow-md shadow-red-600/20"
              onClick={handleDisconnect} 
              title="Disconnect Remote Session"
            >
              <X className="h-4 w-4" />
            </Button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
