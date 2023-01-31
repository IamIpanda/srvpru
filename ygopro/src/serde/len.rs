use std::marker::PhantomData;

use serde::Deserialize;
use serde::ser::SerializeSeq;
use serde::de::Visitor;

#[derive(Debug)]
pub struct LengthWrapper<T>(pub T);

pub trait LengthDescribed {
    fn sizeof(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

impl<T: LengthDescribed + serde::Serialize> serde::Serialize for LengthWrapper<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let size = self.0.sizeof();
        let size = u16::try_from(size).expect("Cannot serialize a struct size over u16");
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&size)?;
        seq.serialize_element(&self.0)?;
        seq.end()
    }
}

impl <'de, T: LengthDescribed> Deserialize<'de> for LengthWrapper<T> where T: Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_any(LengthWrapperVisitor::<T>::new())
    }
}

impl<T> LengthDescribed for LengthWrapper<T> where T: LengthDescribed {
    fn sizeof(&self) -> usize {
        self.0.sizeof() + 2
    }
}

struct LengthWrapperVisitor<T> {
    phantom: PhantomData<T>
}

impl<T> LengthWrapperVisitor<T> {
    fn new() -> Self {
        Self { phantom: PhantomData {} }
    }
}

impl<'de, T> Visitor<'de> for LengthWrapperVisitor<T> where T: Deserialize<'de> {
    type Value = LengthWrapper<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Length wrapper")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: serde::Deserializer<'de>, {
        Ok(LengthWrapper(T::deserialize(deserializer)?))
    }
}

mod test {
    #![allow(unused_imports)]

    use crate::message::UndeserializedBytes;
    use crate::serde::de::deserialize;
    use crate::serde::ser::serialize;
    use crate::message::stoc::{MessageType, Chat, MessageEnum, HandResult, FieldFinish};
    use crate::utils::string::cast_to_c_array;

    use super::LengthWrapper;

    #[test]
    fn test_length_serialize() {
        let chat = Chat { name: 2, msg: "hello".into() };
        let hand_result = HandResult { res1: 3, res2: 7 };
        assert_eq!(serialize(&LengthWrapper(chat)).unwrap(), vec![14, 0, 2, 0, 104, 0, 101, 0, 108, 0, 108, 0, 111, 0, 0, 0]);
        assert_eq!(serialize(&LengthWrapper(MessageEnum::HandResult(hand_result))).unwrap(), vec![3, 0, 5, 3, 7]);
    }

    #[test]
    fn test_length_deserialize() {
        let d: LengthWrapper<MessageEnum> = deserialize(&[3, 0, 5, 3, 7]).unwrap();
        assert!(matches!(d.0, MessageEnum::HandResult(_)));
    }

    fn test_multiple_serialize() -> Vec<u8> {
        let chat = Chat { name: 2, msg: "M".into() };
        let hand_result = HandResult { res1: 3, res2: 7 };
        let field_finish = FieldFinish {};
        let messages = vec![
            LengthWrapper(MessageEnum::HandResult(hand_result)), 
            LengthWrapper(MessageEnum::Chat(chat)), 
            LengthWrapper(MessageEnum::FieldFinish(field_finish))
        ];
        let binary = serialize(&messages).unwrap();
        assert_eq!(binary, vec![3, 0, 5, 3, 7, 7, 0, 25, 2, 0, 77, 0, 0, 0, 1, 0, 48]);
        binary
    }

    #[test]
    fn test_multiple_deserialize() {
        let binary = test_multiple_serialize();
        let d: Vec<LengthWrapper<MessageEnum>> = deserialize(&binary).unwrap();
        assert!(matches!(d[0].0, MessageEnum::HandResult(_)));
        assert!(matches!(d[1].0, MessageEnum::Chat(_)));
        assert!(matches!(d[2].0, MessageEnum::FieldFinish(_)));
    }

    #[test]
    fn test_multiple_deserialize_undeserialized() {
        let binary = test_multiple_serialize();
        let d: Vec<LengthWrapper<UndeserializedBytes<MessageType>>> = deserialize(&binary).unwrap();
        assert_eq!(d[0].0.message_type, MessageType::HandResult);
        assert_eq!(d[1].0.message_type, MessageType::Chat);
        assert_eq!(d[2].0.message_type, MessageType::FieldFinish);
        let d2: Vec<MessageEnum> = d.into_iter().map(|s| s.0.try_into().unwrap()).collect();
        assert!(matches!(d2[0], MessageEnum::HandResult(_)));
        assert!(matches!(d2[1], MessageEnum::Chat(_)));
        assert!(matches!(d2[2], MessageEnum::FieldFinish(_))); 
    }
}
