use serde::Deserialize;
use std::error::Error;
use zeromq::{Socket, SocketRecv, SubSocket};

#[derive(Debug, Deserialize)]
pub struct BlockNotification {
    pub block_id: String,
    pub block_height: u64,
    pub wide_difficulty: String,
    // Add other fields as necessary for Curve Tree anchoring
}

#[derive(Debug, Deserialize)]
pub struct TxpoolNotification {
    pub tx_hash: String,
    pub blob_size: u64,
    pub weight: u64,
    pub fee: u64,
}

pub async fn start_zmq_listener(daemon_address: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "Starting Tomori L1 Bridge: Connecting to Monero daemon ZMQ at {}",
        daemon_address
    );

    let mut socket = SubSocket::new();
    socket.connect(daemon_address).await?;

    // Subscribe to chain updates and mempool additions
    socket.subscribe("json-minimal-chain_main").await?;
    socket.subscribe("json-minimal-txpool_add").await?;

    println!("Successfully subscribed to ZMQ topics.");

    // The listener loop
    loop {
        // Monero sends ZMQ messages as multipart: [Topic, Payload]
        let msg = socket.recv().await?;

        if msg.len() < 2 {
            continue; // Ignore malformed messages
        }

        let topic_bytes = msg.get(0).expect("Missing topic frame");
        let payload_bytes = msg.get(1).expect("Missing payload frame");
        let topic = String::from_utf8_lossy(&topic_bytes);
        let payload_str = String::from_utf8_lossy(&payload_bytes);

        match topic.as_ref() {
            "json-minimal-chain_main" => {
                // Parse the payload into our BlockNotification struct
                if let Ok(parsed) = serde_json::from_str::<Vec<BlockNotification>>(&payload_str) {
                    if let Some(block_data) = parsed.first() {
                        println!(
                            "🔗 [L1 CHAIN] New Block Mined! Height: {}, Hash: {}",
                            block_data.block_height, block_data.block_id
                        );
                        // TODO: Trigger L2 state anchor verification
                    }
                }
            }
            "json-minimal-txpool_add" => {
                // Parse the payload into our TxpoolNotification struct
                if let Ok(parsed) = serde_json::from_str::<Vec<TxpoolNotification>>(&payload_str) {
                    for tx in parsed {
                        println!("📥 [L1 MEMPOOL] New TX: {} (Fee: {})", tx.tx_hash, tx.fee);
                        // TODO: Check if this TX is a Tomori L2 proof anchor
                    }
                }
            }
            _ => {
                println!("Received unknown topic: {}", topic);
            }
        }
    }
}
