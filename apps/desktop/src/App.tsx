import { useState } from "react";
import { invoke } from "@/lib/ipc";
import { Dashboard } from "./components/devices/Dashboard";
import { ActiveSession } from "./components/session/ActiveSession";
import { HostAgentPanel } from "./components/host/HostAgentPanel";
import { BouncingLoader } from "./components/ui/bouncing-loader";
import { MorphingText } from "./components/ui/morphing-text";
import { cn } from "@/lib/utils";
import { motion } from "framer-motion";
import { Monitor, MonitorPlay, Server } from "lucide-react";

/* Hallmark · component: sidebar · genre: modern-minimal
 * states: default · hover · focus-visible · active
 * contrast: pass (46–50)
 */
function App() {
  const [activeTab, setActiveTab] = useState<"client" | "host">("client");
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [streamUrl, setStreamUrl] = useState<string>("");
  const [audioUrl, setAudioUrl] = useState<string>("");
  const [connecting, setConnecting] = useState(false);

  // Hosting (screen capture + input injection) is Windows-only. On Linux/macOS
  // the machine can only be a viewer, so hide the Host Agent tab there to avoid
  // the confusing "host sends a mock gradient" state.
  const isWindows = /Windows|Win32|Win64|WOW64/i.test(navigator.userAgent);
  const tabs = (isWindows ? ["client", "host"] : ["client"]) as Array<"client" | "host">;

  const handleConnect = async (deviceId: string) => {
    setConnecting(true);
    try {
      const res = await invoke<{ video: string; audio: string }>("connect_to_device", { id: deviceId });
      setStreamUrl(res.video);
      setAudioUrl(res.audio);
      setActiveSession(deviceId);
    } catch (e) {
      console.error(e);
      alert("Failed to connect");
    } finally {
      setConnecting(false);
    }
  };

  if (activeSession) {
    return <ActiveSession deviceId={activeSession} streamUrl={streamUrl} audioUrl={audioUrl} onDisconnect={() => setActiveSession(null)} />;
  }

  return (
    <div className="flex min-h-screen bg-background">
      {/* Sidebar Navigation — compact rail on narrow windows, full on wider */}
      <aside className="w-16 md:w-64 shrink-0 border-r border-border bg-card flex flex-col p-2 md:p-4 shadow-sm z-10 transition-[width] duration-200">
        <div className="mb-6 h-[40px] flex items-center px-1 md:px-2 overflow-hidden">
          {/* Full wordmark on wider windows */}
          <div className="hidden md:block min-w-0">
            <MorphingText words={["Remote Control", "Workstation", "P2P Stream"]} className="text-lg font-bold tracking-tight text-foreground !text-left truncate" />
          </div>
          {/* Compact monogram on the narrow rail */}
          <div className="md:hidden mx-auto w-9 h-9 rounded-lg bg-primary/15 border border-primary/30 flex items-center justify-center">
            <Monitor className="h-4 w-4 text-primary" />
          </div>
        </div>
        
        <nav className="flex flex-col space-y-1.5 relative">
          {tabs.map((tab) => {
            const isActive = activeTab === tab;
            const Icon = tab === "client" ? MonitorPlay : Server;
            return (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                title={tab === "client" ? "Connect" : "Host Agent"}
                className={cn(
                  "relative group flex items-center gap-2.5 justify-center md:justify-start px-0 md:px-3.5 py-2.5 text-xs font-semibold rounded-lg transition-all duration-200 outline-none w-full",
                  isActive
                    ? "text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.98]"
                )}
              >
                {isActive && (
                  <motion.div
                    layoutId="sidebar-active-tab"
                    className="absolute inset-0 bg-primary rounded-lg -z-10"
                    initial={false}
                    transition={{
                      type: "spring",
                      stiffness: 400,
                      damping: 30,
                    }}
                  />
                )}
                {!isActive && (
                  <div className="absolute inset-0 bg-secondary/50 opacity-0 group-hover:opacity-100 rounded-lg transition-opacity duration-150 -z-10" />
                )}
                <Icon className="relative z-10 h-4 w-4 shrink-0" />
                <span className="relative z-10 hidden md:inline">
                  {tab === "client" ? "Connect" : "Host Agent"}
                </span>
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 min-w-0 overflow-auto relative">
        {connecting && (
          <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/90 backdrop-blur-md p-6">
            <div className="flex flex-col items-center space-y-6">
              <BouncingLoader />
              <p className="text-base sm:text-xl font-bold tracking-widest text-primary animate-pulse text-center">ESTABLISHING CONNECTION</p>
            </div>
          </div>
        )}
        
        {activeTab === "host" && isWindows ? (
          <HostAgentPanel />
        ) : (
          <Dashboard onConnect={handleConnect} />
        )}
      </main>
    </div>
  );
}

export default App;
