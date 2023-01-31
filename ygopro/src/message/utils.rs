use std::fmt::Debug;
use std::marker::PhantomData;

use serde::Serialize;
use serde::Deserialize;
use serde::ser::SerializeSeq;
use srvpru_proc_macros::serde_default;

use crate::constants::Mode;
use crate::serde::LengthDescribed;
use crate::serde::Error;
use crate::serde::de::deserialize;

use super::server_to_client;
use super::client_to_server;
use super::game_message;

pub trait PureMessage: erased_serde::Serialize + LengthDescribed + 'static {}

pub trait Message: PureMessage + Debug {
    fn message_type() -> MessageType where Self: Sized;
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum MessageType {
    STOC(server_to_client::MessageType),
    CTOS(client_to_server::MessageType),
    GM(game_message::MessageType),
    Other(&'static str, u8)
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum ExtendedMessageType {
    STOC(server_to_client::ExtendedMessageType),
    CTOS(client_to_server::ExtendedMessageType),
    GM(game_message::ExtendedMessageType),
    Other(&'static str, u8)
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::STOC(message_type) => message_type.into(),
            MessageType::CTOS(message_type) => message_type.into(),
            MessageType::GM(_) => 1,
            MessageType::Other(_, order) => order,
        }    
    }
}

impl From<ExtendedMessageType> for u8 {
    fn from(value: ExtendedMessageType) -> Self {
        match value {
            ExtendedMessageType::STOC(message_type) => message_type.into(),
            ExtendedMessageType::CTOS(message_type) => message_type.into(),
            ExtendedMessageType::GM(_) => 1,
            ExtendedMessageType::Other(_, order) => order,
        }
    }
}

impl From<MessageType> for ExtendedMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::STOC(message_type) => ExtendedMessageType::STOC(message_type.into()),
            MessageType::CTOS(message_type) => ExtendedMessageType::CTOS(message_type.into()),
            MessageType::GM(message_type) => ExtendedMessageType::GM(message_type.into()),
            MessageType::Other(_type, id) => ExtendedMessageType::Other(_type, id),
        }
    }
}

impl From<server_to_client::MessageType> for MessageType {
    fn from(value: server_to_client::MessageType) -> Self {
        MessageType::STOC(value)
    }
}

impl From<client_to_server::MessageType> for MessageType {
    fn from(value: client_to_server::MessageType) -> Self {
        MessageType::CTOS(value)
    }
}

impl From<game_message::MessageType> for MessageType {
    fn from(value: game_message::MessageType) -> Self {
        MessageType::GM(value)
    }
}

impl From<server_to_client::ExtendedMessageType> for ExtendedMessageType {
    fn from(value: server_to_client::ExtendedMessageType) -> Self {
        ExtendedMessageType::STOC(value)
    }
}

impl From<client_to_server::ExtendedMessageType> for ExtendedMessageType {
    fn from(value: client_to_server::ExtendedMessageType) -> Self {
        ExtendedMessageType::CTOS(value)
    }
}

impl From<game_message::ExtendedMessageType> for ExtendedMessageType {
    fn from(value: game_message::ExtendedMessageType) -> Self {
        ExtendedMessageType::GM(value)
    }
}

impl Into<u16> for MessageType {
    fn into(self) -> u16 { Into::<u8>::into(self) as u16 }
}

impl Into<u32> for MessageType {
    fn into(self) -> u32 { Into::<u8>::into(self) as u32 }
}

impl Into<u16> for ExtendedMessageType {
    fn into(self) -> u16 { Into::<u8>::into(self) as u16 }
}

impl Into<u32> for ExtendedMessageType {
    fn into(self) -> u32 { Into::<u8>::into(self) as u32 }
}

impl serde::Serialize for MessageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_u8((*self).into())
    }
}

impl serde::Serialize for ExtendedMessageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_u8((*self).into())
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::STOC(message_type) => std::fmt::Display::fmt(&message_type, f),
            MessageType::CTOS(message_type) => std::fmt::Display::fmt(&message_type, f),
            MessageType::GM(message_type) => std::fmt::Display::fmt(&message_type, f),
            MessageType::Other(meta, number) => write!(f, "{}: {}", meta, number),
        }
    }
}

impl std::fmt::Display for ExtendedMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtendedMessageType::STOC(message_type) => std::fmt::Display::fmt(&message_type, f),
            ExtendedMessageType::CTOS(message_type) => std::fmt::Display::fmt(&message_type, f),
            ExtendedMessageType::GM(message_type) => std::fmt::Display::fmt(&message_type, f),
            ExtendedMessageType::Other(meta, number) => write!(f, "{}: {}", meta, number),
        }
    }
}

