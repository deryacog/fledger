use ed25519_dalek::Signer;

use log::{info, warn};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

use self::{connection_state::CSEnum, node_connection::NCOutput};
use crate::signal::{web_rtc::{
        ConnectionStateMap, MessageAnnounce, NodeStat, PeerInfo, WSSignalMessage, WebRTCSpawner,
        WebSocketMessage,
    }, websocket::{WSError, WSMessage, WebSocketConnection}};
use crate::{
    node::config::NodeConfig,
    node::config::NodeInfo,
    signal::web_rtc::WebRTCConnectionState,
    types::{ProcessCallback, U256},
};
use node_connection::{NCError, NCInput, NodeConnection};
use WSSignalMessage::NodeStats;

pub mod connection_state;
pub mod node_connection;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Couldn't put in output queue")]
    OutputQueue,
    #[error("Couldn't read from input queue")]
    InputQueue,
    #[error("Got alien PeerSetup")]
    AlienPeerSetup,
    #[error(transparent)]
    WebSocket(#[from] WSError),
    #[error(transparent)]
    SerdeJSON(#[from] serde_json::Error),
    #[error(transparent)]
    NodeConnection(#[from] NCError),
}

pub enum NOutput {
    WebRTC(U256, String),
    UpdateList(Vec<NodeInfo>),
    State(
        U256,
        WebRTCConnectionState,
        CSEnum,
        Option<ConnectionStateMap>,
    ),
}

pub enum NInput {
    WebRTC(U256, String),
    SendStats(Vec<NodeStat>),
    ClearNodes,
    UpdateList,
}

pub struct Network {
    pub output_rx: Receiver<NOutput>,
    pub input_tx: Sender<NInput>,
    output_tx: Sender<NOutput>,
    input_rx: Receiver<NInput>,
    list: Vec<NodeInfo>,
    ws: Box<dyn WebSocketConnection>,
    ws_rx: Receiver<WSMessage>,
    web_rtc: Arc<Mutex<WebRTCSpawner>>,
    connections: HashMap<U256, NodeConnection>,
    node_config: NodeConfig,
    process: ProcessCallback,
}

/// Network combines a websocket to connect to the signal server with
/// a WebRTC trait to connect to other nodes.
/// It supports setting up automatic connections to other nodes.
impl Network {
    pub fn new(
        node_config: NodeConfig,
        mut ws: Box<dyn WebSocketConnection>,
        web_rtc: WebRTCSpawner,
        process: ProcessCallback,
    ) -> Network {
        let (output_tx, output_rx) = channel::<NOutput>();
        let (input_tx, input_rx) = channel::<NInput>();
        let (ws_tx, ws_rx) = channel::<WSMessage>();
        let process_clone = process.clone();
        ws.set_cb_wsmessage(Box::new(move |msg| {
            if let Err(e) = ws_tx.send(msg) {
                info!("Couldn't send msg over ws-channel: {}", e);
            }
            match process_clone.try_lock() {
                Ok(mut p) => p(),
                Err(_e) => warn!("network::process was locked"),
            }
        }));
        let net = Network {
            list: vec![],
            output_tx,
            output_rx,
            input_tx,
            input_rx,
            ws,
            ws_rx,
            web_rtc: Arc::new(Mutex::new(web_rtc)),
            connections: HashMap::new(),
            node_config,
            process,
        };
        net
    }

    /// Process all connections with their waiting messages.
    pub async fn process(&mut self) -> Result<usize, NetworkError> {
        Ok(self.process_input().await?
            + self.process_websocket().await?
            + self.process_connections().await?)
    }

    async fn process_input(&mut self) -> Result<usize, NetworkError> {
        let msgs: Vec<NInput> = self.input_rx.try_iter().collect();
        let size = msgs.len();
        for msg in msgs {
            match msg {
                NInput::SendStats(s) => self.ws_send(NodeStats(s))?,
                NInput::WebRTC(id, msg) => self.send(&id, msg).await?,
                NInput::ClearNodes => self.ws_send(WSSignalMessage::ClearNodes)?,
                NInput::UpdateList => self.ws_send(WSSignalMessage::ListIDsRequest)?,
            }
        }
        Ok(size)
    }

    async fn process_websocket(&mut self) -> Result<usize, NetworkError> {
        let msgs: Vec<WSMessage> = self.ws_rx.try_iter().collect();
        for msg in &msgs {
            match msg {
                WSMessage::MessageString(s) => {
                    self.process_msg(WebSocketMessage::from_str(&s)?.msg)
                        .await?;
                }
                _ => {}
            }
        }
        Ok(msgs.len())
    }

    async fn process_connections(&mut self) -> Result<usize, NetworkError> {
        let mut ws_msgs = vec![];
        let mut msgs = 0;
        let conns: Vec<(&U256, &mut NodeConnection)> = self.connections.iter_mut().collect();
        for conn in conns {
            let outputs: Vec<NCOutput> = conn.1.output_rx.try_iter().collect();
            msgs += outputs.len();
            for output in outputs {
                match output {
                    NCOutput::WebSocket(message, remote) => {
                        let (id_init, id_follow) = match remote {
                            true => (conn.0.clone(), self.node_config.our_node.get_id()),
                            false => (self.node_config.our_node.get_id(), conn.0.clone()),
                        };
                        let peer_info = PeerInfo {
                            id_init,
                            id_follow,
                            message,
                        };
                        ws_msgs.push(WSSignalMessage::PeerSetup(peer_info));
                    }
                    NCOutput::WebRTCMessage(msg) => self
                        .output_tx
                        .send(NOutput::WebRTC(conn.0.clone(), msg))
                        .map_err(|_| NetworkError::OutputQueue)?,
                    NCOutput::State(dir, c, sta) => self
                        .output_tx
                        .send(NOutput::State(conn.0.clone(), dir, c, sta))
                        .map_err(|_| NetworkError::OutputQueue)?,
                }
            }
            conn.1.process().await?;
        }
        for msg in ws_msgs {
            self.ws_send(msg)?;
        }
        Ok(msgs)
    }

    /// Processes incoming messages from the signalling server.
    /// This can be either messages requested by this node, or connection
    /// setup requests from another node.
    async fn process_msg(&mut self, msg: WSSignalMessage) -> Result<(), NetworkError> {
        match msg {
            WSSignalMessage::Challenge(version, challenge) => {
                info!("Processing Challenge message version: {}", version);
                let ma = MessageAnnounce {
                    version,
                    challenge,
                    node_info: self.node_config.our_node.clone(),
                    signature: self.node_config.keypair.sign(&challenge.to_bytes()),
                };
                self.ws.send(
                    WebSocketMessage {
                        msg: WSSignalMessage::Announce(ma),
                    }
                    .to_string(),
                )?;
                self.input_tx
                    .send(NInput::UpdateList)
                    .map_err(|_| NetworkError::InputQueue)?;
            }
            WSSignalMessage::ListIDsReply(list) => {
                self.update_list(list)?;
            }
            WSSignalMessage::PeerSetup(pi) => {
                let remote_node = match pi.get_remote(&self.node_config.our_node.get_id()) {
                    Some(id) => id,
                    None => {
                        return Err(NetworkError::AlienPeerSetup);
                    }
                };
                let remote = remote_node == pi.id_init;
                let conn = self
                    .connections
                    .entry(remote_node)
                    .or_insert(NodeConnection::new(
                        Arc::clone(&self.web_rtc),
                        self.process.clone(),
                    )?);
                conn.input_tx
                    .send(NCInput::WebSocket(pi.message, remote))
                    .map_err(|_| NetworkError::InputQueue)?;
            }
            ws => {
                info!("Got unusable message: {:?}", ws);
            }
        }
        Ok(())
    }

    /// Stores a node list sent from the signalling server.
    fn update_list(&mut self, list: Vec<NodeInfo>) -> Result<(), NetworkError> {
        self.list = list
            .iter()
            .filter(|entry| entry.get_id() != self.node_config.our_node.get_id())
            .cloned()
            .collect();
        self.output_tx
            .send(NOutput::UpdateList(list))
            .map_err(|_| NetworkError::OutputQueue)
    }

    fn ws_send(&mut self, msg: WSSignalMessage) -> Result<(), NetworkError> {
        self.ws.send(WebSocketMessage { msg }.to_string())?;
        Ok(())
    }

    /// Sends a message to the node dst.
    /// If no connection is active yet, a new one will be created.
    /// NodeConnection will take care of putting the message in a queue while
    /// the setup is finishing.
    async fn send(&mut self, dst: &U256, msg: String) -> Result<(), NetworkError> {
        let conn = self
            .connections
            .entry(dst.clone())
            .or_insert(NodeConnection::new(
                Arc::clone(&self.web_rtc),
                self.process.clone(),
            )?);
        conn.send(msg.clone()).await?;
        Ok(())
    }

    pub fn get_list(&self) -> Vec<NodeInfo> {
        self.list.clone()
    }
}
