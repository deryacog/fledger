use x25519_dalek::PublicKey;
use std::time::SystemTime;

// Add this struct before the NodeBehavior trait
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub sender: PublicKey,
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
