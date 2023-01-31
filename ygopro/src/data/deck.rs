use serde::Serialize;
use serde::Deserialize;
use serde::de::Visitor;
use serde::ser::SerializeSeq;

use crate::utils::reader::read_array_with_length;
//use crate::message::ctos::UpdateDeck;

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct Deck {
    pub main: Vec<u32>,
    pub side: Vec<u32>,
    pub ex: Vec<u32> // always empty
}

impl Deck {
    /*
    pub fn from_data(data: &UpdateDeck) -> Deck {
        Deck {
            main: data.deckbuf[0..data.mainc].to_vec(),
            side: data.deckbuf[data.mainc..data.mainc + data.sidec].to_vec(),
            ex: Vec::new()
        }
    }
    */

    pub fn from_reader<T: byteorder::ReadBytesExt>(reader: &mut T) -> std::io::Result<Deck> {
        Ok(Deck {
            main: read_array_with_length(reader)?,
            ex: read_array_with_length(reader)?,
            side: Vec::new()
        })
    }
}

impl Serialize for Deck {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.main)?;
        seq.serialize_element(&self.side)?;
        // No extra deck.
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Deck {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_tuple(4, DeckVisitor)
    }
}

struct DeckVisitor;
impl<'de> Visitor<'de> for DeckVisitor {
    type Value = Deck;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Deck")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
        let main_count = seq.next_element()?.unwrap_or(0u32);
        let side_count = seq.next_element()?.unwrap_or(0u32);
        let mut main = vec![];
        let mut side = vec![];
        for _ in 0..main_count { main.push(seq.next_element()?.unwrap_or(0)); }
        for _ in 0..side_count { side.push(seq.next_element()?.unwrap_or(0)); }
        Ok(Deck { main, side, ex: vec![] })
    }    
}
