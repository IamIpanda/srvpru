// In srvpro, it's a config file.
// And welcome to rust, the world without reflect
// Sure, I can load it by json. 
// But I think it's more safe to write it directly.
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[derive(Copy, Clone, TryFromPrimitive, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum CTOSMessageType {
    Response = 1,
    UpdateDeck = 2,
    HandResult = 3,
    TpResult = 4,
    PlayerInfo = 16,
    CreateGame = 17,
    JoinGame = 18,
    LeaveGame = 19,
    Surrender = 20,
    TimeConfirm = 21,
    Chat = 22,
    HsTodueList = 32,
    HsToOBServer = 33,
    HsReady = 34,
    HsNotReady = 35,
    HsKick = 36,
    HsStart = 37,
    RequestField = 48
}

#[derive(Copy, Clone, TryFromPrimitive, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum STOCMessageType {
    GameMessage = 1,
    ErrorMessage = 2,
    SelectHand = 3,
    SelectTp = 4,
    HandResult = 5,
    TpResult = 6,
    ChangeSide = 7,
    WaitingSide = 8,
    DeckCount = 9,
    CreateGame = 17,
    JoinGame = 18,
    TypeChange = 19,
    LeaveGame = 20,
    DuelStart = 21,
    DuelEnd = 22,
    Replay = 23,
    TimeLimit = 24,
    Chat = 25,
    HsPlayerEnter = 32,
    HsPlayerChange = 33,
    HsWatchChange = 34,
    FieldFinish = 48
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    STOC,
    CTOS,
    SRVPRU,
}

pub fn get_message_type(direction: Direction, type_number: u8) -> Option<MessageType> {
    match direction {
        Direction::CTOS => CTOSMessageType::try_from(type_number).ok().map(|s| MessageType::CTOS(s)),
        Direction::STOC => STOCMessageType::try_from(type_number).ok().map(|s| MessageType::STOC(s)),
        Direction::SRVPRU => None
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MessageType {
    STOC(STOCMessageType),
    CTOS(CTOSMessageType),
}