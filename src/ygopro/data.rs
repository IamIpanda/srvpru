#![allow(dead_code)]

use super::message::CTOSUpdateDeck;

pub struct Deck {
    pub main: Vec<u32>,
    pub side: Vec<u32>,
    pub ex: Vec<u32> // always empty
}

impl Deck {
    fn from_data(data: &CTOSUpdateDeck) -> Deck {
        Deck {
            main: data.deckbuf[0..data.mainc].to_vec(),
            side: data.deckbuf[data.mainc..data.mainc + data.sidec].to_vec(),
            ex: Vec::new()
        }
    }
}