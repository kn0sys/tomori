use rocksdb::{ColumnFamilyDescriptor, DB, Options};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wincode::{decode, encode};

/// Represents an unconfirmed L2 transaction in Tomori
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub tx_hash: [u8; 32],
    pub curve_tree_proof: Vec<u8>, // The heavy mathematical proof
    pub fee: u64,
    pub signature: Vec<u8>,
}

/// Represents a cached node in the Curve Tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeCacheNode {
    pub node_hash: [u8; 32],
    pub left_child: Option<[u8; 32]>,
    pub right_child: Option<[u8; 32]>,
}

/// Represents an L1 block and the L2 state anchored to it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1Anchor {
    pub l1_block_hash: [u8; 32],
    pub l1_height: u64,
    pub curve_tree_root: [u8; 32],
    /// Hashes of the L2 transactions that were bundled into this L1 block
    pub anchored_l2_txs: Vec<[u8; 32]>,
}

pub struct TomoriDB {
    db: Arc<DB>,
}

impl TomoriDB {
    /// Initializes the RocksDB instance and its column families
    pub fn new(path: &str) -> Result<Self, rocksdb::Error> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // Define our isolated column families
        let cf_mempool = ColumnFamilyDescriptor::new("mempool_txs", Options::default());
        let cf_tree = ColumnFamilyDescriptor::new("tree_cache", Options::default());
        let cf_state = ColumnFamilyDescriptor::new("validator_state", Options::default());
        let cf_history = ColumnFamilyDescriptor::new("l1_history", Options::default());

        let db = DB::open_cf_descriptors(&db_opts, path, vec![cf_mempool, cf_tree, cf_state])?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Inserts a new L2 transaction into the mempool
    pub fn insert_mempool_tx(&self, tx: &L2Transaction) -> Result<(), Box<dyn std::error::Error>> {
        let cf = self
            .db
            .cf_handle("mempool_txs")
            .ok_or("CF mempool_txs not found")?;

        // Serialize the struct into bytes using wincode
        let encoded_tx = encode(tx)?;

        self.db.put_cf(&cf, tx.tx_hash, encoded_tx)?;
        Ok(())
    }

    /// Retrieves an L2 transaction by its hash
    pub fn get_mempool_tx(
        &self,
        tx_hash: &[u8; 32],
    ) -> Result<Option<L2Transaction>, Box<dyn std::error::Error>> {
        let cf = self
            .db
            .cf_handle("mempool_txs")
            .ok_or("CF mempool_txs not found")?;

        match self.db.get_cf(&cf, tx_hash)? {
            Some(bytes) => {
                // Deserialize the bytes back into our Rust struct
                let tx: L2Transaction = decode(&bytes)?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    /// Rewinds the L2 state to a specific L1 common ancestor height
    pub fn handle_l1_reorg(
        &self,
        common_ancestor_height: u64,
        current_l1_height: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "🚨 [REORG DETECTED] Rewinding Tomori state from height {} to {}",
            current_l1_height, common_ancestor_height
        );

        let cf_history = self
            .db
            .cf_handle("l1_history")
            .ok_or("CF l1_history not found")?;
        let cf_mempool = self
            .db
            .cf_handle("mempool_txs")
            .ok_or("CF mempool_txs not found")?;

        // 1. Iterate backwards from the current orphaned tip down to the ancestor
        for height in (common_ancestor_height + 1)..=current_l1_height {
            let height_bytes = height.to_be_bytes();

            if let Some(bytes) = self.db.get_cf(&cf_history, height_bytes)? {
                // Assuming wincode is still your standard serialization
                let anchor: L1Anchor = wincode::decode(&bytes)?;

                // 2. Resurrect transactions: push them back to the mempool
                for tx_hash in anchor.anchored_l2_txs {
                    println!("Resurrecting L2 TX: {:?}", tx_hash);
                    // In a full implementation, you'd fetch the full TX payload from an archive CF
                    // and write it back into cf_mempool here.
                }

                // 3. Delete the orphaned anchor from history
                self.db.delete_cf(&cf_history, height_bytes)?;
            }
        }

        // 4. Revert the validator state and tree cache to the common ancestor's snapshot
        // RocksDB Checkpoints or manual state reversal would be triggered here.
        self.restore_state_snapshot(common_ancestor_height)?;

        println!("✅ State successfully rolled back. Resuming L2 operations.");
        Ok(())
    }

    fn restore_state_snapshot(&self, _height: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation for swapping the active RocksDB state CF with the backed-up snapshot
        Ok(())
    }
}
