import { useEffect, useState } from "react";
import { invoke } from "@/lib/ipc";
import { Badge } from "../ui/badge";
import { MonitorPlay } from "lucide-react";
import { GlowCard } from "../ui/glow-card";
import { ShinyCTA } from "../ui/shiny-cta";

interface Device {
  id: string;
  name: string;
  status: "online" | "offline";
  last_seen: string;
}

export function Dashboard({ onConnect }: { onConnect: (id: string) => void }) {
  const [devices, setDevices] = useState<Device[]>([]);

  useEffect(() => {
    invoke<Device[]>("get_devices")
      .then(setDevices)
      .catch(console.error);
  }, []);

  return (
    <div className="p-8 space-y-6 max-w-5xl mx-auto">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Devices</h1>
        <p className="text-muted-foreground mt-2">Manage and connect to your remote devices.</p>
      </div>
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        {devices.map((device) => (
          <GlowCard 
            key={device.id} 
            className="w-full h-[280px]" 
            accentColor={device.status === "online" ? "#10b981" : "#6b7280"}
          >
            <div className="flex flex-row items-center justify-between mb-4">
              <h3 className="text-xl font-semibold leading-none tracking-tight">{device.name}</h3>
              <MonitorPlay className="h-5 w-5 text-muted-foreground" />
            </div>
            
            <div className="flex items-center space-x-2 mt-auto mb-6">
              <Badge variant={device.status === "online" ? "default" : "secondary"}>
                {device.status}
              </Badge>
              <span className="text-xs text-muted-foreground">{device.last_seen}</span>
            </div>
            
            {device.status === "online" && (
              <div className="mt-auto flex justify-center pb-2">
                <ShinyCTA 
                  className="w-full text-sm py-2 px-4 shadow-[0_0_20px_rgba(16,185,129,0.3)]"
                  style={{ "--shiny-cta-highlight": "#10b981", "--shiny-cta-highlight-subtle": "#34d399" } as React.CSSProperties}
                  onClick={() => onConnect(device.id)}
                >
                  Connect Session
                </ShinyCTA>
              </div>
            )}
          </GlowCard>
        ))}
      </div>
    </div>
  );
}
