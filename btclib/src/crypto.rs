use crate::sha256::Hash;
use ecdsa::{
    signature::{Signer, Verifier},
    Signature as ECDSASignature, SigningKey, VerifyingKey,
};
use k256::Secp256k1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature(pub ECDSASignature<Secp256k1>);

impl Signature {
    // sign a crate::types::TransactionOutput from its Sha256 hash
    pub fn sign_output(output_hash: &Hash, private_key: &PrivateKey) -> Self {
        let signing_key = &private_key.0;
        let signature = signing_key.sign(&output_hash.as_bytes());
        Signature(signature)
    }

    // verify signature
    pub fn verify(&self, output_hash: &Hash, public_key: &PublicKey) -> bool {
        public_key
            .0
            .verify(&output_hash.as_bytes(), &self.0)
            .is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKey(pub VerifyingKey<Secp256k1>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateKey(#[serde(with = "signkey_serde")] pub SigningKey<Secp256k1>);

mod signkey_serde {
    use ecdsa::SigningKey;
    use serde::Deserialize;

    pub fn serialize<S>(
        key: &SigningKey<super::Secp256k1>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SigningKey<super::Secp256k1>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        Ok(SigningKey::from_slice(&bytes).unwrap())
    }
}

impl PrivateKey {
    pub fn new_key() -> Self {
        PrivateKey(SigningKey::random(&mut rand::thread_rng()))
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().clone())
    }
}
