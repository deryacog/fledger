use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use sphinx_packet::route::{NodeAddressBytes, DestinationAddressBytes};
use std::{sync::Arc, time::SystemTime};
use x25519_dalek::{PublicKey, StaticSecret};
use concurrent_queue::ConcurrentQueue;

use crate::random_connections::messages::ModuleMessage;

use super::{sphinx::Sphinx};

// //////////////////////// Config ///////////////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixConfig {
    pub lambda_loop: f64,       // Loop traffic rate (user)
    pub lambda_drop: f64,       // Drop cover traffic rate (user)
    pub lambda_payload: f64,    // Payload traffic rate (user)
    pub path_length: i32,       // Path length (number of mix nodes in the route)
    pub mean_delay: f64,        // Mean delay at each mix node (in seconds)
    pub lambda_loop_mix: f64,   // Loop traffic rate (mix)
}

impl Default for LoopixConfig {
    fn default() -> Self {
        LoopixConfig {
            lambda_loop: 10.0,
            lambda_drop: 10.0,
            lambda_payload: 10.0,
            path_length: 3,
            mean_delay: 0.001,
            lambda_loop_mix: 10.0,
        }
    }
}

// //////////////////////// Storage ////////////////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LoopixStorageSave {
    V1(LoopixStorage),
}

impl LoopixStorageSave {
    pub fn from_str(data: &str) -> Result<LoopixStorage, serde_yaml::Error> {
        return Ok(serde_yaml::from_str::<LoopixStorageSave>(data)?.to_latest());
    }

