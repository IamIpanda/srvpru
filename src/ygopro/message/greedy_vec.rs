use std::marker::PhantomData;
use serde::ser::{Serializer, SerializeTuple};
use serde::de::{Deserializer, Visitor, SeqAccess, Error};

trait GreedyVec<'a, const N: usize>: Sized {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer;
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'a>;
}

macro_rules! greedy_vec {
    ($($max_length:expr,)+) => {
        $(
            impl<'a, T> GreedyVec<'a, $max_length> for Vec<T> where T: Default + Copy + Serialize + Deserialize<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
                    let mut seq = serializer.serialize_tuple(self.len())?;
                    for elem in &self[..] {
                        seq.serialize_element(elem)?;
                    }
                    seq.end()
                }

                fn deserialize<D>(deserializer: D) -> Result<Vec<T>, D::Error> where D: Deserializer<'a>
                {
                    struct ArrayVisitor<T> { element: PhantomData<T> }

                    impl<'a, T> Visitor<'a> for ArrayVisitor<T> where T: Default + Copy + Deserialize<'a>
                    {
                        type Value = Vec<T>;
                        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                            formatter.write_str(concat!("a series of value no more than ", $max_length))
                        }

                        fn visit_seq<A>(self, mut seq: A) -> Result<Vec<T>, A::Error> where A: SeqAccess<'a>
                        {
                            let mut arr = Vec::new();
                            let count = 0;
                            loop {
                                let element = seq.next_element();
                                if let Ok(value) = element {
                                    if let Some(actual_value) = value { arr.push(actual_value); }
                                    else { break }
                                }
                                else { break }
                                // else { return Err(Error::invalid_type(serde::de::Unexpected::Str("Wrong in deserialization"), &self)); }
                                if count >= $max_length { return Err(Error::invalid_length($max_length, &self)); }
                            }
                            Ok(arr)
                        }
                    }

                    let visitor = ArrayVisitor { element: PhantomData };
                    deserializer.deserialize_tuple($max_length, visitor)
                }
            }
        )+
    }
}