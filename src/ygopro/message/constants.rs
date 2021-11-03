use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::gm;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    STOC,
    CTOS,
    SRVPRU,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum MessageType {
    STOC(stoc::MessageType),
    GM(gm::MessageType),
    CTOS(ctos::MessageType),
    SRVPRU(crate::srvpru::message::MessageType),
}