use crate::U256;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::fmt;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(pub U256);

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl Hash {
    pub fn hash<T: Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = vec![];
        if let Err(e) = ciborium::into_writer(data, &mut serialized) {
            panic!(
                "Failed to serialize data: {:?}. \
            This should not happen",
                e
            );
        }

        let hash = digest(&serialized);
        let hash_bytes = hex::decode(hash).unwrap();
        let hash_array: [u8; 32] = hash_bytes.as_slice().try_into().unwrap();

        Self(U256::from(hash_array))
    }

    // check if a hash matches a target
    pub fn matches_target(&self, target: U256) -> bool {
        self.0 <= target
    }

    // zero hash
    pub fn zero() -> Self {
        Self(U256::zero())
    }

    // convert to bytes
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes = vec![0u8; 32];
        self.0.to_little_endian(&mut bytes);
        bytes.as_slice().try_into().unwrap()
    }
}