#[derive(Debug)]
pub enum MessageEnum {
    STOC(server_to_client::MessageEnum),
    CTOS(client_to_server::MessageEnum),
    Other((&'static str, Box<dyn std::any::Any + Send + Sync>))
}

impl LengthDescribed for MessageEnum {
    fn sizeof(&self) -> usize {
        match self {
            MessageEnum::STOC(message) => message.sizeof(),
            MessageEnum::CTOS(message) => message.sizeof(),
            MessageEnum::Other(_) => todo!(),
        }
    }
}

impl serde::Serialize for MessageEnum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            MessageEnum::STOC(message) => server_to_client::MessageEnum::serialize(message, serializer),
            MessageEnum::CTOS(message) => client_to_server::MessageEnum::serialize(message, serializer),
            MessageEnum::Other(_) => todo!(),
        }
    }
}

impl PureMessage for MessageEnum {}

#[derive(Debug)]
pub struct UndeserializedBytes<'bytes, T> {
    pub message_type: T,
    pub bytes: &'bytes [u8] 
}
// unsafe impl<'bytes, T> Send for UndeserializedBytes<'bytes, T> where T: Send {}

impl<'bytes, T> UndeserializedBytes<'bytes, T> where T: Copy + Into<MessageType> {
    fn produce<'des, M: Message + Deserialize<'des>>(&self) -> Result<M, Error> where 'bytes: 'des {
        if M::message_type() != self.message_type.into() {
            return Err(Error::WrongType)
        }
        Ok(deserialize(self.bytes)?)
    }
}

impl<'bytes, T> LengthDescribed for UndeserializedBytes<'bytes, T> {
    fn sizeof(&self) -> usize {
        return self.bytes.len() + 1;
    }
}

impl<'bytes> std::convert::TryFrom<UndeserializedBytes<'bytes, MessageType>> for MessageEnum {
    type Error = crate::serde::Error;

    fn try_from(value: UndeserializedBytes<'bytes, MessageType>) -> Result<Self, Self::Error> {
        Ok(match value.message_type {
            MessageType::STOC(message_type) => MessageEnum::STOC(server_to_client::MessageEnum::try_from(UndeserializedBytes { message_type, bytes: value.bytes })?),
            MessageType::CTOS(message_type) => MessageEnum::CTOS(client_to_server::MessageEnum::try_from(UndeserializedBytes { message_type, bytes: value.bytes })?),
            MessageType::GM(_) => todo!(),
            MessageType::Other(_, _) => todo!(),
        })
    }
}

impl<'bytes, T> serde::Serialize for UndeserializedBytes<'bytes, T> where T: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.message_type)?;
        seq.serialize_element(&self.bytes)?;
        seq.end()
    }
}

impl<'de, 'bytes, T> serde::Deserialize<'de> for UndeserializedBytes<'bytes, T> 
    where T: Deserialize<'de> + 'static + std::convert::TryFrom<u8> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_bytes(UndeserializedBytesVisitor::new())
    }
}

#[derive(Debug)]
struct UndeserializedBytesVisitor<'bytes, T> { 
    phantom: PhantomData<&'bytes T>
}

impl<'bytes, T> UndeserializedBytesVisitor<'bytes, T> {
    fn new() -> Self {
        Self {
            phantom: PhantomData
        }
    }
}

impl<'de, 'bytes, T> serde::de::Visitor<'de> for UndeserializedBytesVisitor<'bytes, T> 
    where T: Deserialize<'de> + std::convert::TryFrom<u8> {
    type Value = UndeserializedBytes<'bytes, T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Undeserialized bytes")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error, {
        Ok(UndeserializedBytes {
            message_type: T::try_from(v[0]).map_err(|_| E::custom("Unknown Message type"))?,
            bytes: unsafe { std::slice::from_raw_parts(v[1..].as_ptr(), v.len() - 1) },
        })
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> where E: serde::de::Error, {
        self.visit_bytes(&v)
    }
}

trait _Message: Message + erased_serde::Serialize{
    fn dyn_message_type(&self) -> MessageType;
}

impl<M: Message> _Message for M {
    fn dyn_message_type(&self) -> MessageType {
        Self::message_type()
    }
}

#[derive(Debug)]
pub struct DynamicTypeMessage(Box<dyn _Message>);

impl DynamicTypeMessage {
    fn new<M: Message + Sized + 'static>(message: M) -> Self {
        Self(Box::new(message) as Box<dyn _Message>)
    }
}

impl LengthDescribed for DynamicTypeMessage {
    fn sizeof(&self) -> usize where Self: Sized {
        self.0.sizeof() + 1
    }
}

erased_serde::serialize_trait_object!(_Message);
erased_serde::serialize_trait_object!(PureMessage);
erased_serde::serialize_trait_object!(Message);

