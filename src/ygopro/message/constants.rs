use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::gm;


// ====================================================================================================
//  Direction
// ----------------------------------------------------------------------------------------------------
/// Point out where the message from and to.
// ====================================================================================================
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Message is from server to client.
    STOC,
    /// Message is from client to server.
    CTOS,
    /// Srvpru internal message.
    SRVPRU,
}

// ====================================================================================================
//  MessageType
// ----------------------------------------------------------------------------------------------------
/// Point out what type of message is, divided by a Direction.
// ====================================================================================================
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum MessageType {
    /// Message is a type from server to client.
    STOC(stoc::MessageType),
    /// An extra type for processing game_message inner [Processor](crate::srvpru::Processor). 
    GM(gm::MessageType),
    /// Message is a type from client to server.
    CTOS(ctos::MessageType),
    /// Message is a type inner srvpru.
    SRVPRU(crate::srvpru::message::MessageType),
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::STOC(message_type) => write!(f, "STOC {:?}", message_type)?,
            MessageType::GM(message_type) => write!(f, "GM {:?}", message_type)?,
            MessageType::CTOS(message_type) => write!(f, "CTOS {:?}", message_type)?,
            MessageType::SRVPRU(message_type) => write!(f, "SRVPRU {:?}", message_type)?,
        }
        Ok(())
    }
}
