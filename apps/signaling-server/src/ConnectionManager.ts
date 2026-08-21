import { WebSocket } from 'ws';

export class ConnectionManager {
    private connections = new Map<string, WebSocket>();

    public addConnection(id: string, ws: WebSocket): void {
        this.connections.set(id, ws);
    }

    public removeConnection(id: string): void {
        this.connections.delete(id);
    }

    public getConnection(id: string): WebSocket | undefined {
        return this.connections.get(id);
    }

    public sendMessage(id: string, message: any): void {
        const ws = this.connections.get(id);
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify(message));
        }
    }
}
