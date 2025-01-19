pub mod crypto;
pub mod error;
pub mod sha256;
pub mod types;
pub mod util;

#[macro_use]
extern crate ciborium;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate sha256 as sha256_lib;

// pub use error::*;
use uint::construct_uint;

construct_uint! {
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}
