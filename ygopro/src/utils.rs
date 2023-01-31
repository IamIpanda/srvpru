
pub mod reader {
    use byteorder::{ReadBytesExt, LittleEndian};
    use std::io::Result;

    pub fn read_array<T: ReadBytesExt, const N: usize>(reader: &mut T) -> Result<[u8; N]> {
        let mut arr = [0u8; N];
        for i in 0..N { arr[i] = reader.read_u8()?; }
        Ok(arr)
    }

    pub fn read_array_with_length<T: ReadBytesExt>(reader: &mut T) -> Result<Vec<u32>> {
        let length = reader.read_u32::<LittleEndian>()?;
        let mut vec = Vec::new();
        for _ in 0..length as usize { vec.push(reader.read_u32::<LittleEndian>()?) }
        Ok(vec)
    }

    pub fn read_string<T: ReadBytesExt, const N: usize>(reader: &mut T) -> Result<String> {
        let mut arr = [0u16; N];
        for i in 0..N { arr[i] = reader.read_u16::<LittleEndian>()?; }
        super::string::cast_to_string(&arr).ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Illegal utf-16 string"))
    }
}

pub mod string {
    #![allow(dead_code)]

    use std::ops::Deref;

    use serde::ser::{SerializeTuple, SerializeSeq};
    use serde::de::Visitor;

    use crate::serde::LengthDescribed;

    /// transform \[u16\] to string. \
    /// return [`None`] if it's illegal.
    pub fn cast_to_string(array: &[u16]) -> Option<String> {
        let mut str = array;
        if let Some(index) = array.iter().position(|&i| i == 0) {
            str = &str[0..index as usize];
        }
        else { return None }
        let body = unsafe { std::slice::from_raw_parts(str.as_ptr() as *const u8, str.len() * 2) };
        let (cow, _, had_errors) = encoding_rs::UTF_16LE.decode(&body);
        if had_errors { None }
        else { Some(cow.to_string()) }
    }

    /// Transform string to \[u16\] without length limit but a \0 in the end.
    pub fn cast_to_c_array(message: &str) -> Vec<u16> {
        let mut vector: Vec<u16> = message.encode_utf16().collect();
        vector.push(0);
        vector
    }

    /// Transform string to \[u16\] with a fixed size. \
    /// Differennt from ygopro, it will keeps 0 for residual part.
    pub fn cast_to_fix_length_array<const N: usize>(message: &str) -> [u16; N] {
        let mut data = [0u16; N];
        for (index, chr) in message.encode_utf16().enumerate() {
            data[index] = chr;
        }
        data
    }

    #[derive(Debug)]
    pub struct FixedLengthString<const L: usize> {
        data: [u16; L],
        str: Option<String>,
    }

    impl<const L: usize> FixedLengthString<L> {
        pub fn new(str: String) -> Self {
            Self {
                data: cast_to_fix_length_array(&str),
                str: Some(str)
            }
        }

        pub fn resolve_data(&mut self) {
            if self.str == None {
                self.str = cast_to_string(&self.data)
            }
        }

        pub fn resolve_str(&mut self) {
            match self.str.as_ref() {
                Some(str) => self.data = cast_to_fix_length_array(str),
                None => (),
            }
        }        
    }

    impl<const L: usize> Deref for FixedLengthString<L> {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            match &self.str {
                Some(str) => str.as_str(),
                None => ""
            }
        }
    }

    impl<const L: usize> From<String> for FixedLengthString<L> {
        fn from(value: String) -> Self {
            FixedLengthString::new(value)
        }
    }

    impl<'s, const L: usize> From<&'s str> for FixedLengthString<L> {
        fn from(value: &'s str) -> Self {
            FixedLengthString::new(value.to_string())
        }
    }

    impl<const L: usize> LengthDescribed for FixedLengthString<L> {
        fn sizeof(&self) -> usize {
            L * 2
        }
    }

    impl<const L: usize> serde::Serialize for FixedLengthString<L> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
            let mut ser = serializer.serialize_tuple(L)?;
            for v in &self.data[..] {
                ser.serialize_element(v)?;
            }
            ser.end()
        }
    }

    impl<'de, const L: usize> serde::Deserialize<'de> for FixedLengthString<L> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
            let visitor = FixedLengthStringVisitor::<L> {};
            if L == 0 { deserializer.deserialize_seq(visitor) }
            else { deserializer.deserialize_tuple(L, visitor) }
        }
    }

    struct FixedLengthStringVisitor<const L: usize> { }
    impl<'de, const L: usize> Visitor<'de> for FixedLengthStringVisitor<L> {
        type Value = FixedLengthString<L>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(&format!("A {} length string", L))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
            let mut array = [0u16; L];
            for i in 0..L {
                match seq.next_element() {
                    Ok(Some(v)) => array[i] = v,
                    Ok(None) => break,
                    Err(err) => return Err(err)
                }
            }
            Ok(FixedLengthString {
                data: array,
                str: cast_to_string(&array)
            })
        }
    }
    
    #[derive(Debug)]
    pub struct U16String {
        data: Vec<u16>,
        str: Option<String>,
    }

    impl U16String {
        pub fn new(str: String) -> Self {
            Self {
                data: cast_to_c_array(&str),
                str: Some(str)
            }
        }

        pub fn resolve_data(&mut self) {
            if self.str == None {
                self.str = cast_to_string(&self.data)
            }
        }

        pub fn resolve_str(&mut self) {
            match self.str.as_ref() {
                Some(str) => self.data = cast_to_c_array(str),
                None => (),
            }
        }        
    }

    impl Deref for U16String {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            match &self.str {
                Some(str) => str.as_str(),
                None => ""
            }
        }
    }

    impl From<String> for U16String {
        fn from(value: String) -> Self {
            U16String::new(value)
        }
    }

    impl<'s> From<&'s str> for U16String {
        fn from(value: &'s str) -> Self {
            U16String::new(value.to_string())
        }
    }

    impl LengthDescribed for U16String {
        fn sizeof(&self) -> usize {
            self.data.len() * 2
        }
    }
    
    impl serde::Serialize for U16String {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
            let mut ser = serializer.serialize_seq(None)?;
            for v in &self.data[..] {
                ser.serialize_element(v)?;
            }
            ser.end()
        }
    }

    impl<'de> serde::Deserialize<'de> for U16String {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
            deserializer.deserialize_seq(U16StringVisitor)
        }
    }

    struct U16StringVisitor;
    impl<'de> Visitor<'de> for U16StringVisitor {
        type Value = U16String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("u16 string")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
            let mut vec = vec!();
            loop {
                match seq.next_element() {
                    Ok(Some(v)) => vec.push(v),
                    Ok(None) => break,
                    Err(err) => return Err(err)
                }
            }
            Ok(U16String {
                data: vec,
                str: None
            })
        }
    }

    mod test {
        #![allow(unused_imports)]

        use crate::serde::{ser::serialize, de::deserialize};

        use super::FixedLengthString;

        #[test]
        fn test_string() {
            let s = FixedLengthString::<20>::new("Hello, world!".to_string());
            let bytes = serialize(&s).unwrap();
            assert_eq!(bytes, [72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 0, 119, 0, 111, 0, 114, 0, 108, 0, 100, 0, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let mut s2: FixedLengthString<20> = deserialize(&bytes).unwrap();
            s2.resolve_data();
            assert_eq!(s2.str, Some("Hello, world!".to_string()));
        }

        #[test]
        fn test_vec_string() {
            let s = FixedLengthString::<0>::new("Hello, world!".to_string());
            let bytes = serialize(&s).unwrap();
            assert_eq!(bytes, [])
        }
    }
}

