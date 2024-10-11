use flarch::nodeids::NodeID;
use crate::network::messages::NetworkIn;
use serde::{Deserialize, Serialize};
use sphinx_packet::route::{NodeAddressBytes, DestinationAddressBytes};
use std::sync::RwLock;
use std::{time::SystemTime, collections::HashMap};
use x25519_dalek::{PublicKey, StaticSecret};
use concurrent_queue::ConcurrentQueue;

use super::super::ModuleMessage;

use super::messages::{MODULE_NAME, LoopixIn};
use super::{sphinx::Sphinx};

// //////////////////////// Config ///////////////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixConfig {
    lambda_loop: f64,       // Loop traffic rate (user)
    lambda_drop: f64,       // Drop cover traffic rate (user)
    lambda_payload: f64,    // Payload traffic rate (user)
    path_length: i32,       // Path length (number of mix nodes in the route)
    mean_delay: f64,        // Mean delay at each mix node (in seconds)
    lambda_loop_mix: f64,   // Loop traffic rate (mix)
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

impl LoopixConfig {
    pub fn lambda_loop(&self) -> f64 {
        self.lambda_loop
    }
    
    pub fn lambda_drop(&self) -> f64 {
        self.lambda_drop
    }
    
    pub fn lambda_payload(&self) -> f64 {
        self.lambda_payload
    }

    pub fn path_length(&self) -> i32 {
        self.path_length
    }

    pub fn mean_delay(&self) -> f64 {
        self.mean_delay
    }

    pub fn lambda_loop_mix(&self) -> f64 {
        self.lambda_loop_mix
    }

