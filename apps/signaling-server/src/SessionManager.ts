import * as crypto from 'crypto';

export interface Session {
    id: string;
    hostId: string;
    deviceId: string;
    viewerId: string | null;
    createdAt: number;
}

export class SessionManager {
    private sessions = new Map<string, Session>();

    public createSession(hostConnectionId: string, deviceId: string): Session {
        const sessionId = crypto.randomUUID();
        const session: Session = {
            id: sessionId,
            hostId: hostConnectionId,
            deviceId: deviceId,
            viewerId: null,
            createdAt: Date.now()
        };
        this.sessions.set(sessionId, session);
        return session;
    }

    public getSession(sessionId: string): Session | undefined {
        return this.sessions.get(sessionId);
    }

    public joinSession(sessionId: string, viewerConnectionId: string): boolean {
        const session = this.sessions.get(sessionId);
        if (session && !session.viewerId) {
            session.viewerId = viewerConnectionId;
            return true;
        }
        return false;
    }

    public removeSession(sessionId: string): void {
        this.sessions.delete(sessionId);
    }

    public removeSessionByConnectionId(connectionId: string): string[] {
        const removedSessions: string[] = [];
        for (const [sessionId, session] of this.sessions.entries()) {
            if (session.hostId === connectionId || session.viewerId === connectionId) {
                this.sessions.delete(sessionId);
                removedSessions.push(sessionId);
            }
        }
        return removedSessions;
    }

    public getAllSessions(): Map<string, Session> {
        return this.sessions;
    }
}
