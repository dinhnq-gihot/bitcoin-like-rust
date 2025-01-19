use crate::{
    sha256::Hash,
    types::Transaction,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleRoot(Hash);

impl MerkleRoot {
    pub fn calcualte(transactions: &Vec<Transaction>) -> Self {
        let mut layer: Vec<Hash> = Vec::new();

        for transaction in transactions.iter() {
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