    pub fn new(lambda_loop: f64, lambda_drop: f64, lambda_payload: f64, 
               path_length: i32, mean_delay: f64, lambda_loop_mix: f64) -> Self {
        LoopixConfig {
            lambda_loop,
            lambda_drop,
            lambda_payload,
            path_length,
            mean_delay,
            lambda_loop_mix,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LoopixStorage {
    pub last_loop_cover: RwLock<SystemTime>,
    pub last_drop: RwLock<SystemTime>,
    pub last_payload: RwLock<SystemTime>,
    pub last_pull: RwLock<SystemTime>,
    pub last_real: RwLock<SystemTime>,
    #[serde(skip)]
    pub node_public_keys: RwLock<HashMap<NodeID, PublicKey>>,
}

impl Clone for LoopixStorage {
    fn clone(&self) -> Self {
        LoopixStorage {
            last_loop_cover: RwLock::new(*self.last_loop_cover.read().unwrap()),
            last_drop: RwLock::new(*self.last_drop.read().unwrap()),
            last_payload: RwLock::new(*self.last_payload.read().unwrap()),
            last_pull: RwLock::new(*self.last_pull.read().unwrap()),
            last_real: RwLock::new(*self.last_real.read().unwrap()),
            node_public_keys: RwLock::new(self.node_public_keys.read().unwrap().clone()),
        }
    }
}

impl PartialEq for LoopixStorage {
    fn eq(&self, other: &Self) -> bool {
        *self.last_loop_cover.read().unwrap() == *other.last_loop_cover.read().unwrap() &&
        *self.last_drop.read().unwrap() == *other.last_drop.read().unwrap() &&
        *self.last_payload.read().unwrap() == *other.last_payload.read().unwrap() &&
        *self.last_pull.read().unwrap() == *other.last_pull.read().unwrap() &&
        *self.last_real.read().unwrap() == *other.last_real.read().unwrap() &&
        *self.node_public_keys.read().unwrap() == *other.node_public_keys.read().unwrap()
    }
}

impl LoopixStorage {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string::<LoopixStorageSave>(&LoopixStorageSave::V1(self.clone()))
    }
}

impl Default for LoopixStorage {
    fn default() -> Self {
        LoopixStorage {
            last_loop_cover: RwLock::new(SystemTime::now()),
            last_drop: RwLock::new(SystemTime::now()),
            last_payload: RwLock::new(SystemTime::now()),
            last_pull: RwLock::new(SystemTime::now()),
            last_real: RwLock::new(SystemTime::now()),
            node_public_keys: RwLock::new(HashMap::new()),
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
    queue: ConcurrentQueue<NetworkIn>,
    max_queue_size: usize,
}

fn default_queue() -> ConcurrentQueue<NetworkIn> {
    ConcurrentQueue::bounded(100) // TODO probably
}

impl Clone for LoopixCore {
    /// DOES NOT COPY THE CONTENTS OF THE QUEUE
    fn clone(&self) -> Self { 
        Self {
            storage: self.storage.clone(),
            config: self.config.clone(),
            pub_key: self.pub_key,
            secret_key: self.secret_key.clone(),
            queue: ConcurrentQueue::bounded(self.max_queue_size),
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
            queue: ConcurrentQueue::bounded(max_queue_size),
            max_queue_size,
        }
    }

    pub fn create_sphinx_packet(&self, dest: NodeID, msg: ModuleMessage) -> Sphinx { // TODO I'm not sure if this should be here
                // TODO public keys
                // TODO generate route
                // TODO generate delays
                // let sphinx_packet = SphinxPacket::new(message.clone(), &route, &destination, &delays).unwrap();
               !todo!()
    }

    pub fn enqueue_packet(&self, packet: NetworkIn) -> Result<(), &'static str> {
        self.queue.push(packet).map_err(|_| "Queue is full")
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dequeue_packet(&self) -> Option<NetworkIn> {
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
    use std::thread;
    use std::sync::Arc;

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
    
        assert!(*storage.last_loop_cover.read().unwrap() <= now);
        assert!(*storage.last_drop.read().unwrap() <= now);
        assert!(*storage.last_payload.read().unwrap() <= now);
        assert!(*storage.last_pull.read().unwrap() <= now);
        assert!(*storage.last_real.read().unwrap() <= now);
    }

    #[test]
    fn test_loopix_storage_custom() {
        let custom_time = SystemTime::now() - Duration::from_secs(3600);
        let storage = LoopixStorage {
            last_loop_cover: RwLock::new(custom_time),
            last_drop: RwLock::new(custom_time),
            last_payload: RwLock::new(custom_time),
            last_pull: RwLock::new(custom_time),
            last_real: RwLock::new(custom_time),
            node_public_keys: RwLock::new(HashMap::new()),
        };
    
        assert_eq!(*storage.last_loop_cover.read().unwrap(), custom_time);
        assert_eq!(*storage.last_drop.read().unwrap(), custom_time);
        assert_eq!(*storage.last_payload.read().unwrap(), custom_time);
        assert_eq!(*storage.last_pull.read().unwrap(), custom_time);
        assert_eq!(*storage.last_real.read().unwrap(), custom_time);
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
            queue: ConcurrentQueue::bounded(100),
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
        let msg = ModuleMessage {
            module: "test_module".to_string(),
            msg: "test_message".to_string(),
        };

        let packet = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), msg);
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

        let packet1 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        });
        let packet2 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        });
        
        assert!(core.enqueue_packet(packet1).is_ok());
        assert!(core.enqueue_packet(packet2).is_ok());
        assert!(core.is_queue_empty() == false);
    }

    #[test]
    fn test_dequeue_packet() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone(), 2);

        let packet1 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        });
        let packet2 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        });
        
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

        let packet1 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_1".to_string(),
            msg: "test_message_1".to_string(),
        });
        let packet2 = NetworkIn::SendNodeModuleMessage(NodeID::rnd(), ModuleMessage {
            module: "test_module_2".to_string(),
            msg: "test_message_2".to_string(),
        });
        
        assert!(core.enqueue_packet(packet1).is_ok());
        assert!(core.enqueue_packet(packet2).is_err());
    }

    #[test]
    fn test_concurrent_last_loop_cover_update() {
        let storage = Arc::new(LoopixStorage::default());
        let thread_count = 5;
        let mut handles = vec![];
        let init_time = SystemTime::now();

        for _ in 0..thread_count {
            let storage_clone = Arc::clone(&storage);
            let handle = thread::spawn(move || {
                let new_time = SystemTime::now();
                *storage_clone.last_loop_cover.write().unwrap() = new_time;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(*storage.last_loop_cover.read().unwrap() > init_time);
    }

    #[test]
    fn test_concurrent_node_public_keys_update() {
        let storage = Arc::new(LoopixStorage::default());
        let thread_count = 5;
        let mut handles = vec![];

        for i in 0..thread_count {
            let storage_clone = Arc::clone(&storage);
            let handle = thread::spawn(move || {
                let node_id = NodeID::rnd();
                let (pub_key, _) = LoopixCore::generate_key_pair();
                storage_clone.node_public_keys.write().unwrap().insert(node_id, pub_key);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(storage.node_public_keys.read().unwrap().len(), thread_count);
    }
}