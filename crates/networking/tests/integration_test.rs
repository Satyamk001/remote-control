use networking::protocol::{MessagePayload, ProtocolMessage};
use networking::tcp::TcpTransport;
use networking::transport::Transport;
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_tcp_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(stream);
        let msg = transport.receive().await.unwrap();
        
        if let MessagePayload::Ping(val) = msg.payload {
            let resp = ProtocolMessage {
                seq: msg.seq + 1,
                timestamp_ms: msg.timestamp_ms,
                payload: MessagePayload::Pong(val),
            };
            transport.send(&resp).await.unwrap();
        }
    });

    let stream = TcpStream::connect(addr).await.unwrap();
    let mut transport = TcpTransport::new(stream);

    let msg = ProtocolMessage {
        seq: 1,
        timestamp_ms: 0,
        payload: MessagePayload::Ping(42),
    };

    transport.send(&msg).await.unwrap();

    let response = transport.receive().await.unwrap();
    assert_eq!(response.payload, MessagePayload::Pong(42));
    assert_eq!(response.seq, 2);

    server_task.await.unwrap();
}
