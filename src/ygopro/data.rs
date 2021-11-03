#![allow(dead_code)]
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use lzma_rs::lzma_decompress_with_options;


use crate::ygopro::message::ctos::UpdateDeck;
use crate::ygopro::message::string::cast_to_string;

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct Deck {
    pub main: Vec<u32>,
    pub side: Vec<u32>,
    pub ex: Vec<u32> // always empty
}

impl Deck {
    pub fn from_data(data: &UpdateDeck) -> Deck {
        Deck {
            main: data.deckbuf[0..data.mainc].to_vec(),
            side: data.deckbuf[data.mainc..data.mainc + data.sidec].to_vec(),
            ex: Vec::new()
        }
    }

    pub fn from_reader<T: byteorder::ReadBytesExt>(reader: &mut T) -> anyhow::Result<Deck> {
        Ok(Deck {
            main: read_array_with_length(reader)?,
            ex: read_array_with_length(reader)?,
            side: Vec::new()
        })
    }

    pub fn to_update_deck(&self) -> UpdateDeck {
        todo!()
    }
}

impl core::convert::From<UpdateDeck> for Deck {
    fn from(data: UpdateDeck) -> Self {
        Deck::from_data(&data) 
    }
}

impl std::fmt::Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "#generated by srvpru deck log")?;
        writeln!(f, "#main")?;
        for card in self.main.iter() {  
            writeln!(f, "{:}", card)?;
        }
        writeln!(f, "!side")?;
        for card in self.side.iter() {
            writeln!(f, "{:}", card)?;
        }
        Ok(())
    }
}

pub const REPLAY_COMPRESSED_FLAG: u32 = 1;
pub const REPLAY_TAG_FLAG: u32 = 2;
pub const REPLAY_DECODE_FLAG: u32 = 4;
pub const REPLAY_SINGLE_MODE: u32 = 8;
pub const REPLAY_UNIFORM: u32 = 16;

pub struct ReplayHeader {
    pub id: u32,
    pub version: u32,
    pub flag: u32,
    pub seed: u32,
    pub data_size: u32,
    pub start_time: u32,
    pub props: [u8; 8]
}

pub struct Replay {
    pub header: ReplayHeader,
    pub host_name: String,
    pub client_name: String,
    pub start_lp: u32,
    pub start_hand: u32,
    pub draw_count: u32,
    pub opt: u32,
    pub host_deck: Deck,
    pub client_deck: Deck,

    pub tag_host_name: Option<String>,
    pub tag_client_name: Option<String>,
    pub tag_host_deck: Option<Deck>,
    pub tag_client_deck: Option<Deck>,

    pub datas: Vec<Vec<u8>>
}

impl ReplayHeader {
    pub fn from_reader<T: ReadBytesExt>(reader: &mut T) -> anyhow::Result<ReplayHeader> {
        Ok(ReplayHeader {
            id: reader.read_u32::<LittleEndian>()?,
            version: reader.read_u32::<LittleEndian>()?,
            flag: reader.read_u32::<LittleEndian>()?,
            seed: reader.read_u32::<LittleEndian>()?,
            data_size: reader.read_u32::<LittleEndian>()?,
            start_time: reader.read_u32::<LittleEndian>()?,
            props: read_array(reader)?,
        })
    }

    pub fn is_compressed(&self) -> bool { self.flag & REPLAY_COMPRESSED_FLAG > 0 }
    pub fn is_tag(&self)        -> bool { self.flag & REPLAY_TAG_FLAG > 0 }
    #[allow(dead_code)]
    pub fn is_decoded(&self)    -> bool { self.flag & REPLAY_DECODE_FLAG > 0 }
}

impl Replay {
    // ==================================================
    // Correct order: 
    // prop  dict_size  datasize
    //  93    0 0 0 1     u64
    // Ygopro replay header:
    // datasize  prop  dict_size  padding
    //   u32      93    0 0 0 1    0 0 0
    // ==================================================
    pub fn from_reader<T: ReadBytesExt + BufRead>(reader: &mut T) -> anyhow::Result<Replay> {
        let header = ReplayHeader::from_reader(reader)?; 
        let leading_props = Cursor::new(&header.props[0..5]);
        let mut compressed_data = leading_props.chain(reader);
        let mut decompressed_vector = Vec::new();
        lzma_decompress_with_options(&mut compressed_data, &mut decompressed_vector, &lzma_rs::decompress::Options { 
            unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(header.data_size as u64)), 
            memlimit: None,
            allow_incomplete: false 
        })?;
        let mut decompressed_reader = Cursor::new(decompressed_vector);
        let reader = &mut decompressed_reader;
        let is_tag = header.is_tag();
        let mut replay = Replay {
            header,
            host_name: read_string::<_, 20>(reader)?,
            tag_host_name: if is_tag { Some(read_string::<_, 20>(reader)?) } else { None },
            tag_client_name: if is_tag { Some(read_string::<_, 20>(reader)?) } else { None },
            client_name: read_string::<_, 20>(reader)?, 
            start_lp: reader.read_u32::<LittleEndian>()?,
            start_hand: reader.read_u32::<LittleEndian>()?,
            draw_count: reader.read_u32::<LittleEndian>()?,
            opt: reader.read_u32::<LittleEndian>()?,
            host_deck: Deck::from_reader(reader)?,
            tag_host_deck: if is_tag { Some(Deck::from_reader(reader)?) } else { None },
            tag_client_deck: if is_tag { Some(Deck::from_reader(reader)?) } else { None },
            client_deck: Deck::from_reader(reader)?,
            datas: Vec::new(),
        };
        loop {
            let length = reader.read_u8();
            if let Err(ref e) = length {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
            }
            let mut data = vec![0u8; length? as usize];
            reader.read_exact(&mut data)?;
            replay.datas.push(data);
        }
        Ok(replay)
    }
}

fn read_array<T: ReadBytesExt, const N: usize>(reader: &mut T) -> anyhow::Result<[u8; N]> {
    let mut arr = [0u8; N];
    for i in 0..N { arr[i] = reader.read_u8()?; }
    Ok(arr)
}

fn read_array_with_length<T: ReadBytesExt>(reader: &mut T) -> anyhow::Result<Vec<u32>> {
    let length = reader.read_u32::<LittleEndian>()?;
    let mut vec = Vec::new();
    for _ in 0..length as usize { vec.push(reader.read_u32::<LittleEndian>()?) }
    Ok(vec)
}

fn read_string<T: ReadBytesExt, const N: usize>(reader: &mut T) -> anyhow::Result<String> {
    let mut arr = [0u16; N];
    for i in 0..N { arr[i] = reader.read_u16::<LittleEndian>()?; }
    cast_to_string(&arr).ok_or(anyhow!("Cannot cast byte array to string"))
}

pub struct LFLists {
    lists: Vec<String>
}

impl LFLists {
    fn init(&mut self) -> std::io::Result<()> {
        let configuration = crate::srvpru::get_configuration();
        let file = std::fs::File::open(configuration.ygopro.lflist_conf.clone())?;
        for line in std::io::BufReader::new(file).lines() {
            let _line = line?;
            if _line.starts_with("!") {
                self.lists.push(_line[1..].to_string());
            }
        }
        Ok(())
    }

    pub fn first_tcg(&self) -> i32 {
        for (index, name) in self.lists.iter().enumerate() {
            if name.ends_with("TCG") {
                return index as i32;
            }
        }
        return -1;
    } 
}

lazy_static! {
    pub static ref LFLISTS: LFLists = {
        let mut lflists = LFLists { lists: Vec::new() };
        lflists.init().expect("Failed to initialize lflists");
        lflists
    };
}