use x25519_dalek::PublicKey;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Message {
    #[serde(serialize_with = "crate::loopix::core::serialize_public_key", deserialize_with = "crate::loopix::core::deserialize_public_key")]
    pub sender: PublicKey,
    #[serde(serialize_with = "crate::loopix::core::serialize_public_key", deserialize_with = "crate::loopix::core::deserialize_public_key")]
    pub recipient: PublicKey,
    
    pub content: Vec<u8>,
    pub timestamp: SystemTime,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            sender: PublicKey::from([0u8; 32]),
            recipient: PublicKey::from([1u8; 32]),
            content: vec![0, 1, 2, 3],
            timestamp: SystemTime::UNIX_EPOCH,
        }
    }
}
