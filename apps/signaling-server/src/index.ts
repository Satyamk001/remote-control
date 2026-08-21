import { WebSocketServer } from 'ws';
import * as crypto from 'crypto';
import { SessionManager } from './SessionManager';
import { ConnectionManager } from './ConnectionManager';
import { SignalingController } from './SignalingController';
import { MessageValidator } from './MessageValidator';
import { SessionExpiryManager } from './SessionExpiryManager';
const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8081;
const wss = new WebSocketServer({ port });

const sessionManager = new SessionManager();
const connectionManager = new ConnectionManager();
const controller = new SignalingController(sessionManager, connectionManager);
const expiryManager = new SessionExpiryManager(sessionManager, connectionManager);

expiryManager.start();

// Basic rate limiting: IP -> Token count
const rateLimits = new Map<string, { tokens: number; lastRefill: number }>();
const MAX_TOKENS = 100;
const REFILL_RATE = 1000; // ms per token

function checkRateLimit(ip: string): boolean {
    const now = Date.now();
    let record = rateLimits.get(ip);
    
    if (!record) {
        record = { tokens: MAX_TOKENS - 1, lastRefill: now };
        rateLimits.set(ip, record);
        return true;
    }
    
    const timePassed = now - record.lastRefill;
    const refillTokens = Math.floor(timePassed / REFILL_RATE);
    
    if (refillTokens > 0) {
        record.tokens = Math.min(MAX_TOKENS, record.tokens + refillTokens);
        record.lastRefill = now;
    }
    
    if (record.tokens > 0) {
        record.tokens -= 1;
        return true;
    }
    
    return false;
}

wss.on('connection', (ws, req) => {
    const ip = req.socket.remoteAddress || 'unknown';
    
    if (!checkRateLimit(ip)) {
        console.warn(`[AUDIT] Rate limit exceeded for IP: ${ip}. Disconnecting.`);
        ws.close(1008, 'Rate limit exceeded');
        return;
    }

    console.log(`[AUDIT] New WebSocket connection established from ${ip}`);
    const connectionId = crypto.randomUUID();
    connectionManager.addConnection(connectionId, ws);

    ws.on('message', (data) => {
        const messageStr = data.toString();
        const msg = MessageValidator.validate(messageStr);
        if (msg) {
            controller.handleMessage(connectionId, msg);
        } else {
            ws.send(JSON.stringify({ type: 'error', message: 'Invalid message format' }));
        }
    });

    ws.on('close', () => {
        controller.handleDisconnect(connectionId);
    });
});

console.log(`Signaling server listening on ws://localhost:${port}`);