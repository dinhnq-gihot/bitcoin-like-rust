use crate::U256;

pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}
pub struct BlockHeader {
    /// Timestamp of the block
    pub timestamp: u64,
    /// Nonce used to mine the block
    pub nonce: u64,
    /// Hash of the previous block
    pub prev_block_hash: [u8; 32],
    /// Merkle root of the block's transactions
    pub merkle_root: [u8; 32],
    /// target
    pub target: U256,
}
pub struct Blockchain {
    pub blocks: Vec<Block>,
}
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}
pub struct TransactionInput {}
pub struct TransactionOutput {}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain { blocks: Vec::new() }
    }
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Block {
            header,
            transactions,
        }
    }

    pub fn hash(&self) -> String {
        // TODO: Implement hash function
        unimplemented!()
    }
}

impl BlockHeader {
    pub fn new(
        timestamp: u64,
        nonce: u64,
        prev_block_hash: [u8; 32],
        merkle_root: [u8; 32],
        target: U256,
    ) -> Self {
        BlockHeader {
            timestamp,
            nonce,
            prev_block_hash,
            merkle_root,
            target,
        }
    }

    pub fn hash(&self) -> String {
        // TODO: Implement hash function
        unimplemented!()
    }
}
