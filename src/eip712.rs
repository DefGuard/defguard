use ethers::contract::Eip712;
use ethers::core::types::transaction::eip712::{EIP712Domain, Eip712, Eip712DomainType};
use ethers::prelude::*;
use ethers::utils::hex;
use rocket::serde::json::serde_json;
use std::collections::BTreeMap;

pub type Types = BTreeMap<String, Vec<Eip712DomainType>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypedData {
    /// Signing domain metadata. The signing domain is the intended context for the signature (e.g.
    /// the dapp, protocol, etc. that it's intended for). This data is used to construct the domain
    /// seperator of the message.
    pub domain: EIP712Domain,
    /// The custom types used by this message.
    pub types: Types,
    #[serde(rename = "primaryType")]
    /// The type of the message.
    #[serde(rename = "value")]
    pub primary_type: String,
    /// The message to be signed.
    pub message: BTreeMap<String, serde_json::Value>,
}

pub fn create_eip712_message(default_message: String, nonce: String, address: String) -> String {
    // Define the typed data message
    let message = SignMessage {
        content: default_message,
        address,
        nonce,
    };

    let permit_hash = message.encode_eip712().unwrap();
    hex::encode(permit_hash)
}
