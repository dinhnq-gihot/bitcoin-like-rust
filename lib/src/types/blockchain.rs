use {
    super::{
        block::Block,
        transaction::{
            Transaction,
            TransactionOutput,
        },
    },
    crate::{
        error::{
            BtcError,
            Result,
        },
        sha256::Hash,
        util::MerkleRoot,
        U256,
    },
    std::collections::HashMap,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub utxos: HashMap<Hash, TransactionOutput>,
    pub target: U256,
    pub blocks: Vec<Block>,
    #[serde(default, skip_serializing)]
    pub mempool: Vec<Transaction>,
}

impl Blockchain {
    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            target: crate::MIN_TARGET,
            blocks: Vec::new(),
            mempool: Vec::new(),
        }
    }
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        if self.blocks.is_empty() {
            if block.header.prev_block_hash != Hash::zero() {
                println!("zero hash");
                return Err(BtcError::InvalidBlock);
            }
        } else {
            // if this is not the first block,
            // check if the previous block hash is the hash of the last block
            let last_block = self.blocks.last().unwrap();
            if block.header.prev_block_hash != last_block.hash() {
                println!("prev hash is wrong");
                return Err(BtcError::InvalidBlock);
            }
            // check if the block's hash is lesss than the target
            if !block.header.hash().matches_target(block.header.target) {
                println!("does not match target");
                return Err(BtcError::InvalidBlock);
            }
            // check if the block's merkle root is correct
            let calculated_merkle_root = MerkleRoot::calculature(&block.transactions);
            if block.header.merkle_root != calculated_merkle_root {
                println!("invalid merkle root");
                return Err(BtcError::InvalidMerkleRoot);
            }

            // check if the block's timestamp is after the last block's timestamp
            if block.header.timestamp <= last_block.header.timestamp {
                println!("invalid block timestamp");
                return Err(BtcError::InvalidBlock);
            }

            // Verify all transactions in the block
            block.verify_transactions(self.block_height(), &self.utxos)?;
        }
        self.blocks.push(block);
        Ok(())
    }

    pub fn rebuild_utxos(&mut self) {
        for block in self.blocks.iter() {
            for transaction in block.transactions.iter() {
                for input in transaction.inputs.iter() {
                    self.utxos.remove(&input.prev_tx_output_hash);
                }
                for output in transaction.outputs.iter() {
                    self.utxos.insert(transaction.hash(), output.clone());
                }
            }
        }
    }
}
