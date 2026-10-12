import { useState } from "react";
import { invoke } from "@/lib/ipc";
import { Dashboard } from "./components/devices/Dashboard";
import { ActiveSession } from "./components/session/ActiveSession";
import { HostAgentPanel } from "./components/host/HostAgentPanel";
import { BouncingLoader } from "./components/ui/bouncing-loader";
import { MorphingText } from "./components/ui/morphing-text";
import { cn } from "@/lib/utils";
import { motion } from "framer-motion";

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
      {/* Sidebar Navigation */}
      <aside className="w-64 border-r border-border bg-card flex flex-col p-4 shadow-sm z-10">
        <div className="mb-6 h-[40px] flex items-center px-2">
          <MorphingText words={["Remote Control", "Workstation", "P2P Stream"]} className="text-lg font-bold tracking-tight text-foreground !text-left" />
        </div>
        
        <nav className="flex flex-col space-y-1.5 relative">
          {(["client", "host"] as const).map((tab) => {
            const isActive = activeTab === tab;
            return (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={cn(
                  "relative group flex items-center justify-between px-3.5 py-2.5 text-xs font-semibold rounded-lg transition-all duration-200 outline-none w-full",
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
                <span className="relative z-10">
                  {tab === "client" ? "Connect" : "Host Agent"}
                </span>
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 overflow-auto relative">
        {connecting && (
          <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/90 backdrop-blur-md">
            <div className="flex flex-col items-center space-y-6">
              <BouncingLoader />
              <p className="text-xl font-bold tracking-widest text-primary animate-pulse">ESTABLISHING CONNECTION</p>
            </div>
          </div>
        )}
        
        {activeTab === "client" ? (
          <Dashboard onConnect={handleConnect} />
        ) : (
          <HostAgentPanel />
        )}
      </main>
    </div>
  );
}

export default App;
