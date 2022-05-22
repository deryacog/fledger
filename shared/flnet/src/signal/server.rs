use std::{
    collections::HashMap,
    fmt::{Error, Formatter},
};

use async_trait::async_trait;
use bimap::BiMap;
use flmodules::{
    broker::{Broker, BrokerError, Destination, Subsystem, SubsystemListener},
    nodeids::U256,
};

use crate::{
    config::NodeInfo,
    signal::{web_rtc::WSSignalMessageToNode, websocket::WSServerInput},
};

use super::{
    web_rtc::{MessageAnnounce, NodeStat, PeerInfo, WSSignalMessageFromNode},
    websocket::{WSServerMessage, WSServerOutput},
};

/// This implements a signalling server. It can be used for tests, in the cli implementation, and
/// will also be used later directly in the network struct to allow for direct node-node setups.
/// It handles the setup phase where the nodes authenticate themselves to the server, and passes
/// PeerInfo messages between nodes.
/// It also handles statistics by forwarding NodeStats to a listener.

#[derive(Clone, Debug)]
pub enum SignalMessage {
    Input(SignalInput),
    Output(SignalOutput),
    WSServer(WSServerMessage),
}

#[derive(Clone)]
pub enum SignalInput {
    WebSocket((U256, WSSignalMessageToNode)),
    Timer,
}

#[derive(Clone, Debug)]
pub enum SignalOutput {
    WebSocket((U256, WSSignalMessageFromNode)),
    NodeStats(Vec<NodeStat>),
    NewNode(U256),
}

pub struct SignalServer {
    connection_ids: BiMap<U256, usize>,
    info: HashMap<U256, NodeInfo>,
    ttl: HashMap<usize, u64>,
    ttl_init: u64,
}

impl SignalServer {
    /// Creates a new SignalServer.
    pub async fn new(
        ws_server: Broker<WSServerMessage>,
        ttl_init: u64,
    ) -> Result<Broker<SignalMessage>, BrokerError> {
        let mut broker = Broker::new();
        broker
            .add_subsystem(Subsystem::Handler(Box::new(SignalServer {
                connection_ids: BiMap::new(),
                info: HashMap::new(),
                ttl: HashMap::new(),
                ttl_init,
            })))
            .await?;
        broker
            .link_bi(
                ws_server,
                Box::new(Self::link_wss_ss),
                Box::new(Self::link_ss_wss),
            )
            .await;
        Ok(broker)
    }

    fn link_wss_ss(msg: WSServerMessage) -> Option<SignalMessage> {
        matches!(msg, WSServerMessage::Output(_)).then(|| SignalMessage::WSServer(msg))
    }

    fn link_ss_wss(msg: SignalMessage) -> Option<WSServerMessage> {
        if let SignalMessage::WSServer(msg_wss) = msg {
            matches!(msg_wss, WSServerMessage::Input(_)).then(|| msg_wss)
        } else {
            None
        }
    }

    fn msg_in(&mut self, msg_in: SignalInput) -> Vec<SignalMessage> {
        match msg_in {
            SignalInput::WebSocket((dst, msg)) => {
                if let Some(index) = self.connection_ids.get_by_left(&dst) {
                    return self.send_msg_node(*index, msg.clone());
                }
            }
            SignalInput::Timer => self.msg_in_timer(),
        }
        vec![]
    }

    fn msg_wss(&mut self, msg: WSServerOutput) -> Vec<SignalMessage> {
        match msg {
            WSServerOutput::Message((index, msg_s)) => {
                self.ttl
                    .entry(index.clone())
                    .and_modify(|ttl| *ttl = self.ttl_init);
                if let Ok(msg_ws) = serde_json::from_str::<WSSignalMessageFromNode>(&msg_s) {
                    return self.msg_ws_process(index, msg_ws);
                }
            }
            WSServerOutput::Connect(index) => return self.msg_ws_connect(index),
            WSServerOutput::Disconnect(id) => self.remove_node(id),
            WSServerOutput::Stopped => todo!(),
        }
        vec![]
    }

    fn msg_in_timer(&mut self) {
        let mut to_remove = Vec::new();
        for (index, ttl) in self.ttl.iter_mut() {
            *ttl -= 1;
            if *ttl == 0 {
                to_remove.push(*index);
            }
        }
        for id in to_remove {
            self.remove_node(id);
        }
    }

    // The id is the challange until the announcement succeeds. Then ws_announce calls
    // set_cb_message again to create a new callback using the node-id as id.
    fn msg_ws_process(&mut self, index: usize, msg: WSSignalMessageFromNode) -> Vec<SignalMessage> {
        match msg {
            WSSignalMessageFromNode::Announce(ann) => self.ws_announce(index, ann),
            WSSignalMessageFromNode::ListIDsRequest => self.ws_list_ids(index),
            WSSignalMessageFromNode::ClearNodes => self.ws_clear(),
            WSSignalMessageFromNode::PeerSetup(pi) => self.ws_peer_setup(index, pi),
            WSSignalMessageFromNode::NodeStats(ns) => self.ws_node_stats(ns),
        }
    }

