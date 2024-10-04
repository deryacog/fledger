use flarch::broker::{Broker, BrokerError};
use crate::ping::messages::{PingMessage, PingIn, MessageNode};
use flarch::nodeids::NodeID;
use super::messages::ModuleMessage;

pub struct LoopixBroker {
    broker: Broker<ModuleMessage>,
}

impl LoopixBroker {
    pub fn new() -> Self {
        Self {
            broker: Broker<ModuleMessage>,
        }
    }

    pub fn send_ping_from_loopix(&self, id: NodeID) -> Result<(), BrokerError> {
        let ping_in = PingIn::Message(id, MessageNode::Ping);
        let ping_message = PingMessage::from(ping_in);
        let module_message = ModuleMessage::Ping(ping_message);
        self.broker.emit_msg(module_message)
    }
}