    fn to_latest(self) -> LoopixStorage {
        match self {
            LoopixStorageSave::V1(es) => es,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixStorage {
    pub last_loop_cover: SystemTime,
    pub last_drop: SystemTime,
    pub last_payload: SystemTime,
    pub last_pull: SystemTime,
    pub last_real: SystemTime,
}

impl LoopixStorage {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string::<LoopixStorageSave>(&LoopixStorageSave::V1(self.clone()))
    }
}

impl Default for LoopixStorage {
    fn default() -> Self {
        LoopixStorage {
            last_loop_cover: SystemTime::now(),
            last_drop: SystemTime::now(),
            last_payload: SystemTime::now(),
            last_pull: SystemTime::now(),
            last_real: SystemTime::now(),
        }
    }
}

// //////////////////////// Core ////////////////////////////////////////////////////////
#[derive(Serialize, Deserialize)]
pub struct LoopixCore {
    pub storage: LoopixStorage,
    pub config: LoopixConfig,
    
    #[serde(serialize_with = "serialize_public_key", deserialize_with = "deserialize_public_key")]
    pub_key: PublicKey,
    
    #[serde(serialize_with = "serialize_static_secret", deserialize_with = "deserialize_static_secret")]
    secret_key: StaticSecret,

    #[serde(skip, default = "default_queue")]
    queue: Arc<ConcurrentQueue<ModuleMessage>>,
    max_queue_size: usize,
}

fn default_queue() -> Arc<ConcurrentQueue<ModuleMessage>> {
    Arc::new(ConcurrentQueue::bounded(100))
}

impl Clone for LoopixCore {
    /// DOES NOT COPY THE CONTENTS OF THE QUEUE
    fn clone(&self) -> Self { 
        Self {
            storage: self.storage.clone(),
            config: self.config.clone(),
            pub_key: self.pub_key,
            secret_key: self.secret_key.clone(),
            queue: Arc::new(ConcurrentQueue::bounded(self.max_queue_size)),
            max_queue_size: self.max_queue_size,
        }
    }
}

impl LoopixCore {
    pub fn new(storage: LoopixStorage, config: LoopixConfig, max_queue_size: usize) -> Self {
        let (pub_key, secret_key) = Self::generate_key_pair();

        Self {
            storage,
            config,
            pub_key,
            secret_key,
            queue: Arc::new(ConcurrentQueue::bounded(max_queue_size)),
            max_queue_size,
        }
    }

    pub fn enqueue_packet(&self, packet: ModuleMessage) -> Result<(), &'static str> {
        self.queue.push(packet).map_err(|_| "Queue is full")
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dequeue_packet(&self) -> Option<ModuleMessage> {
        self.queue.pop().ok()
    }

    // TODO maybe errors
    pub fn node_address_from_node_id(node_id: NodeID) -> NodeAddressBytes {
        let node_id_bytes = node_id.to_bytes();
        NodeAddressBytes::from_bytes(node_id_bytes)
    }

    // TODO maybe errors
    pub fn node_id_from_node_address(node_address: NodeAddressBytes) -> NodeID {
        let node_address_bytes = node_address.as_bytes();
        NodeID::from(node_address_bytes)
    }

    // TODO maybe errors
    pub fn node_id_from_destination_address(dest_addr: DestinationAddressBytes) -> NodeID {
        let dest_bytes = dest_addr.as_bytes();
        NodeID::from(dest_bytes)
    }

    // TODO maybe errors
    pub fn destination_address_from_node_id(node_id: NodeID) -> DestinationAddressBytes {
        let node_id_bytes = node_id.to_bytes();
        DestinationAddressBytes::from_bytes(node_id_bytes)
    }

    pub fn get_config(&self) -> &LoopixConfig {
        &self.config
    }

    pub fn get_storage(&self) -> &LoopixStorage {
        &self.storage
    }

    pub fn get_public_key(&self) -> &PublicKey {
        &self.pub_key
    }

    pub fn get_secret_key(&self) -> &StaticSecret {
        &self.secret_key
    }

    fn generate_key_pair() -> (PublicKey, StaticSecret) {
        let rng = rand::thread_rng();
        let private_key = StaticSecret::random_from_rng(rng);
        let public_key = PublicKey::from(&private_key);
        (public_key, private_key)
    }
}

impl PartialEq for LoopixCore {
    fn eq(&self, other: &Self) -> bool {
        self.storage == other.storage
            && self.config == other.config
            && self.pub_key == other.pub_key
            && self.secret_key.to_bytes() == other.secret_key.to_bytes()
    }
}

impl std::fmt::Debug for LoopixCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopixCore")
            .field("storage", &self.storage)
            .field("config", &self.config)
            .field("pub_key", &hex::encode(self.pub_key.as_bytes()))
            .field("secret_key", &"[secret_key]")
            .finish()
    }
}

// region: Serde functions
pub fn serialize_public_key<S>(key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let key_bytes: &[u8] = key.as_bytes();
    serializer.serialize_bytes(key_bytes)
}

pub fn deserialize_public_key<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let key_bytes: [u8; 32] = serde::Deserialize::deserialize(deserializer)?;
    Ok(PublicKey::from(key_bytes))
}

pub fn serialize_static_secret<S>(key: &StaticSecret, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let key_bytes: [u8; 32] = key.to_bytes();
    serializer.serialize_bytes(&key_bytes)
}

pub fn deserialize_static_secret<'de, D>(deserializer: D) -> Result<StaticSecret, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let key_bytes: [u8; 32] = serde::Deserialize::deserialize(deserializer)?;
    Ok(StaticSecret::from(key_bytes))
}

// endregion: Serde functions

