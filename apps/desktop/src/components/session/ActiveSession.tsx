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
  Check
} from "lucide-react";
import { invoke, listen } from "@/lib/ipc";
import { BouncingLoader } from "../ui/bouncing-loader";
import { motion, AnimatePresence } from "framer-motion";

export function ActiveSession({ 
  deviceId, 
  streamUrl, 
  onDisconnect 
}: { 
  deviceId: string; 
  streamUrl: string; 
  onDisconnect: () => void; 
}) {
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [connectionState, setConnectionState] = useState<"connected" | "reconnecting" | "failed">("connected");
  const [isAudioMuted, setIsAudioMuted] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [monitorIndex, setMonitorIndex] = useState(0);
  const [showToolbar, setShowToolbar] = useState(true);
  const [clipboardCopied, setClipboardCopied] = useState(false);
  
  const toolbarTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

    const handleMouseMove = (e: MouseEvent) => {
      handleUserActivity();
      invoke("send_input_event", { 
        event: { MouseMove: { x: Math.floor(e.clientX), y: Math.floor(e.clientY) } } 
      }).catch(() => {}); // silent catch for high frequency mouse stream
    };

    const handleMouseDown = (e: MouseEvent) => {
      handleUserActivity();
      invoke("send_input_event", { 
        event: { MouseButton: { button: e.button, down: true } } 
      }).catch(console.error);
    };

    const handleMouseUp = (e: MouseEvent) => {
      handleUserActivity();
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

    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    document.addEventListener("fullscreenchange", handleFullscreenChange);

    return () => {
      unlisten.then(f => f());
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, []);

  const handleSwitchMonitor = () => {
    const nextIndex = monitorIndex === 0 ? 1 : 0;
    setMonitorIndex(nextIndex);
    invoke("switch_monitor", { index: nextIndex }).catch(console.error);
  };

  const handleToggleAudio = () => {
    setIsAudioMuted(!isAudioMuted);
    invoke("toggle_audio", { muted: !isAudioMuted }).catch(console.error);
  };

  const handleToggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().catch(console.error);
    } else {
      document.exitFullscreen().catch(console.error);
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
      {/* Video Streaming Canvas */}
      <div className={`w-full h-full flex items-center justify-center transition-opacity duration-500 ${connectionState !== "connected" ? "opacity-30 blur-sm" : "opacity-100"}`}>
        <img 
          src={streamUrl} 
          className="w-full h-full object-contain pointer-events-none" 
          alt={`Video Stream from ${deviceId}`} 
        />
      </div>

      {/* Top Floating Status Pill */}
      <div className="absolute top-4 right-4 z-40 flex items-center gap-2 pointer-events-auto">
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
        <div className="z-50 pointer-events-auto">
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
