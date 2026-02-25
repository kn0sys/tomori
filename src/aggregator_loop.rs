use crate::l2_mempool::{L2Transaction, TomoriDB};
use crate::rpc;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Calculates the state root of the aggregated payload.
/// TODO: implement a real
/// Merkle tree using a crate like `tiny-keccak` (since Monero uses Keccak) or `blake3`.
fn calculate_merkle_root(_payload: &[u8]) -> [u8; 32] {
    // Returning a dummy 32-byte array to satisfy the compiler for now
    [0u8; 32]
}

/// The main loop for the L2 Block Producer / Aggregator
pub async fn start_aggregator_engine(db: Arc<TomoriDB>) {
    println!("⚙️ Starting Tomori Consensus & Proof Aggregation Engine...");

    loop {
        // Dictates how often we attempt to create a new L2 batch
        sleep(Duration::from_secs(15)).await;

        // Fetch all pending transactions (requires an iterator over the mempool CF)
        let mempool_txs = match db.get_all_mempool_txs() {
            Ok(txs) => txs,
            Err(e) => {
                eprintln!("Failed to read mempool: {}", e);
                continue;
            }
        };

        if mempool_txs.is_empty() {
            continue;
        }

        println!(
            "📦 Found {} pending L2 transactions. Beginning aggregation...",
            mempool_txs.len()
        );

        let mut valid_txs = Vec::new();

        // 1. Verification Phase
        for tx in mempool_txs {
            if verify_curve_tree_proof(&tx, &db) {
                valid_txs.push(tx);
            } else {
                println!(
                    "❌ Invalid proof detected for TX: {:?}. Evicting from mempool.",
                    hex::encode(tx.tx_hash)
                );
                let _ = db.delete_mempool_tx(&tx.tx_hash);
            }
        }

        if valid_txs.is_empty() {
            continue;
        }

        // 2. Aggregation Phase
        let aggregated_payload = aggregate_proofs(&valid_txs);
        let state_root = calculate_merkle_root(&aggregated_payload);

        let wallet_url = "http://127.0.0.1:38083/json_rpc"; // Default stagenet wallet RPC
        let validator_addr = "55LJ9... (your stagenet address)";

        match rpc::anchor_l2_batch_to_l1(wallet_url, &state_root, validator_addr).await {
            Ok(_l1_tx_hash) => {
                // 3. Broadcast the heavy payload to the L2 P2P network so peers can sync
                // libp2p_swarm.behaviour_mut().gossipsub.publish(topic, aggregated_payload);

                // 4. Update local RocksDB history with the new anchor
            }
            Err(e) => eprintln!("Failed to anchor batch to L1: {}", e),
        }

        // 3. Local State Update & L1 Prep
        println!(
            "✅ Successfully aggregated {} proofs. Ready for L1 anchoring.",
            valid_txs.len()
        );

        // TODO: Pass the `aggregated_payload` to the L1 RPC bridge to be broadcasted to the Monero daemon.
    }
}

/// Verifies the mathematical integrity of a single L2 transaction
fn verify_curve_tree_proof(_tx: &L2Transaction, _db: &Arc<TomoriDB>) -> bool {
    // TODO:
    // 1. Fetch the referenced Curve Tree root/path from `db` (tree_cache CF).
    // 2. Run the Bulletproofs+ verification against the transaction's proof bytes.
    // 3. Verify the transaction signature.

    // Stubbed as valid for now
    true
}

/// Condenses multiple valid Curve Tree proofs into a single batch
fn aggregate_proofs(txs: &[L2Transaction]) -> Vec<u8> {
    // In production:
    // 1. Combine the individual bulletproofs into a single aggregated proof.
    // 2. Calculate the new L2 state root.
    // 3. Serialize the final payload for the Monero L1 OP_RETURN / extra data field.

    let mut batch_payload = Vec::new();
    for tx in txs {
        batch_payload.extend_from_slice(&tx.tx_hash);
    }

    batch_payload
}
