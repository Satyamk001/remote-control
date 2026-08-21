import { SessionManager } from '../src/SessionManager';
import { ConnectionManager } from '../src/ConnectionManager';
import { SignalingController } from '../src/SignalingController';
import { MessageValidator } from '../src/MessageValidator';
import { SessionExpiryManager } from '../src/SessionExpiryManager';

describe('Signaling Server Tests', () => {
    let sessionManager: SessionManager;
    let connectionManager: ConnectionManager;
    let controller: SignalingController;

    beforeEach(() => {
        sessionManager = new SessionManager();
        connectionManager = new ConnectionManager();
        controller = new SignalingController(sessionManager, connectionManager);
        
        // Mock sendMessage so we can observe behavior
        connectionManager.sendMessage = jest.fn();
    });

    test('should validate correct messages', () => {
        const raw = '{"type":"register_host"}';
        const msg = MessageValidator.validate(raw);
        expect(msg).not.toBeNull();
        expect(msg?.type).toBe('register_host');
    });

    test('should reject malformed messages', () => {
        expect(MessageValidator.validate('not json')).toBeNull();
        expect(MessageValidator.validate('{"random":"field"}')).toBeNull();
        expect(MessageValidator.validate('{"type":"join_session"}')).toBeNull(); // Missing sessionId
    });

    test('should create session on register_host', () => {
        controller.handleMessage('host-conn-1', { type: 'register_host' });
        
        expect(connectionManager.sendMessage).toHaveBeenCalledWith('host-conn-1', expect.objectContaining({
            type: 'host_registered',
            sessionId: expect.any(String)
        }));
        
        const sessions = sessionManager.getAllSessions();
        expect(sessions.size).toBe(1);
    });

    test('should allow viewer to join and route SDP', () => {
        const session = sessionManager.createSession('host-conn-1');
        
        controller.handleMessage('viewer-conn-1', { type: 'join_session', sessionId: session.id });
        expect(session.viewerId).toBe('viewer-conn-1');
        
        // Host should be notified (mock behavior assumed in controller)
        // Check SDP routing
        controller.handleMessage('host-conn-1', { type: 'sdp_offer', sessionId: session.id, sdp: 'fake-sdp' });
        expect(connectionManager.sendMessage).toHaveBeenCalledWith('viewer-conn-1', {
            type: 'sdp_offer',
            sessionId: session.id,
            sdp: 'fake-sdp'
        });
    });

    test('should expire old sessions', () => {
        const expiryManager = new SessionExpiryManager(sessionManager, connectionManager, 100); // 100ms
        const session = sessionManager.createSession('host-conn-1');
        
        // Artificially age the session
        session.createdAt = Date.now() - 200;
        
        (expiryManager as any).checkExpirations();
        expect(sessionManager.getSession(session.id)).toBeUndefined();
    });
});
