import { SessionManager } from './SessionManager';
import { ConnectionManager } from './ConnectionManager';

export class SessionExpiryManager {
    private intervalId: NodeJS.Timeout | null = null;
    
    constructor(
        private sessionManager: SessionManager,
        private connectionManager: ConnectionManager,
        private expiryThresholdMs: number = 1000 * 60 * 60 // 1 hour
    ) {}

    public start(): void {
        this.intervalId = setInterval(() => this.checkExpirations(), 60000);
    }

    public stop(): void {
        if (this.intervalId) {
            clearInterval(this.intervalId);
        }
    }

    private checkExpirations(): void {
        const now = Date.now();
        const sessions = this.sessionManager.getAllSessions();
        
        for (const [id, session] of sessions.entries()) {
            if (now - session.createdAt > this.expiryThresholdMs) {
                this.sessionManager.removeSession(id);
                // In a real app we might disconnect the websockets too, 
                // but for now just drop the session.
            }
        }
    }
}