    fn msg_ws_connect(&mut self, index: usize) -> Vec<SignalMessage> {
        log::debug!("Sending challenge to new connetion");
        let challenge = U256::rnd();
        self.connection_ids.insert(challenge, index);
        self.ttl.insert(index, self.ttl_init);
        let challenge_msg =
            serde_json::to_string(&WSSignalMessageToNode::Challenge(2u64, challenge)).unwrap();
        vec![WSServerInput::Message((index, challenge_msg)).into()]
    }

    fn ws_announce(&mut self, index: usize, msg: MessageAnnounce) -> Vec<SignalMessage> {
        let challenge = match self.connection_ids.get_by_right(&index) {
            Some(id) => id,
            None => {
                log::warn!("Got an announcement message without challenge.");
                return vec![];
            }
        };
        if !msg.node_info.verify(&challenge.to_bytes(), &msg.signature) {
            log::warn!("Got node with wrong signature");
            return vec![];
        }
        let id = msg.node_info.get_id();
        self.connection_ids.insert(id, index);

        log::info!("Registration of node-id {}: {}", id, msg.node_info.info);
        self.info.insert(id, msg.node_info);
        vec![SignalOutput::NewNode(id).into()]
    }

    fn ws_list_ids(&mut self, id: usize) -> Vec<SignalMessage> {
        self.send_msg_node(
            id,
            WSSignalMessageToNode::ListIDsReply(self.info.values().cloned().collect()),
        )
    }

    fn ws_clear(&mut self) -> Vec<SignalMessage> {
        self.ttl.clear();
        self.connection_ids.clear();
        self.info.clear();
        vec![]
    }

    fn ws_peer_setup(&mut self, index: usize, pi: PeerInfo) -> Vec<SignalMessage> {
        let id = match self.connection_ids.get_by_right(&index) {
            Some(id) => id,
            None => {
                log::warn!("Got a peer-setup message without challenge.");
                return vec![];
            }
        };
        log::trace!("Node {} sent peer setup: {:?}", id, pi);
        if let Some(dst) = pi.get_remote(id) {
            if let Some(dst_index) = self.connection_ids.get_by_left(&dst) {
                return self.send_msg_node(*dst_index, WSSignalMessageToNode::PeerSetup(pi));
            }
        }
        vec![]
    }

    fn ws_node_stats(&mut self, ns: Vec<NodeStat>) -> Vec<SignalMessage> {
        vec![SignalOutput::NodeStats(ns).into()]
    }

    fn send_msg_node(&self, index: usize, msg: WSSignalMessageToNode) -> Vec<SignalMessage> {
        vec![WSServerInput::Message((index, serde_json::to_string(&msg).unwrap())).into()]
    }

    fn remove_node(&mut self, index: usize) {
        if let Some((id, _)) = self.connection_ids.remove_by_right(&index) {
            self.info.remove(&id);
        }
        self.ttl.remove(&index);
    }
}

#[cfg_attr(feature = "nosend", async_trait(?Send))]
#[cfg_attr(not(feature = "nosend"), async_trait)]
impl SubsystemListener<SignalMessage> for SignalServer {
    async fn messages(
        &mut self,
        from_broker: Vec<SignalMessage>,
    ) -> Vec<(Destination, SignalMessage)> {
        let mut out = vec![];
        for msg in from_broker {
            match msg {
                SignalMessage::Input(msg_in) => out.extend(self.msg_in(msg_in)),
                SignalMessage::WSServer(WSServerMessage::Output(msg_wss)) => {
                    out.extend(self.msg_wss(msg_wss))
                }
                _ => {}
            }
        }
        out.into_iter()
            .map(|msg| (Destination::Others, msg))
            .collect()
    }
}

impl std::fmt::Debug for SignalInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            SignalInput::WebSocket(_) => write!(f, "WebSocket"),
            SignalInput::Timer => write!(f, "Timer"),
        }
    }
}

impl From<WSServerMessage> for SignalMessage {
    fn from(msg: WSServerMessage) -> Self {
        SignalMessage::WSServer(msg)
    }
}

impl From<WSServerInput> for SignalMessage {
    fn from(msg: WSServerInput) -> Self {
        SignalMessage::WSServer(msg.into())
    }
}

impl From<WSServerOutput> for SignalMessage {
    fn from(msg: WSServerOutput) -> Self {
        SignalMessage::WSServer(msg.into())
    }
}

impl From<SignalInput> for SignalMessage {
    fn from(msg: SignalInput) -> Self {
        SignalMessage::Input(msg)
    }
}

impl From<SignalOutput> for SignalMessage {
    fn from(msg: SignalOutput) -> Self {
        SignalMessage::Output(msg)
    }
}
