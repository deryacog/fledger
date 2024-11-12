use std::collections::HashMap;
use rand::seq::IteratorRandom;
use sphinx_packet::header::delays::Delay;
use x25519_dalek::{PublicKey, StaticSecret};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashSet;
use crate::{loopix::sphinx::Sphinx, nodeconfig::NodeInfo};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopixStorageSave {
    V1(LoopixStorage),
}

impl LoopixStorageSave {
    pub fn from_str(data: &str) -> Result<LoopixStorage, serde_yaml::Error> {
        Ok(serde_yaml::from_str::<LoopixStorageSave>(data)?.to_latest())
    }

    fn to_latest(self) -> LoopixStorage {
        match self {
            LoopixStorageSave::V1(es) => es,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkStorage {
    node_id: NodeID,
    #[serde(
        serialize_with = "serialize_static_secret",
        deserialize_with = "deserialize_static_secret"
    )]
    private_key: StaticSecret,
    #[serde(
        serialize_with = "serialize_public_key",
        deserialize_with = "deserialize_public_key"
    )]
    public_key: PublicKey,
    mixes: Vec<Vec<NodeID>>,
    providers: HashSet<NodeID>,
    #[serde(
        serialize_with = "serialize_node_public_keys",
        deserialize_with = "deserialize_node_public_keys"
    )]
    node_public_keys: HashMap<NodeID, PublicKey>,
}