pub trait NodeBehavior {
    fn process_packet(&self, sphinx_packet: Sphinx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_loopix_config_default() {
        let config = LoopixConfig::default();
        assert_eq!(config.lambda_loop, 10.0);
        assert_eq!(config.lambda_drop, 10.0);
        assert_eq!(config.lambda_payload, 10.0);
        assert_eq!(config.path_length, 3);
        assert_eq!(config.mean_delay, 0.001);
        assert_eq!(config.lambda_loop_mix, 10.0);
    }

    #[test]
    fn test_loopix_config_custom() {
        let custom_config = LoopixConfig {
            lambda_loop: 5.0,
            lambda_drop: 7.0,
            lambda_payload: 8.0,
            path_length: 5,
            mean_delay: 0.002,
            lambda_loop_mix: 6.0,
        };
        assert_eq!(custom_config.lambda_loop, 5.0);
        assert_eq!(custom_config.lambda_drop, 7.0);
        assert_eq!(custom_config.lambda_payload, 8.0);
        assert_eq!(custom_config.path_length, 5);
        assert_eq!(custom_config.mean_delay, 0.002);
        assert_eq!(custom_config.lambda_loop_mix, 6.0);
    }

    #[test]
    fn test_loopix_storage_default() {
        let storage = LoopixStorage::default();
        let now = SystemTime::now();
        assert!(storage.last_loop_cover <= now);
        assert!(storage.last_drop <= now);
        assert!(storage.last_payload <= now);
        assert!(storage.last_pull <= now);
        assert!(storage.last_real <= now);
    }

    #[test]
    fn test_loopix_storage_custom() {
        let custom_time = SystemTime::now() - Duration::from_secs(3600);
        let storage = LoopixStorage {
            last_loop_cover: custom_time,
            last_drop: custom_time,
            last_payload: custom_time,
            last_pull: custom_time,
            last_real: custom_time,
        };
        assert_eq!(storage.last_loop_cover, custom_time);
        assert_eq!(storage.last_drop, custom_time);
        assert_eq!(storage.last_payload, custom_time);
        assert_eq!(storage.last_pull, custom_time);
        assert_eq!(storage.last_real, custom_time);
    }

    #[test]
    fn test_loopix_storage_serialization() {
        let storage = LoopixStorage::default();
        let yaml = storage.to_yaml().unwrap();
        let deserialized = LoopixStorageSave::from_str(&yaml).unwrap();
        assert_eq!(storage, deserialized);
    }

    #[test]
    fn test_loopix_core_new() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 100);
        
        assert_eq!(core.storage, storage);
        assert_eq!(core.config, config);
        
        let (new_public_key, _) = LoopixCore::generate_key_pair();
        
