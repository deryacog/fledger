use sphinx_packet::SphinxPacket;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct Sphinx {
    #[serde(serialize_with = "serialize_sphinx_packet", deserialize_with = "deserialize_sphinx_packet")]
    pub inner: SphinxPacket,
}

impl std::fmt::Debug for Sphinx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sphinx")
            .field("header", &self.inner.header)
            .finish()
    }
}

pub fn serialize_sphinx_packet<S>(sphinx_packet: &SphinxPacket, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes = sphinx_packet.to_bytes();
    serializer.serialize_bytes(&bytes)
}

pub fn deserialize_sphinx_packet<'de, D>(deserializer: D) -> std::result::Result<SphinxPacket, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    SphinxPacket::from_bytes(&bytes).map_err(serde::de::Error::custom)
}

impl PartialEq for Sphinx {
    fn eq(&self, other: &Self) -> bool {
        self.inner.to_bytes() == other.inner.to_bytes()
    }
}

impl Clone for Sphinx {
    fn clone(&self) -> Self {
        let mut buffer = Vec::new();
        serialize_sphinx_packet(&self.inner, &mut serde_json::Serializer::new(&mut buffer)).unwrap();
        let cloned_packet = deserialize_sphinx_packet(&mut serde_json::Deserializer::from_slice(&buffer)).unwrap();
        Sphinx { inner: cloned_packet }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sphinx_packet::header::SphinxHeader;
    use sphinx_packet::payload::Payload;
    use rand::RngCore;

    fn create_dummy_sphinx_packet() -> SphinxPacket {
        let mut header_bytes = [0u8; 348];
        rand::thread_rng().fill_bytes(&mut header_bytes);

        let mut payload_bytes = vec![0u8; 1024];
        rand::thread_rng().fill_bytes(&mut payload_bytes);

        SphinxPacket {
            header: SphinxHeader::from_bytes(&header_bytes).unwrap(),
            payload: Payload::from_bytes(&payload_bytes).unwrap(),
        }
    }

    #[test]
    fn test_sphinx_debug() {
        let packet = create_dummy_sphinx_packet();
        let sphinx = Sphinx { inner: packet };
        let debug_output = format!("{:?}", sphinx);
        println!("Debug output: {}", debug_output);
        assert!(debug_output.contains("Sphinx"));
        assert!(debug_output.contains("header"));
    }

    #[test]
    fn test_sphinx_serialization() {
        let packet = create_dummy_sphinx_packet();
        let sphinx = Sphinx { inner: packet };
        
        let mut serialized = Vec::new();
        bincode::serialize_into(&mut serialized, &sphinx).unwrap();
        
        let deserialized: Sphinx = bincode::deserialize(&serialized).unwrap();
        
        assert_eq!(sphinx, deserialized);
    }

    #[test]
    fn test_sphinx_clone() {
        let packet = create_dummy_sphinx_packet();
        let sphinx = Sphinx { inner: packet };
        let cloned_sphinx = sphinx.clone();
        assert_eq!(sphinx, cloned_sphinx);
    }

    #[test]
    fn test_sphinx_equality() {
        let packet1 = create_dummy_sphinx_packet();
        let sphinx1 = Sphinx { inner: packet1 };
        let sphinx2 = sphinx1.clone();
        assert_eq!(sphinx1, sphinx2);
    }


}