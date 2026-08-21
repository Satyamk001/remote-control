import { SessionManager } from './SessionManager';
import { ConnectionManager } from './ConnectionManager';
import { SignalingMessage } from './types';

export class SignalingController {
    constructor(
        private sessionManager: SessionManager,
        private connectionManager: ConnectionManager
    ) {}

    public handleMessage(connectionId: string, message: SignalingMessage): void {
        if (!message || typeof message.type !== 'string') {
            console.warn('[AUDIT] Invalid message format received');
            return;
        }

        console.log(`[AUDIT] Processing signaling message: ${message.type}`);
        switch (message.type) {
            case 'register_host':
                const session = this.sessionManager.createSession(connectionId, message.deviceId);
                this.connectionManager.sendMessage(connectionId, {
                    type: 'host_registered',
                    sessionId: session.id
                });
                break;
                
            case 'get_hosts':
                const sessions = Array.from(this.sessionManager.getAllSessions().values());
                const hosts = sessions.map(s => ({
                    sessionId: s.id,
                    deviceId: s.deviceId
                }));
                this.connectionManager.sendMessage(connectionId, {
                    type: 'hosts_list',
                    hosts: hosts
                });
                break;
                
            case 'join_session':
                const success = this.sessionManager.joinSession(message.sessionId, connectionId);
                if (!success) {
                    this.connectionManager.sendMessage(connectionId, {
                        type: 'error',
                        message: 'Session not found or already full'
                    });
                } else {
                    const s = this.sessionManager.getSession(message.sessionId);
                    if (s) {
                        this.connectionManager.sendMessage(s.hostId, {
                            type: 'viewer_joined',
                            sessionId: s.id
                        });
                    }
                }
                break;
                
            case 'sdp_offer':
            case 'sdp_answer':
            case 'ice_candidate':
                const s = this.sessionManager.getSession(message.sessionId);
                if (s) {
                    // Route to the *other* party
                    const targetId = connectionId === s.hostId ? s.viewerId : s.hostId;
                    if (targetId) {
                        this.connectionManager.sendMessage(targetId, message);
                    }
                }
                break;
        }
    }

    public handleDisconnect(connectionId: string): void {
        this.sessionManager.removeSessionByConnectionId(connectionId);
        // We could notify the remaining party here if needed.
        this.connectionManager.removeConnection(connectionId);
    }
}
