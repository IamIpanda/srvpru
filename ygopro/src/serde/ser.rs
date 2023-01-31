use std::io::BufWriter;
use std::io::Write;

use byteorder::{WriteBytesExt, LittleEndian};
use serde::Serialize;


use super::Error;
use super::map_std_io_result;

pub struct Serializer {
    memory_alignment: u32,
    writer: BufWriter<Vec<u8>>,
}

impl Serializer {
    pub fn new() -> Self {
        Self {
            memory_alignment: 4,
            writer: BufWriter::new(Vec::new()),
        }
    }
}

pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    serializer.writer.into_inner().map_err(|_| Error::UnwrapWriter)
}

impl<'ser> serde::ser::Serializer for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_u8(if v { 1 } else { 0 })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_i8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_i16::<LittleEndian>(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_i32::<LittleEndian>(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_i64::<LittleEndian>(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_u8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_u16::<LittleEndian>(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_u32::<LittleEndian>(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_u64::<LittleEndian>(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_f32::<LittleEndian>(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_f64::<LittleEndian>(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_u8(v as u8) 
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Cannot serializer str")
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        map_std_io_result(self.writer.write_all(v))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!("Cannot serialize Option")
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error> where T: serde::Serialize,
    {
        unimplemented!("Cannot serialize Option")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let value = u8::try_from(variant_index).expect("An Enum variant index is over u8 range."); 
        map_std_io_result(self.writer.write_u8(value))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> where T: serde::Serialize,
    {
        T::serialize(value, self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> where T: serde::Serialize,
    {
        let lead = u8::try_from(variant_index).expect("An Enum variant index is over u8 range.");
        self.writer.write_u8(lead).map_err(|err| Error::IO(err))?;
        T::serialize(value, self)
    }
    

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let len = u16::try_from(len).map_err(|_| Error::Oversize)?;
        map_std_io_result(self.writer.write_u16::<LittleEndian>(len))?;
        Ok(self)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!("Cannot serialize hashmap")
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(self)
    }
}

impl<'ser> serde::ser::SerializeMap for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        unimplemented!("Don't support serialize hashmap")
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        unimplemented!("Don't support serialize hashmap")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeSeq for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeStruct for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeTuple for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeStructVariant for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeTupleStruct for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser> serde::ser::SerializeTupleVariant for &'ser mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

mod test {
    #![allow(unused_imports)]

    use crate::message::server_to_client::HandResult;
    use crate::message::client_to_server::CreateGame;
    use crate::message::client_to_server::Chat;
    use crate::message::HostInfo;
    use crate::utils::string::FixedLengthString;
    use crate::utils::string::cast_to_c_array;
    use super::serialize;

    #[test]
    fn test_serialize() {
        let v1 = HandResult {
            res1: 5,
            res2: 7,
        };
        let v2 = CreateGame {
            info: HostInfo::default(),
            name: FixedLengthString::new("Player".to_string()),
            pass: FixedLengthString::new("N".to_string()),
        };
        let v3: Chat = "Hello, world!".into();
        assert_eq!(serialize(&v1).unwrap(), unsafe { crate::serde::raw::serialize(&v1) });
        assert_eq!(serialize(&v2).unwrap(), unsafe { crate::serde::raw::serialize(&v2) });
        assert_eq!(serialize(&v3).unwrap(), vec![72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 0, 119, 0, 111, 0, 114, 0, 108, 0, 100, 0, 33, 0, 0, 0]);
    }
}
