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
    chrono::{
        DateTime,
        Utc,
    },
    std::collections::{
        HashMap,
        HashSet,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub utxos: HashMap<Hash, (bool, TransactionOutput)>,
    pub target: U256,
    pub blocks: Vec<Block>,
    #[serde(default, skip_serializing)]
    pub mempool: Vec<(DateTime<Utc>, Transaction)>,
}

impl Blockchain {
    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }

    pub fn target(&self) -> U256 {
        self.target
    }

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    pub fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        &self.mempool
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
            .retain(|(_, tx)| !block_transactions.contains(&tx.hash()));

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
                    self.utxos
                        .insert(transaction.hash(), (false, output.clone()));
                }
            }
        }
    }

    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<()> {
        // validate transaction before insertion
        // all input must match known UTXOs, and must be unique
        let mut known_inputs = HashSet::new();
        for input in transaction.inputs.iter() {
            if !self.utxos.contains_key(&input.prev_tx_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            if known_inputs.contains(&input.prev_tx_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            known_inputs.insert(input.prev_tx_output_hash);
        }

        // check if any of the utxos have the bool mark set to true and if so, find the
        // transaction that references them in mempool, remove it, and set all the utxos
        // it references to false
        for input in transaction.inputs.iter() {
            if let Some((true, _)) = self.utxos.get(&input.prev_tx_output_hash) {
                // find the references the UTXO
                // we are trying to reference
                let referencing_tx = self.mempool.iter().enumerate().find(|(_, (_, tx))| {
                    tx.outputs
                        .iter()
                        .any(|output| output.hash() == input.prev_tx_output_hash)
                });

                // If we have found one, unmark all of its UTXOs
                if let Some((idx, (_, referencing_tx))) = referencing_tx {
                    for input in referencing_tx.inputs.iter() {
                        // set all utxos from this tx to false
                        self.utxos
                            .entry(input.prev_tx_output_hash)
                            .and_modify(|(marked, _)| {
                                *marked = false;
                            });
                    }
                    self.mempool.remove(idx);
                } else {
                    // if, somehow, there is no matching transaction, set this utxo to false
                    self.utxos
                        .entry(input.prev_tx_output_hash)
                        .and_modify(|(marked, _)| {
                            *marked = false;
                        });
                }
            }
        }

        // all inputs must be lower than all outputs
        let all_inputs = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos()
                    .get(&input.prev_tx_output_hash)
                    .expect("Bug: impossible")
                    .1
                    .value
            })
            .sum::<u64>();
        let all_outputs = transaction
            .outputs
            .iter()
            .map(|output| output.value)
            .sum::<u64>();

        if all_inputs < all_outputs {
            println!("inputs are lower than outputs");
            return Err(BtcError::InvalidTransaction);
        }

        // Mark the UTXOs as used
        for input in transaction.inputs.iter() {
            self.utxos
                .entry(input.prev_tx_output_hash)
                .and_modify(|(marked, _)| {
                    *marked = true;
                });
        }

        // push the tx to the mempool
        self.mempool.push((Utc::now(), transaction));
        // sort by miner fee
        self.mempool.sort_by_key(|(_, tx)| {
            let all_inputs = tx
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_tx_output_hash)
                        .expect("Bug: Impossible")
                        .1
                        .value
                })
                .sum::<u64>();
            let all_outputs = tx.outputs.iter().map(|output| output.value).sum::<u64>();
            let miner_fee = all_inputs - all_outputs;
            miner_fee
        });

        Ok(())
    }

    pub fn clean_up_mempool(&mut self) {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark: Vec<Hash> = Vec::new();
        self.mempool.retain(|(timestamp, transaction)| {
            if now - *timestamp
                > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64)
            {
                // push all utxos to unmark to the vector so we can unmark them later
                utxo_hashes_to_unmark.extend(
                    transaction
                        .inputs
                        .iter()
                        .map(|input| input.prev_tx_output_hash),
                );
                false
            } else {
                true
            }
        });

        // unmark all of the UTXOs
        for hash in utxo_hashes_to_unmark.into_iter() {
            self.utxos.entry(hash).and_modify(|(marked, _)| {
                *marked = false;
            });
        }
    }
}
