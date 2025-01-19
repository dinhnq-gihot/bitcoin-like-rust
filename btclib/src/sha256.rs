use {
    crate::U256,
    serde::Serialize,
    std::fmt::Display,
};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(U256);

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl Hash {
    pub fn hash<T: serde::Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = vec![];

        if let Err(e) = ciborium::into_writer(data, &mut serialized) {
            panic!(
                "Failed to serizalize data: {:?}. \
                    This should not happend",
                e
            )
        }
        let hash = sha256::digest(&serialized);
        let hash_bytes = hex::decode(hash).unwrap();
        let hash_array: [u8; 32] = hash_bytes.as_slice().try_into().unwrap();

        Hash(U256::from(hash_array))
    }

    pub fn zero() -> Self {
        Hash(U256::zero())
    }

    pub fn matches_target(&self, target: U256) -> bool {
        self.0 < target
    }

    // convert to bytes
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes: Vec<u8> = vec![0; 32];
        self.0.to_little_endian(&mut bytes);
        bytes.as_slice().try_into().unwrap()
    }
}
