import { useEffect, useState, useCallback } from "react";
import { invoke } from "@/lib/ipc";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { GlowCard } from "../ui/glow-card";
import { ShinyCTA } from "../ui/shiny-cta";
import { HyperText } from "../ui/hyper-text";
import { 
  MonitorPlay, 
  RotateCw, 
  Radio, 
  Copy, 
  Check, 
  Laptop, 
  WifiOff 
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

interface Device {
  id: string;
  name: string;
  status: "online" | "offline";
  last_seen: string;
}

export function Dashboard({ onConnect }: { onConnect: (id: string) => void }) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [localDeviceId, setLocalDeviceId] = useState<string | null>(null);

  // Fetch local device ID once on mount
  useEffect(() => {
    invoke<string>("get_local_device_id")
      .then(setLocalDeviceId)
      .catch(console.error);
  }, []);

  const fetchDevices = useCallback(async (manual = false) => {
    if (manual) setIsRefreshing(true);
    try {
      const result = await invoke<Device[]>("get_devices");
      setDevices(result || []);
    } catch (err) {
      console.error("Failed to fetch devices:", err);
    } finally {
      setLoading(false);
      if (manual) {
        setTimeout(() => setIsRefreshing(false), 400);
      }
    }
  }, []);

  useEffect(() => {
    fetchDevices();
    const interval = setInterval(() => {
      fetchDevices();
    }, 3000);
    return () => clearInterval(interval);
  }, [fetchDevices]);

  const handleCopyId = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    navigator.clipboard.writeText(id);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const filteredDevices = devices.filter(d => d.id !== localDeviceId);
  const onlineCount = filteredDevices.filter(d => d.status === "online").length;

  return (
    <div className="p-8 space-y-6 max-w-6xl mx-auto">
      {/* Header Section */}
      <div className="flex items-center justify-between border-b border-border/40 pb-5">
        <div className="flex items-center gap-3">
          <HyperText text="DEVICES" className="text-2xl font-bold tracking-tight text-foreground" />
          <Badge variant="secondary" className="px-2 py-0.5 text-xs font-semibold">
            {onlineCount} Online
          </Badge>
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={() => fetchDevices(true)}
          disabled={isRefreshing}
          className="gap-2 text-xs font-medium h-8 px-3 border-border/60 hover:bg-secondary/60 active:scale-95 transition-all"
        >
          <RotateCw className={`h-3 w-3 text-muted-foreground ${isRefreshing ? "animate-spin text-primary" : ""}`} />
          <span>{isRefreshing ? "Refreshing" : "Refresh"}</span>
        </Button>
      </div>

      {/* Main Content */}
      {loading ? (
        <div className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
          {[1, 2, 3].map((i) => (
            <div
              key={i}
              className="h-[240px] rounded-2xl border border-border/40 bg-card/40 p-5 animate-pulse flex flex-col justify-between"
            >
              <div className="flex justify-between items-center">
                <div className="h-5 w-28 bg-muted/60 rounded" />
                <div className="h-4 w-12 bg-muted/40 rounded-full" />
              </div>
              <div className="h-9 w-full bg-muted/40 rounded-lg" />
            </div>
          ))}
        </div>
      ) : filteredDevices.length === 0 ? (
        /* Empty State */
        <motion.div
          initial={{ opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.25 }}
          className="flex flex-col items-center justify-center p-12 text-center rounded-2xl border border-border/40 bg-card/20 backdrop-blur-sm min-h-[320px] relative overflow-hidden"
        >
          <div className="relative mb-4">
            <div className="w-14 h-14 rounded-full bg-primary/10 flex items-center justify-center border border-primary/20 text-primary">
              <Radio className="w-7 h-7 animate-pulse" />
            </div>
            <div className="absolute inset-0 rounded-full bg-primary/5 animate-ping -z-10" />
          </div>

          <h3 className="text-lg font-semibold tracking-tight text-foreground mb-1">
            Scanning for Hosts
          </h3>
          <p className="text-xs text-muted-foreground max-w-xs mb-4">
            Launch Remote Control on your host device to begin.
          </p>

          <div className="flex items-center gap-1.5 text-[11px] font-mono text-muted-foreground/80 bg-secondary/30 px-3 py-1 rounded-full border border-border/30">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
            <span>Signaling Active</span>
          </div>
        </motion.div>
      ) : (
        /* Animated Device Grid */
        <motion.div 
          layout
          className="grid gap-5 md:grid-cols-2 lg:grid-cols-3"
        >
          <AnimatePresence mode="popLayout">
            {filteredDevices.map((device) => {
              const isOnline = device.status === "online";
              const shortId = device.id.length > 14 
                ? `${device.id.substring(0, 8)}...${device.id.substring(device.id.length - 4)}` 
                : device.id;

              return (
                <motion.div
                  key={device.id}
                  layout
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  whileHover={{ y: -3 }}
                  transition={{ duration: 0.2 }}
                >
                  <GlowCard 
                    className="w-full h-[250px] transition-all" 
                    accentColor={isOnline ? "#10b981" : "#64748b"}
                    showRay={isOnline}
                    showDot={isOnline}
                    showGrid={true}
                  >
                    <div className="flex flex-col h-full justify-between">
                      {/* Card Header */}
                      <div>
                        <div className="flex items-center justify-between gap-2 mb-2">
                          <div className="flex items-center gap-2 truncate">
                            <Laptop className="h-4 w-4 text-muted-foreground shrink-0" />
                            <span className="text-base font-bold tracking-tight text-foreground truncate">
                              {device.name}
                            </span>
                          </div>
                          <Badge 
                            variant={isOnline ? "default" : "secondary"}
                            className={`gap-1 px-2 py-0 text-[10px] font-semibold shrink-0 ${
                              isOnline 
                                ? "bg-emerald-500/15 text-emerald-400 border border-emerald-500/30" 
                                : "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {isOnline ? (
                              <>
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                                Online
                              </>
                            ) : (
                              <>
                                <WifiOff className="w-2.5 h-2.5" />
                                Offline
                              </>
                            )}
                          </Badge>
                        </div>

                        {/* Monospace Device ID Badge */}
                        <div 
                          onClick={(e) => handleCopyId(e, device.id)}
                          className="mt-2 flex items-center justify-between bg-secondary/30 hover:bg-secondary/50 border border-border/30 rounded-md px-2 py-1 cursor-pointer transition-colors"
                          title="Click to copy Device ID"
                        >
                          <span className="text-[11px] font-mono text-muted-foreground truncate">
                            {shortId}
                          </span>
                          <span className="text-muted-foreground p-0.5">
                            {copiedId === device.id ? (
                              <Check className="h-3 w-3 text-emerald-400" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </span>
                        </div>
                      </div>

                      {/* Status / Activity */}
                      {device.last_seen && (
                        <div className="text-[11px] text-muted-foreground/70 font-mono py-1">
                          {device.last_seen}
                        </div>
                      )}

                      {/* Connect Button */}
                      <div className="pt-2">
                        {isOnline ? (
                          <ShinyCTA 
                            className="w-full text-xs font-semibold py-2 px-3 shadow-[0_0_15px_rgba(16,185,129,0.25)]"
                            style={{ "--shiny-cta-highlight": "#10b981", "--shiny-cta-highlight-subtle": "#34d399" } as React.CSSProperties}
                            onClick={() => onConnect(device.id)}
                          >
                            <div className="flex items-center justify-center gap-1.5">
                              <MonitorPlay className="h-3.5 w-3.5" />
                              <span>Connect</span>
                            </div>
                          </ShinyCTA>
                        ) : (
                          <Button 
                            disabled 
                            variant="secondary" 
                            className="w-full text-xs font-medium py-2 h-auto opacity-40 cursor-not-allowed"
                          >
                            Offline
                          </Button>
                        )}
                      </div>
                    </div>
                  </GlowCard>
                </motion.div>
              );
            })}
          </AnimatePresence>
        </motion.div>
      )}
    </div>
  );
}
