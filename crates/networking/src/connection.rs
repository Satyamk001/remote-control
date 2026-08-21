use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tracing::error;

use crate::protocol::ProtocolMessage;
use crate::transport::Transport;

pub struct Connection {
    pub sender: mpsc::Sender<ProtocolMessage>,
    pub receiver: mpsc::Receiver<ProtocolMessage>,
}

impl Connection {
    pub fn start<T: Transport + 'static>(transport: T) -> Self {
        let (tx_out, mut rx_out) = mpsc::channel::<ProtocolMessage>(100);
        let (tx_in, rx_in) = mpsc::channel::<ProtocolMessage>(100);
        
        let transport = Arc::new(Mutex::new(transport));
        let transport_read = transport.clone();
        let transport_write = transport.clone();

        // Write Task
        tokio::spawn(async move {
            while let Some(msg) = rx_out.recv().await {
                let mut t = transport_write.lock().await;
                if let Err(e) = t.send(&msg).await {
                    error!("Failed to send message: {:?}", e);
                    break;
                }
            }
        });

        // Read Task
        tokio::spawn(async move {
            loop {
                let mut t = transport_read.lock().await;
                match t.receive().await {
                    Ok(msg) => {
                        if tx_in.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Read error: {:?}", e);
                        break;
                    }
                }
                // Yield to allow write task to acquire the lock
                tokio::task::yield_now().await;
            }
        });

        Self {
            sender: tx_out,
            receiver: rx_in,
        }
    }
}

pub fn get_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}
