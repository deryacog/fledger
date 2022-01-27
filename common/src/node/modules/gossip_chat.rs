use super::text_messages_v1::TextMessagesStorage;
use crate::broker::{Subsystem, SubsystemListener};
use crate::node::modules::messages::{Message, MessageV1};
use crate::node::timer::BrokerTimer;
use crate::node::NodeData;
use crate::node::{modules::messages::NodeMessage, network::BrokerNetwork};
use crate::node::{BrokerMessage, BrokerModules};
use std::sync::Arc;
use std::sync::Mutex;

pub use raw::gossip_chat::{MessageIn, MessageNode, MessageOut};

use super::random_connections::RandomMessage;

#[derive(Debug, Clone, PartialEq)]
pub enum GossipMessage {
    MessageIn(MessageIn),
    MessageOut(MessageOut),
}

impl From<GossipMessage> for BrokerModules {
    fn from(msg: GossipMessage) -> Self {
        Self::Gossip(msg)
    }
}

impl From<MessageIn> for GossipMessage {
    fn from(msg: MessageIn) -> Self {
        Self::MessageIn(msg)
    }
}

impl From<MessageOut> for GossipMessage {
    fn from(msg: MessageOut) -> Self {
        Self::MessageOut(msg)
    }
}

/// This is a wrapper around the raw::gossip_chat module. It parses the
/// BrokerMessages for messages of other nodes and for a new NodeList sent by the
/// random_connections module.
pub struct GossipChat {
    node_data: Arc<Mutex<NodeData>>,
}

const STORAGE_GOSSIP_CHAT: &str = "gossip_chat";

impl GossipChat {
    pub fn start(node_data: Arc<Mutex<NodeData>>) {
        {
            let mut nd = node_data.lock().unwrap();

            let gossip_msgs_str = nd
                .storage
                .get(STORAGE_GOSSIP_CHAT)
                .get(STORAGE_GOSSIP_CHAT)
                .unwrap();
            if !gossip_msgs_str.is_empty() {
                if let Err(e) = nd.gossip_chat.set(&gossip_msgs_str) {
                    log::warn!("Couldn't load gossip messages: {}", e);
                }
            } else {
                log::info!("Migrating from old TextMessageStorage to new one.");
                let mut messages = TextMessagesStorage::new();
                if let Err(e) =
                    messages.load(&nd.storage.get("something").get("something").unwrap())
                {
                    log::warn!("Error while loading messages: {}", e);
                } else {
                    let msgs = messages
                        .storage
                        .values()
                        .map(|msg| raw::gossip_chat::text_message::TextMessage {
                            src: msg.src,
                            created: msg.created,
                            msg: msg.msg.clone(),
                        })
                        .collect();
                    nd.gossip_chat.add_messages(msgs);
                }
            }
            nd.broker.clone()
        }
        .add_subsystem(Subsystem::Handler(Box::new(Self { node_data })))
        .unwrap();
    }

    // Searches for a matching NodeMessageIn or a RandomMessage that needs conversion.
    fn process_msg_bm(&self, msg: &BrokerMessage) -> Vec<BrokerMessage> {
        match msg {
            BrokerMessage::Network(BrokerNetwork::NodeMessageIn(nm)) => match &nm.msg {
                Message::V1(MessageV1::GossipChat(gc)) => Some(MessageIn::Node(nm.id, gc.clone())),
                _ => None,
            },
            BrokerMessage::Modules(BrokerModules::Random(RandomMessage::MessageOut(msg_rnd))) => {
                msg_rnd.clone().into()
            }
            BrokerMessage::Timer(BrokerTimer::Minute) => Some(MessageIn::Tick),
            _ => None,
        }
        .map(|msg| self.process_msg_in(&msg))
        .unwrap_or_default()
    }

    fn process_msg_in(&self, msg: &MessageIn) -> Vec<BrokerMessage> {
        if let Ok(mut nd) = self.node_data.try_lock() {
            if let Ok(msgs) = nd.gossip_chat.process_message(msg.clone()) {
                return msgs
                    .iter()
                    .map(|msg| match msg {
                        MessageOut::Node(id, nm) => NodeMessage {
                            id: *id,
                            msg: nm.clone().into(),
                        }
                        .output(),
                        _ => msg.clone().into(),
                    })
                    .collect();
            }
        } else {
            log::error!("Couldn't lock");
        }
        vec![]
    }
}

impl SubsystemListener for GossipChat {
    fn messages(&mut self, msgs: Vec<&BrokerMessage>) -> Vec<BrokerMessage> {
        msgs.iter()
            .flat_map(|msg| {
                if let BrokerMessage::Modules(BrokerModules::Gossip(GossipMessage::MessageIn(
                    msg_in,
                ))) = msg
                {
                    self.process_msg_in(msg_in)
                } else {
                    self.process_msg_bm(msg)
                }
            })
            .collect()
    }
}
