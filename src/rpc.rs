use reqwest::Client;
use serde_json::json;
use std::error::Error;

/// Queries the Monero daemon to find the highest block hash that matches our local L1 history.
pub async fn find_common_ancestor(
    daemon_rpc_url: &str,
    db: &crate::l2_mempool::TomoriDB,
    mut current_local_height: u64,
) -> Result<u64, Box<dyn Error>> {
    println!(
        "🔍 Searching for L1 common ancestor starting from height: {}",
        current_local_height
    );
    let client = Client::new();

    loop {
        // 1. Fetch the expected block hash from our local RocksDB history
        // (Assuming you add a helper method `get_l1_hash_at_height` to TomoriDB)
        let local_hash_opt = db.get_l1_hash_at_height(current_local_height)?;

        let local_hash = match local_hash_opt {
            Some(hash) => hash,
            None => {
                // If we don't have a record for this height, step back.
                current_local_height -= 1;
                continue;
            }
        };

        // 2. Fetch the block header from the Monero daemon at the same height
        let req_body = json!({
            "jsonrpc": "2.0",
            "id": "tomori",
            "method": "get_block_header_by_height",
            "params": {
                "height": current_local_height
            }
        });

        let res: serde_json::Value = client
            .post(daemon_rpc_url)
            .json(&req_body)
            .send()
            .await?
            .json()
            .await?;

        // Extract the daemon's block hash
        let daemon_hash = res["result"]["block_header"]["hash"]
            .as_str()
            .ok_or("Failed to parse daemon block hash from RPC response")?;

        // 3. Compare the hashes
        // Convert local_hash (likely [u8; 32]) to a hex string for comparison,
        // or decode the daemon_hash to bytes.
        let local_hash_hex = hex::encode(local_hash);

        if local_hash_hex == daemon_hash {
            println!(
                "🎯 Common ancestor found at height: {}",
                current_local_height
            );
            return Ok(current_local_height);
        }

        // 4. If they don't match, the fork is deeper. Step back and try again.
        if current_local_height == 0 {
            return Err(
                "Genesis block mismatch! The stagenet may have been completely reset.".into(),
            );
        }

        current_local_height -= 1;
    }
}
