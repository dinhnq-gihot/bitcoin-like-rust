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
    bigdecimal::BigDecimal,
    std::collections::{
        HashMap,
        HashSet,
    },
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
        // Remove transactions from mempool that are now in the block
        let block_transactions = block
            .transactions
            .iter()
            .map(|tx| tx.hash())
            .collect::<HashSet<_>>();

        self.mempool
            .retain(|tx| !block_transactions.contains(&tx.hash()));

        self.blocks.push(block);

        self.try_adjust_target();
        Ok(())
    }

    // try to adjust the target of the blockchain
    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        if self.block_height() % crate::DIFFICULTY_UPDATE_INTERVAL != 0 {
            return;
        }

        //measure the time it took to mine the last crate::DIFFICULTY_UPDATE_INTERVAL
        // blocks with chrono
        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;
        let time_diff = end_time - start_time;

        //convert time_diff to seconds
        let time_diff_seconds = time_diff.num_seconds();
        // calcualte the ideal number of seconds
        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        // multiply the current target by actual time didvided by ideal time
        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10)
            .expect("Bug: impossible")
            * (BigDecimal::from(time_diff_seconds) / BigDecimal::from(target_seconds));

        let new_target_str = new_target
            .to_string()
            .split('.')
            .next()
            .expect("Bug: Expected a decimal point")
            .to_owned();
        let new_target = U256::from_str_radix(&new_target_str, 10).expect("Bug: Impossible");

        // clamp new_target to be within the range of 4 * self.target and self.target /
        // 4
        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };

        // if the new target is more than the minimum target, set it to the minimum
        // target
        self.target = new_target.min(crate::MIN_TARGET);
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
