import { useState, useEffect } from "react";
import { invoke } from "@/lib/ipc";
import { Card, CardHeader, CardTitle, CardContent } from "../ui/card";

interface Metrics {
  fps: number;
  bitrate: string;
  rtt: string;
  packet_loss: string;
  jitter: string;
  encode_latency: string;
  decode_latency: string;
  resolution: string;
  codec: string;
  network: string;
}

export function DiagnosticsPanel() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);

  useEffect(() => {
    const interval = setInterval(() => {
      invoke<Metrics>("get_diagnostics")
        .then(setMetrics)
        .catch(console.error);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  if (!metrics) return null;

  return (
    <Card className="absolute top-4 left-4 w-72 bg-black/80 text-white border-zinc-800 backdrop-blur">
      <CardHeader className="py-3">
        <CardTitle className="text-sm font-mono flex items-center justify-between">
          <span>Diagnostics</span>
          <span className="text-green-400">{metrics.fps} FPS</span>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-1 text-xs font-mono pb-4">
        <div className="flex justify-between"><span>Resolution</span><span>{metrics.resolution}</span></div>
        <div className="flex justify-between"><span>Codec</span><span>{metrics.codec}</span></div>
        <div className="flex justify-between"><span>Bitrate</span><span>{metrics.bitrate}</span></div>
        <div className="flex justify-between text-muted-foreground mt-2 pt-2 border-t border-zinc-800">
          <span>Network</span><span>{metrics.network}</span>
        </div>
        <div className="flex justify-between"><span>RTT</span><span className="text-yellow-400">{metrics.rtt}</span></div>
        <div className="flex justify-between"><span>Packet Loss</span><span>{metrics.packet_loss}</span></div>
        <div className="flex justify-between"><span>Jitter</span><span>{metrics.jitter}</span></div>
        <div className="flex justify-between text-muted-foreground mt-2 pt-2 border-t border-zinc-800">
          <span>Encode Latency</span><span>{metrics.encode_latency}</span>
        </div>
        <div className="flex justify-between"><span>Decode Latency</span><span>{metrics.decode_latency}</span></div>
      </CardContent>
    </Card>
  );
}
