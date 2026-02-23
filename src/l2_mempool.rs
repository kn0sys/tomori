use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
// Assuming wincode is added to your dependencies
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
}
