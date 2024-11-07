use {
    crate::{
        sha256::Hash,
        types::transaction::Transaction,
    },
    std::{
        fs::File,
        io::{
            Read,
            Result as IoResult,
            Write,
        },
        path::Path,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleRoot(Hash);

impl MerkleRoot {
    // calculature the merkle root of if a block's transaction
    pub fn calculature(transactions: &[Transaction]) -> MerkleRoot {
        let mut layer: Vec<Hash> = vec![];
        for transaction in transactions {
            layer.push(Hash::hash(transaction));
        }
        while layer.len() > 1 {
            let mut new_layer = vec![];
            for pair in layer.chunks(2) {
                let left = pair[0];
                let right = pair.get(1).unwrap_or(&pair[0]);
                new_layer.push(Hash::hash(&[left, *right]));
            }
            layer = new_layer;
        }
        MerkleRoot(layer[0])
    }
}

pub trait Saveable
where
    Self: Sized,
{
    fn load<I: Read>(reader: I) -> IoResult<Self>;
    fn save<O: Write>(&self, reader: O) -> IoResult<()>;
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> IoResult<()> {
        let file = File::create(&path)?;
        self.save(file)
    }
    fn load_from_file<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let file = File::open(&path)?;
        Self::load(file)
    }
}