        assert_ne!(core.get_public_key(), &new_public_key);
    }

    #[test]
    fn test_loopix_core_getters() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 100);
        
        assert_eq!(core.get_config(), &config);
        assert_eq!(core.get_storage(), &storage);
        assert_eq!(core.get_public_key(), &core.pub_key);
    }
    #[test]
    fn test_loopix_core_partial_eq() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core1 = LoopixCore::new(storage.clone(), config.clone(), 100);
        let core2 = LoopixCore::new(storage.clone(), config.clone(), 100);
        
        assert_ne!(core1, core2);
        
        let core3 = LoopixCore {
            storage: core1.storage.clone(),
            config: core1.config.clone(),
            pub_key: core1.pub_key,
            secret_key: core1.secret_key.clone(),
            queue: Arc::new(ConcurrentQueue::bounded(100)),
            max_queue_size: 100,
        };
        
        assert_eq!(core1, core3);
    }

    #[test]
    fn test_loopix_core_debug() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage, config, 100);
        
        let debug_output = format!("{:?}", core);
        println!("Debug output: {}", debug_output);
        
        assert!(debug_output.contains("LoopixCore"));
        assert!(debug_output.contains("storage"));
        assert!(debug_output.contains("config"));
        assert!(debug_output.contains("pub_key"));
        assert!(debug_output.contains("[secret_key]"));
    }

    #[test]
    fn test_serialize_deserialize_public_key() {
        let (pub_key, _) = LoopixCore::generate_key_pair();
        let mut serializer = serde_json::Serializer::new(Vec::new());
        serialize_public_key(&pub_key, &mut serializer).unwrap();
        let serialized = serializer.into_inner();
        
        let mut deserializer = serde_json::Deserializer::from_slice(&serialized);
        let deserialized: PublicKey = deserialize_public_key(&mut deserializer).unwrap();
        
        assert_eq!(pub_key, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_static_secret() {
        let (_, secret_key) = LoopixCore::generate_key_pair();
        let mut serializer = serde_json::Serializer::new(Vec::new());
        serialize_static_secret(&secret_key, &mut serializer).unwrap();
        let serialized = serializer.into_inner();
        
        let mut deserializer = serde_json::Deserializer::from_slice(&serialized);
        let deserialized: StaticSecret = deserialize_static_secret(&mut deserializer).unwrap();
        
        assert_eq!(secret_key.to_bytes(), deserialized.to_bytes());
    }

    #[test]
    fn test_node_address_from_node_id() {
        let node_id = NodeID::rnd();
        let node_address = LoopixCore::node_address_from_node_id(node_id.clone());
        assert_eq!(node_address.as_bytes(), node_id.to_bytes());
    }

    #[test]
    fn test_node_id_from_node_address() {
        let node_id = NodeID::rnd();
        let node_address = NodeAddressBytes::from_bytes(node_id.to_bytes());
        let result_node_id = LoopixCore::node_id_from_node_address(node_address);
        assert_eq!(result_node_id, node_id);
    }

    #[test]
    fn test_node_id_from_destination_address() {
        let node_id = NodeID::rnd();
        let dest_address = DestinationAddressBytes::from_bytes(node_id.to_bytes());
        let result_node_id = LoopixCore::node_id_from_destination_address(dest_address);
        assert_eq!(result_node_id, node_id);
    }

    #[test]
    fn test_destination_address_from_node_id() {
        let node_id = NodeID::rnd();
        let dest_address = LoopixCore::destination_address_from_node_id(node_id.clone());
        assert_eq!(dest_address.as_bytes(), node_id.to_bytes());
    }

    #[test]
    fn test_node_id_to_node_address_and_back() {
        let original_node_id = NodeID::rnd();
        let node_address = LoopixCore::node_address_from_node_id(original_node_id.clone());
        let result_node_id = LoopixCore::node_id_from_node_address(node_address);
        assert_eq!(result_node_id, original_node_id);
    }

    #[test]
    fn test_node_id_to_destination_address_and_back() {
        let original_node_id = NodeID::rnd();
        let dest_address = LoopixCore::destination_address_from_node_id(original_node_id.clone());
        let result_node_id = LoopixCore::node_id_from_destination_address(dest_address);
        assert_eq!(result_node_id, original_node_id);
    }

    #[test]
    fn test_is_queue_empty() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 2);

        assert!(core.is_queue_empty());
        
        // TODO maybe add a default/clone function to module message
        let packet = ModuleMessage {
            module: "test_module".to_string(),
            msg: "test_message".to_string(),
        };
        core.enqueue_packet(packet).unwrap();
        
        assert!(!core.is_queue_empty());
        core.dequeue_packet();
        assert!(core.is_queue_empty());
    }

    #[test]
    fn test_enqueue_packet() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 2);

        let packet1 = ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        };
        let packet2 = ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        };
        
        assert!(core.enqueue_packet(packet1).is_ok());
        assert!(core.enqueue_packet(packet2).is_ok());
        assert!(core.is_queue_empty() == false);
    }

    #[test]
    fn test_dequeue_packet() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 2);

        let packet1 = ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        };
        let packet2 = ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        };
        
        core.enqueue_packet(packet1).unwrap();
        core.enqueue_packet(packet2).unwrap();

        assert_eq!(core.dequeue_packet().is_some(), true);
        assert_eq!(core.dequeue_packet().is_some(), true);
        assert_eq!(core.dequeue_packet().is_none(), true);
    }

    #[test]
    fn test_queue_full() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 1);

        let packet1 = ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        };
        let packet2 = ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        };
        
        assert!(core.enqueue_packet(packet1).is_ok());
        assert!(core.enqueue_packet(packet2).is_err());
    }


}