impl Serialize for DynamicTypeMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0.dyn_message_type())?;
        seq.serialize_element(&self.0)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DynamicTypeMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        todo!()
    }
}

pub enum FullMessage<'bytes, Enum, Type> {
    Raw(&'bytes [u8]),
    Unknown(&'bytes [u8]),
    Undeserialized(UndeserializedBytes<'bytes, Type>),
    Dynamic(DynamicTypeMessage),
    Known(Enum),
}

impl<'bytes, Enum, Type> FullMessage<'bytes, Enum, Type>
where Type: Deserialize<'bytes> + std::convert::TryFrom<u8> + 'static,
      Enum: Deserialize<'bytes> + TryFrom<UndeserializedBytes<'bytes, Type>> {
    fn new(bytes: &'bytes [u8]) -> Self { FullMessage::Raw(bytes) }
    fn to_underserialized(&mut self) -> Result<(), crate::serde::Error> {
        *self = match self {
            FullMessage::Raw(bytes) => {
                match crate::serde::de::deserialize(bytes) {
                    Ok(v) => FullMessage::Dynamic(v),
                    Err(err) => return Err(err),
                }
            },
            FullMessage::Undeserialized(_) => return Ok(()),
            _ => return Err(crate::serde::Error::WrongStatus)
        };
        Ok(())
    }

    fn to_known(&mut self) -> Result<(), crate::serde::Error> {
        *self = match self {
            FullMessage::Raw(bytes) => {
                match crate::serde::de::deserialize(bytes) {
                    Ok(v) => FullMessage::Known(v),
                    Err(_) => FullMessage::Unknown(bytes)
                }
            },
            FullMessage::Unknown(_) => return Err(crate::serde::Error::UnknownType),
            FullMessage::Undeserialized(unde) => {
                todo!()
                //FullMessage::Known(Enum::try_from(*unde).map_err(|e| crate::serde::Error::UnknownType)?)
            },
            FullMessage::Dynamic(_) => todo!(),
            FullMessage::Known(_) => todo!(),
        };
        Ok(())
    }
    fn to_dyn(&mut self) {
        
    }
}

impl<'bytes, Enum, Type> Serialize for FullMessage<'bytes, Enum, Type>
    where Enum: Serialize, Type: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            FullMessage::Raw(bytes) => serializer.serialize_bytes(bytes),
            FullMessage::Unknown(bytes) => serializer.serialize_bytes(bytes),
            FullMessage::Undeserialized(v) => v.serialize(serializer),
            FullMessage::Dynamic(v) => v.serialize(serializer),
            FullMessage::Known(v) => v.serialize(serializer),
        }
    }
}

impl<'de, 'bytes, Enum, Type> Deserialize<'de> for FullMessage<'bytes, Enum, Type>
where 
    Type: Deserialize<'de> + std::convert::TryFrom<u8> + 'static,
    Enum: 'bytes,
    Type: 'bytes 
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        Ok(deserializer.deserialize_bytes(FullMessageVisitor::new())?)
    }
}

struct FullMessageVisitor<'bytes, Enum, Type> {
    phantom: PhantomData<&'bytes (Enum, Type)>
}

impl<'bytes, Enum, Type> FullMessageVisitor<'bytes, Enum, Type> {
    fn new() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<'de, 'bytes, Enum, Type> serde::de::Visitor<'de> for FullMessageVisitor<'bytes, Enum, Type> {
    type Value = FullMessage<'bytes, Enum, Type>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Full message")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error, {
        let extended_v = unsafe { std::slice::from_raw_parts(v.as_ptr(), v.len()) };
        Ok(FullMessage::Raw(extended_v))
    }
}

mod test {
    #![allow(unused_imports)]

    use crate::serde::de::deserialize;
    use crate::serde::ser::serialize;
    use crate::serde::LengthWrapper;
    use crate::message::stoc::HandResult;
    use crate::message::stoc::MessageType;
    use crate::message::stoc::MessageEnum;

    use super::UndeserializedBytes;
 
    #[test]
    fn test_unknown_bytes() {
        let message = MessageEnum::HandResult(HandResult {
            res1: 3,
            res2: 5,
        });
        let bytes = serialize(&message).unwrap();
        let undeser = UndeserializedBytes::<MessageType> {
            message_type: MessageType::HandResult,
            bytes: bytes[2..].as_ref()
        };
        let target = LengthWrapper(undeser);
        let target_bytes = serialize(&target).unwrap();
        assert_eq!(target_bytes, [3, 0, 5, 3, 5]);
        let target2: LengthWrapper<UndeserializedBytes<MessageType>> = deserialize(&target_bytes).unwrap();
        println!("{:?}", target2)
    }
}