impl NetworkStorage {
    pub fn get_mixes(&self) -> &Vec<Vec<NodeID>> {
        &self.mixes
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClientStorage {
    our_provider: Option<NodeID>,
    clients: Vec<NodeInfo>,
    client_to_provider_map: HashMap<NodeID, NodeID>,
}

impl ClientStorage {
    pub fn new(
        our_provider: Option<NodeID>,
        clients: Vec<NodeInfo>,
        client_to_provider_map: HashMap<NodeID, NodeID>,
    ) -> Self {
        ClientStorage {
            clients,
            our_provider,
            client_to_provider_map,
        }
    }

    /// Client IDs are chosen randomly
    pub fn default_with_path_length(our_node_id: NodeID, all_nodes: Vec<NodeInfo>, path_length: usize) -> Self {
        let mut our_provider: Option<NodeID> = None;

        let mut client_to_provider_map: HashMap<NodeID, NodeID> = HashMap::new();
        let mut clients = Vec::new();

        for i in 0..path_length {
            let client = all_nodes.get(i).unwrap().get_id();
            let provider = all_nodes.get(i + path_length).unwrap().get_id();

            if client == our_node_id {
                our_provider = Some(provider);
            } else {
                clients.push(all_nodes.get(i).unwrap().clone());
                client_to_provider_map.insert(client, provider);
            }
        }

        ClientStorage {
            clients,
            our_provider,
            client_to_provider_map,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ProviderStorage {
    subscribed_clients: HashSet<NodeID>,
    #[serde(serialize_with = "serialize_client_messages", deserialize_with = "deserialize_client_messages")]
    client_messages: HashMap<NodeID, Vec<(Delay, Sphinx)>>,
}

impl ProviderStorage {
    pub fn new(subscribed_clients: HashSet<NodeID>, client_messages: HashMap<NodeID, Vec<(Delay, Sphinx)>>) -> Self {
        ProviderStorage {
            subscribed_clients,
            client_messages,
        }
    }

    pub fn default() -> Self {
        ProviderStorage {
            subscribed_clients: HashSet::new(),
            client_messages: HashMap::new(),
        }
    }

    /// given path_length 3: our node either 3, 4, or 5
    pub fn default_with_path_length(clients: HashSet<NodeID>) -> Self {

        ProviderStorage {
            subscribed_clients: clients,
            client_messages: HashMap::new(),
        }
    }
}

pub struct LoopixStorage {
    pub network_storage: Arc<RwLock<NetworkStorage>>,
    pub client_storage: Arc<RwLock<Option<ClientStorage>>>,
    pub provider_storage: Arc<RwLock<Option<ProviderStorage>>>,
}

impl LoopixStorage {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string::<LoopixStorageSave>(&LoopixStorageSave::V1((*self).clone()))
    }

    pub async fn get_random_provider(&self) -> NodeID {
        let storage = self.network_storage.read().await;
        if storage.providers.is_empty() {
            panic!("No providers available");
        }
        storage.providers.iter().choose(&mut rand::thread_rng()).unwrap().clone()
    }

    pub async fn get_our_id(&self) -> NodeID {
        self.network_storage.read().await.node_id.clone()
    }

    pub async fn get_private_key(&self) -> StaticSecret {
        self.network_storage.read().await.private_key.clone()
    }

    pub async fn get_public_key(&self) -> PublicKey {
        self.network_storage.read().await.public_key.clone()
    }

    pub async fn get_mixes(&self) -> Vec<Vec<NodeID>> {
        self.network_storage.read().await.mixes.clone()
    }

    pub async fn set_mixes(&self, new_mixes: Vec<Vec<NodeID>>) {
        self.network_storage.write().await.mixes = new_mixes;
    }

    pub async fn get_providers(&self) -> HashSet<NodeID> {
        self.network_storage.read().await.providers.clone()
    }

    pub async fn set_providers(&self, new_providers: HashSet<NodeID>) {
        self.network_storage.write().await.providers = new_providers;
    }

    pub async fn get_node_public_keys(&self) -> HashMap<NodeID, PublicKey> {
        self.network_storage.read().await.node_public_keys.clone()
    }

    pub async fn set_node_public_keys(&self, new_keys: HashMap<NodeID, PublicKey>) {
        self.network_storage.write().await.node_public_keys = new_keys;
    }

    pub async fn get_our_provider(&self) -> Option<NodeID> {
        if let Some(storage) = self.client_storage.read().await.as_ref() {
            storage.our_provider.clone()
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn set_our_provider(&self, provider: Option<NodeID>) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage.our_provider = provider;
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn get_client_to_provider_map(&self) -> HashMap<NodeID, NodeID> {
        if let Some(storage) = self.client_storage.read().await.as_ref() {
            storage.client_to_provider_map.clone()
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn update_client_provider_mapping(&self, client_id: NodeID, new_provider_id: NodeID) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage
                .client_to_provider_map
                .insert(client_id, new_provider_id);
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn set_client_provider_map(&self, new_map: HashMap<NodeID, NodeID>) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage.client_to_provider_map = new_map;
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn get_clients_in_network(&self) -> Vec<NodeInfo> {
        if let Some(storage) = self.client_storage.read().await.as_ref() {
            storage.clients.clone()
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn set_clients_in_network(&self, clients: Vec<NodeInfo>) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage.clients = clients;
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn add_client_in_network(&self, client: NodeInfo) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage.clients.push(client);
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn remove_client_in_network(&self, client: NodeInfo) {
        if let Some(storage) = &mut *self.client_storage.write().await {
            storage.clients.retain(|c| c.get_id() != client.get_id());
        } else {
            panic!("Client storage not found");
        }
    }

    pub async fn get_subscribed_clients(&self) -> HashSet<NodeID> {
        if let Some(storage) = self.provider_storage.read().await.as_ref() {
            storage.subscribed_clients.clone()
        } else {
            panic!("Provider storage not found");
        }
    }

    pub async fn add_subscribed_client(&self, client_id: NodeID) {
        if let Some(storage) = &mut *self.provider_storage.write().await {
            storage.subscribed_clients.insert(client_id);
        } else {
            panic!("Provider storage not found");
        }
    }

    pub async fn set_subscribed_clients(&self, new_clients: HashSet<NodeID>) {
        if let Some(storage) = &mut *self.provider_storage.write().await {
            storage.subscribed_clients = new_clients;
        } else {
            panic!("Provider storage not found");
        }
    }

    pub async fn get_client_messages(&self, node_id: NodeID) -> Vec<(Delay, Sphinx)> {
        if let Some(storage) = self.provider_storage.read().await.as_ref() {
            storage
                .client_messages
                .get(&node_id)
                .cloned()
                .unwrap_or_default()
        } else {
            panic!("Provider storage not found");
        }
    }

    pub async fn add_client_message(&self, client_id: NodeID, delay: Delay, sphinx: Sphinx) {
        if let Some(storage) = &mut *self.provider_storage.write().await {
            storage
                .client_messages
                .entry(client_id)
                .or_insert(Vec::new())
                .push((delay, sphinx));
        } else {
            panic!("Provider storage not found");
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SerializableLoopixStorage {
    pub network_storage: NetworkStorage,
    pub client_storage: Option<ClientStorage>,
    pub provider_storage: Option<ProviderStorage>,
}

impl Serialize for LoopixStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serde_struct = SerializableLoopixStorage {
            network_storage: self.network_storage.blocking_read().clone(),
            client_storage: self.client_storage.blocking_read().clone(),
            provider_storage: self.provider_storage.blocking_read().clone(),
        };

        serde_struct.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LoopixStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serde_struct = SerializableLoopixStorage::deserialize(deserializer)?;

        Ok(LoopixStorage {
            network_storage: Arc::new(RwLock::new(serde_struct.network_storage)),
            client_storage: Arc::new(RwLock::new(serde_struct.client_storage)),
            provider_storage: Arc::new(RwLock::new(serde_struct.provider_storage)),
        })
    }
}

impl Default for LoopixStorage {
    fn default() -> Self {
        let (public_key, private_key) = Self::generate_key_pair();
        LoopixStorage {
            network_storage: Arc::new(RwLock::new(NetworkStorage {
                node_id: NodeID::rnd(),
                private_key,
                public_key,
                mixes: Vec::new(),
                providers: HashSet::new(),
                node_public_keys: HashMap::new(),
            })),
            client_storage: Arc::new(RwLock::new(Option::None)),
            provider_storage: Arc::new(RwLock::new(Option::None)),
        }
    }
}

impl LoopixStorage {
    pub fn new(
        node_id: NodeID,
        private_key: StaticSecret,
        public_key: PublicKey,
        mixes: Vec<Vec<NodeID>>,
        providers: HashSet<NodeID>,
        node_public_keys: HashMap<NodeID, PublicKey>,
        client_storage: Option<ClientStorage>,
        provider_storage: Option<ProviderStorage>,
    ) -> Self {
        LoopixStorage {
            network_storage: Arc::new(RwLock::new(NetworkStorage {
                node_id,
                private_key,
                public_key,
                mixes,
                providers,
                node_public_keys,
            })),
            client_storage: Arc::new(RwLock::new(client_storage)),
            provider_storage: Arc::new(RwLock::new(provider_storage)),
        }
    }

    pub fn arc_clone(&self) -> Self {
        LoopixStorage {
            network_storage: Arc::clone(&self.network_storage),
            client_storage: Arc::clone(&self.client_storage),
            provider_storage: Arc::clone(&self.provider_storage),
        }
    }

    pub fn new_with_key_generation(
        node_id: NodeID,
        mixes: Vec<Vec<NodeID>>,
        providers: HashSet<NodeID>,
        node_public_keys: HashMap<NodeID, PublicKey>,
        client_storage: Option<ClientStorage>,
        provider_storage: Option<ProviderStorage>,
    ) -> Self {
        let (public_key, private_key) = Self::generate_key_pair();
        LoopixStorage {
            network_storage: Arc::new(RwLock::new(NetworkStorage {
                node_id,
                private_key,
                public_key,
                mixes,
                providers,
                node_public_keys,
            })),
            client_storage: Arc::new(RwLock::new(client_storage)),
            provider_storage: Arc::new(RwLock::new(provider_storage)),
        }
    }

    pub fn generate_key_pair() -> (PublicKey, StaticSecret) {
        let rng = rand::thread_rng();
        let private_key = StaticSecret::random_from_rng(rng);
        let public_key = PublicKey::from(&private_key);
        (public_key, private_key)
    }

    /// default testing storage with a given path length
    ///
    /// given path_length 3:
    /// first 0 to 3 indices of all_nodes are reserved for clients,
    /// next 3 to 6 are providers,
    /// next 6 to 14 are mixes.
    pub fn default_with_path_length(
        node_id: NodeID,
        path_length: usize,
        private_key: StaticSecret,
        public_key: PublicKey,
        client_storage: Option<ClientStorage>,
        provider_storage: Option<ProviderStorage>,
        all_nodes: Vec<NodeInfo>,
    ) -> Self {
        //provider generation
        let provider_infos = all_nodes.iter().skip(path_length).take(path_length).collect::<Vec<&NodeInfo>>();
        
        let mut providers = HashSet::from_iter(provider_infos.iter().map(|node| node.get_id()));
        if providers.contains(&node_id) {
            providers.remove(&node_id);
        }
        
        // mix generation
        let mix_infos = all_nodes.iter().skip(path_length * 2);
        let mut mixes = Vec::new();
        for i in 0..path_length {
            mixes.push(mix_infos.clone().skip(i * path_length).take(path_length).collect::<Vec<&NodeInfo>>().iter().map(|node| node.get_id()).collect::<Vec<NodeID>>());
        }

        for mix_layer in &mut mixes {
            if let Some(index) = mix_layer.iter().position(|&r| r == node_id) {
                mix_layer.remove(index);
            }
        }

        LoopixStorage {
            network_storage: Arc::new(RwLock::new(NetworkStorage {
                node_id: NodeID::from(node_id),
                private_key,
                public_key,
                mixes,
                providers,
                node_public_keys: HashMap::new(),
            })),
            client_storage: Arc::new(RwLock::new(client_storage)),
            provider_storage: Arc::new(RwLock::new(provider_storage)),
        }
    }
}

// region: Derived functions
impl PartialEq for LoopixStorage {
    fn eq(&self, other: &Self) -> bool {
        let self_network_storage = self.network_storage.blocking_read();
        let other_network_storage = other.network_storage.blocking_read();

        let self_client_storage = self.client_storage.blocking_read();
        let other_client_storage = other.client_storage.blocking_read();

        let self_provider_storage = self.provider_storage.blocking_read();
        let other_provider_storage = other.provider_storage.blocking_read();

        self_network_storage.node_id == other_network_storage.node_id
            && self_network_storage.private_key.to_bytes()
                == other_network_storage.private_key.to_bytes()
            && self_network_storage.public_key == other_network_storage.public_key
            && self_network_storage.mixes == other_network_storage.mixes
            && self_network_storage.providers == other_network_storage.providers
            && self_network_storage.node_public_keys == other_network_storage.node_public_keys
            && *self_client_storage == *other_client_storage
            && *self_provider_storage == *other_provider_storage
    }
}

impl std::fmt::Debug for LoopixStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let network_storage = self.network_storage.blocking_read();
        let client_storage = self.client_storage.blocking_read();
        let provider_storage = self.provider_storage.blocking_read();

        f.debug_struct("LoopixStorage")
            .field("node_id", &network_storage.node_id)
            .field("private_key", &"[hidden]")
            .field("public_key", &network_storage.public_key)
            .field("mixes", &network_storage.mixes)
            .field("providers", &network_storage.providers)
            .field("node_public_keys", &network_storage.node_public_keys)
            .field("client_storage", &*client_storage)
            .field("provider_storage", &*provider_storage)
            .finish()
    }
}
// endregion: Derived functions

// region: Serde functions
#[derive(Serialize, Deserialize)]
struct SerializableMessage {
    delay: [u8; 8],
    sphinx: Sphinx, 
}
pub fn serialize_client_messages<S>(messages: &HashMap<NodeID, Vec<(Delay, Sphinx)>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serializable_messages: HashMap<_, Vec<_>> = messages
        .iter()
        .map(|(node_id, messages)| {
            let serialized_messages: Vec<_> = messages
                .iter()
                .map(|(delay, sphinx)| SerializableMessage {
                    delay: delay.to_bytes(),
                    sphinx: sphinx.clone(),
                })
                .collect();
            (node_id.clone(), serialized_messages)
        })
        .collect();

    serializable_messages.serialize(serializer)
}

pub fn deserialize_client_messages<'de, D>(deserializer: D) -> Result<HashMap<NodeID, Vec<(Delay, Sphinx)>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let serializable_messages: HashMap<NodeID, Vec<SerializableMessage>> = HashMap::deserialize(deserializer)?;
    
    let messages: HashMap<_, Vec<_>> = serializable_messages
        .into_iter()
        .map(|(node_id, messages)| {
            let deserialized_messages = messages
                .into_iter()
                .map(|msg| {
                    let delay = Delay::from_bytes(msg.delay);
                    Ok((delay, msg.sphinx))
                })
                .collect::<Result<_, D::Error>>()?;
            Ok((node_id, deserialized_messages))
        })
        .collect::<Result<_, D::Error>>()?;

    Ok(messages)
}

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

pub fn serialize_node_public_keys<S>(
    keys: &HashMap<NodeID, PublicKey>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let keys_bytes: HashMap<_, _> = keys
        .iter()
        .map(|(node_id, key)| (node_id, key.as_bytes()))
        .collect();
    keys_bytes.serialize(serializer)
}

pub fn deserialize_node_public_keys<'de, D>(
    deserializer: D,
) -> Result<HashMap<NodeID, PublicKey>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let keys_bytes: HashMap<NodeID, [u8; 32]> = HashMap::deserialize(deserializer)?;
    let keys = keys_bytes
        .into_iter()
        .map(|(node_id, key_bytes)| (node_id, PublicKey::from(key_bytes)))
        .collect();
    Ok(keys)
}

// endregion: Serde functions

impl Clone for LoopixStorage {
    fn clone(&self) -> Self {
        LoopixStorage {
            network_storage: Arc::new(RwLock::new(self.network_storage.blocking_read().clone())),
            client_storage: Arc::new(RwLock::new(self.client_storage.blocking_read().clone())),
            provider_storage: Arc::new(RwLock::new(self.provider_storage.blocking_read().clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::loopix::testing::LoopixSetup;

    use super::*;

    #[tokio::test]
    async fn test_default_with_path_length() {
        let path_length = 3;
        let client_storage = None;
        let provider_storage = None;

        let (all_nodes, _, _, _) = LoopixSetup::create_nodes_and_keys(path_length);

        let node_id = all_nodes.iter().next().unwrap().get_id();
        let (public_key, private_key) = LoopixStorage::generate_key_pair();

        let storage = LoopixStorage::default_with_path_length(
            node_id,
            path_length,
            private_key,
            public_key,
            client_storage,
            provider_storage,
            all_nodes,
        );

        let network_storage = storage.network_storage.read().await;
        println!("Providers: {:?}", network_storage.providers);
        println!("Mixes: {:?}", network_storage.mixes);
        println!("NodeId: {:?}", network_storage.node_id);
    }

    #[test]
    fn test_client_storage_default_with_path_length() {
        let path_length = 3;

        let (all_nodes, _, _, _) = LoopixSetup::create_nodes_and_keys(path_length);

        let our_node_id = all_nodes[0].get_id();

        let client_storage = ClientStorage::default_with_path_length(our_node_id, all_nodes, path_length);

        println!("Our Provider: {:?}", client_storage.our_provider);
        println!(
            "Client to Provider Map: {:?}",
            client_storage.client_to_provider_map
        );
    }

    #[test]
    fn test_provider_storage_default_with_path_length() {
        let path_length = 3;

        let (all_nodes, _, _, _) = LoopixSetup::create_nodes_and_keys(path_length);

        let clients = all_nodes
            .clone()
            .into_iter()
            .skip(path_length)
            .take(path_length)
            .collect::<Vec<NodeInfo>>();

        let provider_storage = ProviderStorage::default_with_path_length(HashSet::from_iter(clients.iter().map(|node| node.get_id())));

        println!("Clients: {:?}", provider_storage.subscribed_clients);
    }

    #[test]
    fn test_loopix_storage_serde() {
        let path_length = 3;    

        let (all_nodes, _, _, _) = LoopixSetup::create_nodes_and_keys(path_length);

        let node_id = all_nodes.clone().into_iter().next().unwrap().get_id();
        let (public_key, private_key) = LoopixStorage::generate_key_pair();

        let client_storage = Some(ClientStorage::default_with_path_length(
            node_id,
            all_nodes.clone(),
            path_length,
        ));
        let provider_storage = None;

        let original_storage = LoopixStorage::default_with_path_length(
            node_id,
            path_length,
            private_key,
            public_key,
            client_storage,
            provider_storage,
            all_nodes.clone(),
        );

        let serialized = serde_yaml::to_string(&original_storage).expect("Failed to serialize");
        let deserialized: LoopixStorage =
            serde_yaml::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(original_storage, deserialized);
    }

    #[tokio::test]
    async fn test_client_messages_serde() {
        let path_length = 3;

        let (all_nodes, _, _, _) = LoopixSetup::create_nodes_and_keys(path_length);

        let node_id = all_nodes.clone().into_iter().next().unwrap().get_id();
        let (public_key, private_key) = LoopixStorage::generate_key_pair();

        let clients = all_nodes
            .clone()
            .into_iter()
            .skip(path_length)
            .take(path_length)
            .collect::<Vec<NodeInfo>>();

        let provider_storage = ProviderStorage::default_with_path_length(HashSet::from_iter(clients.iter().map(|node| node.get_id())));

        let loopix_storage = LoopixStorage::default_with_path_length(
            node_id,
            path_length,
            private_key,
            public_key,
            None,
            Some(provider_storage),
            all_nodes.clone(),
        );

        loopix_storage.add_client_message(NodeID::from(1), Delay::new_from_nanos(1), Sphinx::default()).await;

        let provider_storage = loopix_storage.provider_storage.read().await;

        let updated_provider_storage = provider_storage.clone().unwrap();

        println!("Provider Storage: {:?}", updated_provider_storage);

        let serialized = serde_yaml::to_string(&updated_provider_storage).expect("Failed to serialize");
        let deserialized: ProviderStorage = serde_yaml::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(updated_provider_storage, deserialized);
    }
}
