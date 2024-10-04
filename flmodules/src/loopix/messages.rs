use serde::{Deserialize, Serialize};
use sphinx_packet::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ModuleMessage {
    Random(crate::random_connections::messages::RandomMessage),
    Ping(crate::ping::messages::PingMessage),
    Loopix(LoopixMessage),
}

#[derive(Serialize, Deserialize)]
pub enum LoopixMessage {
    #[serde(serialize_with = "serialize_sphinx_packet", deserialize_with = "deserialize_sphinx_packet")]
    Sphinx(SphinxPacket),
}

impl std::fmt::Debug for LoopixMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopixMessage::Sphinx(_) => write!(f, "LoopixMessage::Sphinx(...)"),
        }
    }
}

impl Clone for LoopixMessage {
    fn clone(&self) -> Self {
        match self {
            LoopixMessage::Sphinx(packet) => {
                let mut buffer = Vec::new();
                serialize_sphinx_packet(packet, &mut serde_json::Serializer::new(&mut buffer)).unwrap();
                let cloned_packet = deserialize_sphinx_packet(&mut serde_json::Deserializer::from_slice(&buffer)).unwrap();
                LoopixMessage::Sphinx(cloned_packet)
            },
        }
    }
}

impl PartialEq for LoopixMessage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LoopixMessage::Sphinx(p1), LoopixMessage::Sphinx(p2)) => p1.to_bytes() == p2.to_bytes(),
        }
    }
}

pub fn serialize_sphinx_packet<S>(packet: &SphinxPacket, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes = packet.to_bytes();
    serializer.serialize_bytes(&bytes)
}

pub fn deserialize_sphinx_packet<'de, D>(deserializer: D) -> std::result::Result<SphinxPacket, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    SphinxPacket::from_bytes(&bytes).map_err(serde::de::Error::custom)
}