macro_rules! build_it {
    ($($name:ident = $flag:expr,)*) => {
        #[derive(serde::Serialize, serde::Deserialize, Copy, Clone, num_enum::TryFromPrimitive, num_enum::IntoPrimitive, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
        #[repr(u8)]
        #[serde(into = "u8", try_from = "u8")]
        pub enum MessageType {
            $($name = $flag),*
        }

        #[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
        #[serde(into = "u8", from = "u8")]
        pub enum ExtendedMessageType {
            Known(MessageType),
            Unknown(u8)
        }

        impl std::convert::Into<u8> for ExtendedMessageType {
            fn into(self) -> u8 { 
                match self {
                    ExtendedMessageType::Known(message_type) => message_type.into(),
                    ExtendedMessageType::Unknown(message_type) => message_type
                }
            }
        }

        impl std::convert::From<u8> for ExtendedMessageType {
            fn from(value: u8) -> Self { 
                match MessageType::try_from(value) {
                    Ok(message_type) => ExtendedMessageType::Known(message_type),
                    Err(_) => ExtendedMessageType::Unknown(value)
                }
            }
        }

        impl From<MessageType> for ExtendedMessageType {
            fn from(value: MessageType) -> Self {
                ExtendedMessageType::Known(value)
            }
        }

        
        impl std::fmt::Display for MessageType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(MessageType::$name => write!(f, stringify!($name))),*
                }       
            }
        }

        impl std::fmt::Display for ExtendedMessageType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    ExtendedMessageType::Known(message_type) => message_type.fmt(f),
                    ExtendedMessageType::Unknown(number) => write!(f, "Unknown [{}]", number)
                }       
            }
        } 

        #[derive(Debug)]
        pub enum MessageEnum {
            $($name($name),)*
        }

        impl crate::serde::LengthDescribed for MessageEnum {
            fn sizeof(&self) -> usize {
                1 + match self {
                    $(MessageEnum::$name(v) => v.sizeof(),)*
                }
            } 
        }

        impl<'bytes> std::convert::TryFrom<crate::message::UndeserializedBytes<'bytes, MessageType>> for MessageEnum {
            type Error = crate::serde::Error;
            fn try_from(message: crate::message::UndeserializedBytes<'bytes, MessageType>) -> Result<Self, Self::Error> {
                Ok(match message.message_type {
                    $(MessageType::$name => MessageEnum::$name(crate::serde::de::deserialize(message.bytes)?) ,)*
                })
            }
        }

        impl<'bytes> std::convert::TryFrom<crate::message::UndeserializedBytes<'bytes, ExtendedMessageType>> for MessageEnum {
            type Error = crate::serde::Error;
            fn try_from(message: crate::message::UndeserializedBytes<'bytes, ExtendedMessageType>) -> Result<Self, Self::Error> {
                match message.message_type {
                    ExtendedMessageType::Known(message_type) => MessageEnum::try_from(crate::message::UndeserializedBytes { message_type, bytes: message.bytes }),
                    ExtendedMessageType::Unknown(_) => Err(crate::serde::Error::UnknownType)
                }
            }
        } 

        impl serde::Serialize for MessageEnum {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                match self {
                    $(MessageEnum::$name(v) => serializer.serialize_newtype_variant("MessageEnum", $name::message_type().into(), "$name", v),)*
                }
            }
        }
        
        impl<'de> serde::Deserialize<'de> for MessageEnum {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                deserializer.deserialize_enum("MessageEnum", &[], MessageEnumVisitor)
            }
        }
        struct MessageEnumVisitor;
        
        impl<'de> serde::de::Visitor<'de> for MessageEnumVisitor {
            type Value = MessageEnum;
        
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Message Enum")
            }
        
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error> where A: serde::de::EnumAccess<'de>, {
                Ok(match data.variant()? {
                    $(($flag, v) => MessageEnum::$name(v.newtype_variant::<$name>()?),)*
                    _ => return Err(<A::Error as serde::de::Error>::custom("Unknown variant"))
                })
            }
        }
    };
}
pub(crate) use build_it;


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
#[repr(C)]
pub struct HostInfo {
    pub lflist: i32,
    pub rule: u8,
    pub mode: Mode,
    pub duel_rule: u8,
    pub no_check_deck: bool,
    pub no_shuffle_deck: bool,
    pub padding: [u8; 3],
    pub start_lp: u32,
    pub start_hand: u8,
    pub draw_count: u8,
    pub time_limit: u16
}

impl Default for HostInfo {
    fn default() -> Self {
        Self { 
            lflist: 0, 
            rule: 0, 
            mode: Mode::Match, 
            duel_rule: 5, 
            no_check_deck: false, 
            no_shuffle_deck: false, 
            padding: [0; 3], 
            start_lp: 8000,
            start_hand: 5, 
            draw_count: 1, 
            time_limit: 180
        }
    }
}
