import { useState, useEffect } from "react";
import { Button } from "../ui/button";
import { DiagnosticsPanel } from "../diagnostics/DiagnosticsPanel";
import { Monitor, Volume2, Files, ClipboardPaste, Maximize, X } from "lucide-react";
import { invoke, listen } from "@/lib/ipc";
import { BouncingLoader } from "../ui/bouncing-loader";

export function ActiveSession({ deviceId, streamUrl, onDisconnect }: { deviceId: string, streamUrl: string, onDisconnect: () => void }) {
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [connectionState, setConnectionState] = useState<"connected" | "reconnecting" | "failed">("connected");

    useEffect(() => {
      const unlisten = listen("connection_state_changed", (event: any) => {
        const state = event.payload?.state;
        if (state) setConnectionState(state);
      });
      
      const handleMouseMove = (e: MouseEvent) => {
        invoke("send_input_event", { 
            event: { MouseMove: { x: Math.floor(e.clientX), y: Math.floor(e.clientY) } } 
        }).catch(() => {}); // silent catch for spam
      };
      const handleKeyDown = (e: KeyboardEvent) => {
        invoke("send_input_event", { 
            event: { KeyPress: { key_code: e.keyCode, down: true } } 
        }).catch(console.error);
      };
      const handleKeyUp = (e: KeyboardEvent) => {
        invoke("send_input_event", { 
            event: { KeyPress: { key_code: e.keyCode, down: false } } 
        }).catch(console.error);
      };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("keydown", handleKeyDown);
      window.addEventListener("keyup", handleKeyUp);

      return () => {
        unlisten.then(f => f());
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("keydown", handleKeyDown);
        window.removeEventListener("keyup", handleKeyUp);
      };
    }, []);

  const [monitorIndex, setMonitorIndex] = useState(0);

  const handleSwitchMonitor = () => {
    const nextIndex = monitorIndex === 0 ? 1 : 0;
    setMonitorIndex(nextIndex);
    invoke("switch_monitor", { index: nextIndex }).catch(console.error);
  };

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center overflow-hidden">
      {/* Video Canvas (Phase 23 Loopback MJPEG) */}
      <div className={`w-full h-full flex items-center justify-center transition-opacity duration-500 ${connectionState !== "connected" ? "opacity-20" : "opacity-100"}`}>
        <img src={streamUrl} className="w-full h-full object-contain" alt={`Video Stream from ${deviceId}`} />
      </div>

      {connectionState === "reconnecting" && (
        <div className="absolute inset-0 z-40 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="flex flex-col items-center space-y-6">
            <BouncingLoader />
            <p className="text-xl font-bold tracking-widest text-primary animate-pulse">RECONNECTING...</p>
            <p className="text-sm text-zinc-400">Network connection lost. Attempting to re-establish P2P tunnel.</p>
          </div>
        </div>
      )}

      {showDiagnostics && <DiagnosticsPanel />}

      {/* Floating Toolbar */}
      <div className="absolute bottom-6 left-1/2 -translate-x-1/2 flex items-center space-x-2 bg-zinc-900/90 border border-zinc-700 rounded-full px-4 py-2 backdrop-blur shadow-2xl">
        <Button variant="ghost" size="icon" className="text-white hover:bg-zinc-800" onClick={handleSwitchMonitor} title="Switch Monitor">
          <Monitor className="h-5 w-5" />
        </Button>
        <Button variant="ghost" size="icon" className="text-white hover:bg-zinc-800" title="Audio">
          <Volume2 className="h-5 w-5" />
        </Button>
        <Button variant="ghost" size="icon" className="text-white hover:bg-zinc-800" title="File Transfer">
          <Files className="h-5 w-5" />
        </Button>
        <Button variant="ghost" size="icon" className="text-white hover:bg-zinc-800" title="Clipboard">
          <ClipboardPaste className="h-5 w-5" />
        </Button>
        <div className="w-px h-6 bg-zinc-700 mx-2" />
        <Button variant="ghost" size="sm" className="text-white hover:bg-zinc-800 font-mono text-xs" onClick={() => setShowDiagnostics(!showDiagnostics)}>
          STATS
        </Button>
        <Button variant="ghost" size="icon" className="text-white hover:bg-zinc-800" title="Fullscreen">
          <Maximize className="h-5 w-5" />
        </Button>
        <div className="w-px h-6 bg-zinc-700 mx-2" />
        <Button variant="destructive" size="icon" className="rounded-full" onClick={() => { invoke("terminate_host_session").catch(console.error); onDisconnect(); }} title="Disconnect">
          <X className="h-5 w-5" />
        </Button>
      </div>
    </div>
  );
}
