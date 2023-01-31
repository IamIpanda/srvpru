use std::io::{BufReader, BufRead, Cursor, Seek, Read};

use serde::Deserialize;
use serde::de::{SeqAccess, DeserializeSeed, EnumAccess, VariantAccess, IntoDeserializer};
use byteorder::{ReadBytesExt, LittleEndian};

use super::Error;

struct Deserializer<'de> {
    memory_alignment: u32,
    limit: Vec<(u64, u64)>,
    reader: BufReader<Cursor<&'de [u8]>>,
}

impl<'de> Deserializer<'de> {
    pub fn new(data: &'de [u8]) -> Self {
        Self {
            memory_alignment: 0,
            limit: vec!(),
            reader: BufReader::new(Cursor::new(data))
        }
    }

    pub fn limit(&mut self, length: u64) {
        let position = self.reader.stream_position().unwrap_or(0);
        self.limit.push((position, length));
    }

    pub fn eof(&mut self) -> bool {
        match self.limit.get(0) {
            Some((start, length)) => if self.reader.stream_position().unwrap_or(0) >= start + length {
                self.limit.pop();
                true
            } else { false },
            None => match self.reader.fill_buf() {
                Ok(b) => b.is_empty(),
                Err(_) => true
            }
        }
    }
}

pub fn deserialize<'de, T: Deserialize<'de>>(data: &'de [u8]) -> Result<T, Error> {
    let mut deserializer = Deserializer::new(data);
    T::deserialize(&mut deserializer)
}

impl<'de, 'der> serde::Deserializer<'de> for &'der mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        let len = self.reader.read_u16::<LittleEndian>().map_err(|err| Error::IO(err))?;
        self.limit(len as u64);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_bool(match self.reader.read_u8() {
            Ok(v) => Ok(v > 0),
            Err(err) => Err(Error::IO(err))
        }?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_i8(self.reader.read_i8().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_i16(self.reader.read_i16::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_i32(self.reader.read_i32::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_i64(self.reader.read_i64::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_u8(self.reader.read_u8().map_err(|err| Error::IO(err))?) 
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_u16(self.reader.read_u16::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_u32(self.reader.read_u32::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_u64(self.reader.read_u64::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_f32(self.reader.read_f32::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_f64(self.reader.read_f64::<LittleEndian>().map_err(|err| Error::IO(err))?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_char(match self.reader.read_u8() {
            Ok(v) => Ok(v as char),
            Err(err) => Err(Error::IO(err))
        }?)
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        unimplemented!("Don't support read str, please use utf-16 structure.")
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        match self.limit.pop() {
            Some((pos, length)) => {
                let current_position = self.reader.stream_position().map_err(|e| Error::IO(e))? as usize;
                if current_position != pos as usize { Err(Error::Oversize)?; }
                let stop_position = (pos + length) as usize;
                self.reader.consume(length as usize);
                let reference = &self.reader.get_ref().get_ref();
                visitor.visit_bytes(&reference[current_position..stop_position])
            },
            None => Err(Error::Unlimited)
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        match self.limit.pop() {
            Some((pos, length)) => {
                // check pos
                let mut buf = vec![0u8; length as usize];
                self.reader.read_exact(buf.as_mut()).map_err(|e| Error::IO(e))?;
                visitor.visit_bytes(&buf)
            },
            None => Err(Error::Unlimited)
        }
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        unimplemented!("Don't support read option")
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_unit() 
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_newtype_struct(self)
    }

    // seq is always infinity.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_seq(DeserializeSequenceAccess {
            deserializer: self,
            length: usize::MAX
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_seq(DeserializeSequenceAccess {
            deserializer: self,
            length: len
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        unimplemented!("Don't support deserialize Hashmap.")
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        unimplemented!("Don't support deserialize Identifier.")
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        unimplemented!("Don't support deserialize Ignored any")
    }
}


struct DeserializeSequenceAccess<'de, 'der> {
    deserializer: &'der mut Deserializer<'de>,
    length: usize, 
}

impl<'de, 'der> SeqAccess<'de> for DeserializeSequenceAccess<'de, 'der> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de> {
        if self.length == 0 { return Ok(None) }
        if self.length < usize::MAX { self.length -= 1; }
        else if self.deserializer.eof() { return Ok(None) }
        let res = DeserializeSeed::deserialize(seed, &mut *self.deserializer);
        if self.length == 0 { self.deserializer.eof(); } // A perfect LengthWrapped reach its end, remove that limit
        Ok(Some(res?))
    }

    fn size_hint(&self) -> Option<usize> {
        if self.length == usize::MAX { None }
        else { Some(self.length) }
    }
}

impl<'de, 'der> EnumAccess<'de> for &'der mut Deserializer<'de> {
    type Error = Error;
    type Variant = &'der mut Deserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> where V: DeserializeSeed<'de> {
        let leading = self.reader.read_u8().map_err(|err| Error::IO(err))?;
        let val =seed.deserialize(leading.into_deserializer());
        Ok((val?, self))
    }
}

impl<'de, 'der> VariantAccess<'de> for &'der mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error> where T: DeserializeSeed<'de> {
        DeserializeSeed::deserialize(seed, self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        serde::de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> where V: serde::de::Visitor<'de> {
        serde::de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}

mod test {
    #![allow(unused_imports)]
    
    use crate::message::HostInfo;
    use crate::message::client_to_server::Chat;
    use crate::message::client_to_server::CreateGame;
    use crate::message::server_to_client::HandResult;
    
    use crate::serde::raw::serialize;
    use crate::utils::string::cast_to_string;

    use super::deserialize;

    #[test]
    fn test_deserialize() {
        let v1 = HandResult {
            res1: 5,
            res2: 7,
        };
        let v2 = CreateGame {
            info: HostInfo::default(),
            name: "player".into(),
            pass: "M".into(),
        };
        let v3 = vec![72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 0, 119, 0, 111, 0, 114, 0, 108, 0, 100, 0, 33, 0, 0, 0];
        let hand_result: HandResult = deserialize(& unsafe { serialize(&v1) }).unwrap();
        let create_game: CreateGame = deserialize(& unsafe { serialize(&v2) }).unwrap();
        let chat: Chat = deserialize(&v3).unwrap();
        assert_eq!(hand_result.res1, 5);
        //assert_eq!(create_game.pass, [2; 20]);
        //assert_eq!(cast_to_string(&chat.msg).unwrap(), "Hello, world!")
    }
}
