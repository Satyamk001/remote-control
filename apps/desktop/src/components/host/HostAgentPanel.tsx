import { useState, useEffect } from "react";
import { invoke } from "@/lib/ipc";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { GlowCard } from "../ui/glow-card";
import { HyperText } from "../ui/hyper-text";
import { 
  ShieldAlert, 
  Activity, 
  Copy, 
  Check, 
  Radio, 
  ShieldCheck, 
  CheckCircle2, 
  Cpu
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

interface HostStatus {
  state: "idle" | "awaiting_approval" | "active";
  client_name?: string;
  duration?: string;
  network?: string;
  fps?: number;
  bitrate?: string;
  latency?: string;
}

export function HostAgentPanel() {
  const [deviceId, setDeviceId] = useState("Loading...");
  const [status, setStatus] = useState<HostStatus>({ state: "idle" });
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    invoke<string>("get_local_device_id").then(setDeviceId).catch(console.error);

    const interval = setInterval(() => {
      invoke<HostStatus>("get_host_session_status")
        .then(setStatus)
        .catch(console.error);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleCopy = () => {
    navigator.clipboard.writeText(deviceId);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const handleApprove = (allow: boolean) => {
    invoke("approve_connection", { allowed: allow }).catch(console.error);
  };

  const handleDisconnect = () => {
    invoke("terminate_host_session").catch(console.error);
  };

  return (
    <div className="p-8 space-y-6 max-w-5xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border/40 pb-5">
        <div className="flex items-center gap-3">
          <HyperText text="HOST AGENT" className="text-2xl font-bold tracking-tight text-foreground" />
          <Badge 
            variant="outline" 
            className={`gap-1.5 px-2.5 py-0.5 text-xs font-semibold ${
              status.state === "active" 
                ? "bg-red-500/15 text-red-400 border-red-500/30" 
                : status.state === "awaiting_approval"
                ? "bg-amber-500/15 text-amber-400 border-amber-500/30"
                : "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
            }`}
          >
            <span className={`w-2 h-2 rounded-full ${
              status.state === "active" 
                ? "bg-red-500 animate-pulse" 
                : status.state === "awaiting_approval"
                ? "bg-amber-500 animate-ping"
                : "bg-emerald-500 animate-pulse"
            }`} />
            {status.state === "active" ? "Active" : status.state === "awaiting_approval" ? "Request Pending" : "Listening"}
          </Badge>
        </div>

        <div className="flex items-center gap-1.5 text-xs text-muted-foreground bg-secondary/30 border border-border/30 px-2.5 py-1 rounded-full">
          <Radio className="h-3 w-3 text-emerald-400 animate-pulse" />
          <span className="font-mono text-[11px]">Signaling Live</span>
        </div>
      </div>

      {/* Device ID Card */}
      <div className="rounded-2xl border border-border/40 bg-card/30 p-5 backdrop-blur-sm">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="space-y-1">
            <div className="flex items-center gap-1.5 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
              <ShieldCheck className="h-3.5 w-3.5 text-primary" />
              <span>Your Device ID</span>
            </div>
            <div className="font-mono text-base sm:text-lg font-bold text-foreground select-all bg-background/60 px-3 py-1 rounded-md border border-border/40 inline-block">
              {deviceId}
            </div>
          </div>

          <Button
            onClick={handleCopy}
            variant={copied ? "default" : "secondary"}
            className="gap-2 text-xs font-medium h-9 px-3.5 active:scale-95 transition-all self-start sm:self-center"
          >
            {copied ? (
              <>
                <Check className="h-3.5 w-3.5 text-emerald-400" />
                <span>Copied</span>
              </>
            ) : (
              <>
                <Copy className="h-3.5 w-3.5" />
                <span>Copy ID</span>
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Dynamic State Display */}
      <AnimatePresence mode="wait">
        {/* Incoming Connection State */}
        {status.state === "awaiting_approval" && (
          <motion.div
            key="awaiting_approval"
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.97 }}
            transition={{ duration: 0.2 }}
          >
            <GlowCard accentColor="#eab308" className="w-full h-auto min-h-[220px]" showRay={false}>
              <div className="flex items-center justify-between gap-3 mb-3">
                <div className="flex items-center gap-2 text-amber-400">
                  <ShieldAlert className="h-5 w-5" />
                  <h3 className="text-lg font-bold text-foreground">Connection Request</h3>
                </div>
                <Badge variant="outline" className="bg-amber-500/10 text-amber-400 border-amber-500/30 text-[10px] px-2 py-0">
                  Pending
                </Badge>
              </div>

              <div className="bg-secondary/30 border border-border/30 rounded-lg p-3 mb-4 text-xs">
                <p className="text-foreground">
                  <span className="font-bold text-amber-400">{status.client_name || "Remote Client"}</span> is requesting session control.
                </p>

                <div className="mt-2.5 pt-2 border-t border-border/30 grid grid-cols-3 gap-2 text-[11px]">
                  <div className="flex items-center gap-1 text-foreground/80">
                    <CheckCircle2 className="h-3 w-3 text-emerald-400" />
                    <span>Display</span>
                  </div>
                  <div className="flex items-center gap-1 text-foreground/80">
                    <CheckCircle2 className="h-3 w-3 text-emerald-400" />
                    <span>Input</span>
                  </div>
                  <div className="flex items-center gap-1 text-foreground/80">
                    <CheckCircle2 className="h-3 w-3 text-emerald-400" />
                    <span>Audio</span>
                  </div>
                </div>
              </div>

              <div className="flex gap-2.5 mt-auto">
                <Button 
                  onClick={() => handleApprove(true)} 
                  className="bg-amber-500 hover:bg-amber-600 text-black font-semibold flex-1 h-9 text-xs active:scale-95"
                >
                  Approve
                </Button>
                <Button 
                  variant="outline" 
                  onClick={() => handleApprove(false)} 
                  className="flex-1 border-border/60 hover:bg-destructive/10 hover:text-destructive h-9 text-xs active:scale-95"
                >
                  Reject
                </Button>
              </div>
            </GlowCard>
          </motion.div>
        )}

        {/* Active Session Telemetry */}
        {status.state === "active" && (
          <motion.div
            key="active_session"
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.97 }}
            transition={{ duration: 0.2 }}
          >
            <GlowCard accentColor="#ef4444" className="w-full h-auto min-h-[240px]" showRay={true}>
              <div className="flex items-center justify-between gap-3 mb-4">
                <div className="flex items-center gap-2 text-red-500">
                  <Activity className="h-5 w-5 animate-pulse" />
                  <h3 className="text-lg font-bold text-foreground">Session Active</h3>
                </div>
                <Badge variant="destructive" className="animate-pulse px-2 py-0 text-[10px]">
                  Streaming
                </Badge>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
                <div className="bg-secondary/30 border border-border/30 rounded-lg p-2.5">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider block">Peer</span>
                  <span className="font-semibold text-xs text-foreground truncate block">{status.client_name || "Connected"}</span>
                </div>

                <div className="bg-secondary/30 border border-border/30 rounded-lg p-2.5">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider block">Time</span>
                  <span className="font-semibold text-xs font-mono text-foreground block">{status.duration || "00:00"}</span>
                </div>

                <div className="bg-secondary/30 border border-border/30 rounded-lg p-2.5">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider block">FPS</span>
                  <span className="font-semibold text-xs font-mono text-foreground block">{status.fps || 60} FPS</span>
                </div>

                <div className="bg-secondary/30 border border-border/30 rounded-lg p-2.5">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider block">Latency</span>
                  <span className="font-semibold text-xs font-mono text-emerald-400 block">{status.latency || "< 35ms"}</span>
                </div>
              </div>

              <Button 
                variant="destructive" 
                className="w-full text-xs font-semibold py-2.5 h-auto bg-red-600 hover:bg-red-700 active:scale-98 transition-all" 
                onClick={handleDisconnect}
              >
                Disconnect Session
              </Button>
            </GlowCard>
          </motion.div>
        )}

        {/* Idle Readiness State */}
        {status.state === "idle" && (
          <motion.div
            key="idle_state"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="rounded-2xl border border-border/40 bg-card/20 p-5 space-y-4"
          >
            <div className="flex items-center gap-2 text-foreground font-semibold text-xs uppercase tracking-wider">
              <Cpu className="h-4 w-4 text-primary" />
              <span>Engine Status</span>
            </div>

            <div className="grid sm:grid-cols-3 gap-3">
              <div className="p-3 rounded-lg bg-secondary/20 border border-border/30">
                <span className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  DXGI Capture
                </span>
                <span className="text-[11px] text-muted-foreground mt-0.5 block">Zero-Copy VRAM</span>
              </div>

              <div className="p-3 rounded-lg bg-secondary/20 border border-border/30">
                <span className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  Input Dispatch
                </span>
                <span className="text-[11px] text-muted-foreground mt-0.5 block">Win32 Hook API</span>
              </div>

              <div className="p-3 rounded-lg bg-secondary/20 border border-border/30">
                <span className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  Audio Loopback
                </span>
                <span className="text-[11px] text-muted-foreground mt-0.5 block">WASAPI Stream</span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
