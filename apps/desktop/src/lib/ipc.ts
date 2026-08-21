import { invoke as tauriInvoke } from '@tauri-apps/api/tauri';
import { listen as tauriListen } from '@tauri-apps/api/event';
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';

// @ts-ignore
const isTauri = () => typeof window !== 'undefined' && window.__TAURI_IPC__ !== undefined;

export const invoke = async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauri()) {
    return tauriInvoke<T>(cmd, args);
  }
  
  console.log(`[Mock IPC] invoke: ${cmd}`, args);
  
  if (cmd === 'get_devices') {
    return [
      { id: "mock_1", name: "Mock Workstation", status: "online", last_seen: "Just now" },
      { id: "mock_2", name: "Mock Laptop", status: "offline", last_seen: "2 days ago" }
    ] as unknown as T;
  }
  
  if (cmd === 'get_local_device_id') return "MOCK-HOST-9942" as unknown as T;
  
  if (cmd === 'get_diagnostics') return { 
    fps: 60, bitrate: "5 Mbps", rtt: "15ms", packet_loss: "0%", 
    jitter: "2ms", encode_latency: "4ms", decode_latency: "5ms",
    resolution: "1920x1080", codec: "H.264 (Mock)", network: "P2P (Mock)"
  } as unknown as T;
  
  if (cmd === 'get_host_session_status') return {
    state: "active",
    client_name: "Mock Viewer",
    duration: "00:05:32",
    network: "P2P (Mock)",
    fps: 60,
    bitrate: "4.5 Mbps",
    latency: "12ms"
  } as unknown as T;

  if (cmd === 'connect_to_device') {
    // Simulate auto-reconnection flow for ActiveSession
    setTimeout(() => {
        const ev = new CustomEvent("mock_tauri_event", { detail: { event: "connection_state_changed", payload: { state: "reconnecting" } } });
        window.dispatchEvent(ev);
    }, 10000);
    setTimeout(() => {
        const ev = new CustomEvent("mock_tauri_event", { detail: { event: "connection_state_changed", payload: { state: "connected" } } });
        window.dispatchEvent(ev);
    }, 14000);
    return "Connected" as unknown as T;
  }

  return undefined as unknown as T;
};

export const listen = async <T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> => {
  if (isTauri()) {
    return tauriListen<T>(event, handler);
  }
  
  console.log(`[Mock IPC] listen: ${event}`);
  
  const listener = (e: Event) => {
    const ce = e as CustomEvent;
    if (ce.detail && ce.detail.event === event) {
        handler({ event, id: 0, payload: ce.detail.payload as T, windowLabel: "main" });
    }
  };
  
  window.addEventListener("mock_tauri_event", listener);
  
  return Promise.resolve(() => {
      window.removeEventListener("mock_tauri_event", listener);
  });
};
