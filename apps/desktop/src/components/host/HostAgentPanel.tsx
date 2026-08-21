import { useState, useEffect } from "react";
import { invoke } from "@/lib/ipc";
import { Button } from "../ui/button";
import { ShieldAlert, Activity } from "lucide-react";
import { GlowCard } from "../ui/glow-card";

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

  useEffect(() => {
    invoke<string>("get_local_device_id").then(setDeviceId).catch(console.error);

    const interval = setInterval(() => {
      invoke<HostStatus>("get_host_session_status")
        .then(setStatus)
        .catch(console.error);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleApprove = (allow: boolean) => {
    invoke("approve_connection", { allowed: allow }).catch(console.error);
  };

  const handleDisconnect = () => {
    invoke("terminate_host_session").catch(console.error);
  };

  return (
    <div className="p-8 space-y-6 max-w-5xl mx-auto">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Host Agent</h1>
        <p className="text-muted-foreground mt-2">Manage incoming connections to this device.</p>
      </div>

      <div className="flex justify-center w-full mb-8" onClick={() => navigator.clipboard.writeText(deviceId)} title="Click to copy full Device ID" style={{ cursor: "pointer" }}>
        <GlowCard
          value={deviceId.length > 16 ? deviceId.substring(0, 16) : deviceId}
          label="Your Device ID (Click to copy)"
          accentColor="#3b82f6"
          className="w-full max-w-md h-[250px]"
        />
      </div>

      {status.state === "awaiting_approval" && (
        <GlowCard accentColor="#eab308" className="w-full h-auto min-h-[200px]" showRay={false}>
          <div className="flex items-center space-x-2 text-yellow-500 mb-4">
            <ShieldAlert className="h-6 w-6" />
            <span className="text-xl font-bold">Incoming Connection Request</span>
          </div>
          <p className="mb-6 text-lg"><strong>{status.client_name}</strong> is attempting to connect and control this device.</p>
          <div className="flex space-x-4 mt-auto">
            <Button onClick={() => handleApprove(true)} className="bg-yellow-600 hover:bg-yellow-700 text-white flex-1">Approve Session</Button>
            <Button variant="outline" onClick={() => handleApprove(false)} className="flex-1 border-white/20 hover:bg-white/10 text-white">Reject</Button>
          </div>
        </GlowCard>
      )}

      {status.state === "active" && (
        <GlowCard accentColor="#ef4444" className="w-full h-auto min-h-[250px]" showRay={true}>
          <div className="flex items-center space-x-2 text-red-500 mb-6">
            <Activity className="h-6 w-6 animate-pulse" />
            <span className="text-xl font-bold">Remote Session Active</span>
          </div>
          <div className="grid grid-cols-2 gap-6 mb-8 flex-1">
            <div>
              <p className="text-sm text-white/50 uppercase tracking-widest mb-1">Connected Client</p>
              <p className="font-semibold text-lg">{status.client_name}</p>
            </div>
            <div>
              <p className="text-sm text-white/50 uppercase tracking-widest mb-1">Duration</p>
              <p className="font-semibold text-lg">{status.duration}</p>
            </div>
            <div>
              <p className="text-sm text-white/50 uppercase tracking-widest mb-1">Network</p>
              <p className="font-semibold text-lg">{status.network}</p>
            </div>
            <div>
              <p className="text-sm text-white/50 uppercase tracking-widest mb-1">Performance</p>
              <p className="font-semibold text-lg">{status.fps} FPS • {status.bitrate} • {status.latency}</p>
            </div>
          </div>
          <Button variant="destructive" className="w-full mt-auto text-lg py-6 bg-red-600 hover:bg-red-700" onClick={handleDisconnect}>
            Force Disconnect
          </Button>
        </GlowCard>
      )}
    </div>
  );
}
