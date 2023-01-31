#![allow(non_upper_case_globals)]
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::io::Result;

use bitflags::bitflags;
use serde::Serialize;
use serde::Deserialize;
use serde::ser::SerializeSeq;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use lzma_rs::lzma_decompress_with_options;

use crate::utils::reader::read_array;
use crate::utils::reader::read_string;

use super::Deck;

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ReplayHeaderFlags: u32 {
        const Compressed = 1;
        const Tag = 2;
        const Decode = 4;
        const SingleMode = 8;
        const Uniform = 16;
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReplayHeader {
    pub id: u32,
    pub version: u32,
    pub flag: ReplayHeaderFlags,
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
    pub fn from_reader<T: ReadBytesExt>(reader: &mut T) -> Result<ReplayHeader> {
        Ok(ReplayHeader {
            id: reader.read_u32::<LittleEndian>()?,
            version: reader.read_u32::<LittleEndian>()?,
            flag: ReplayHeaderFlags::from_bits_truncate(reader.read_u32::<LittleEndian>()?),
            seed: reader.read_u32::<LittleEndian>()?,
            data_size: reader.read_u32::<LittleEndian>()?,
            start_time: reader.read_u32::<LittleEndian>()?,
            props: read_array(reader)?,
        })
    }

    pub fn is_compressed(&self) -> bool { self.flag.contains(ReplayHeaderFlags::Compressed) }
    pub fn is_tag(&self)        -> bool { self.flag.contains(ReplayHeaderFlags::Tag) }
    pub fn is_decoded(&self)    -> bool { self.flag.contains(ReplayHeaderFlags::Decode) }
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
    pub fn from_reader<T: ReadBytesExt + BufRead>(reader: &mut T) -> Result<Replay> {
        let header = ReplayHeader::from_reader(reader)?; 
        let leading_props = Cursor::new(&header.props[0..5]);
        let mut compressed_data = leading_props.chain(reader);
        let mut decompressed_vector = Vec::new();
        lzma_decompress_with_options(&mut compressed_data, &mut decompressed_vector, &lzma_rs::decompress::Options { 
            unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(header.data_size as u64)), 
            memlimit: None,
            allow_incomplete: false 
        }).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
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


impl Serialize for Replay {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.header)?;
        todo!()
        //seq.end()
    }
}

impl<'de> Deserialize<'de> for Replay {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error> where D: serde::Deserializer<'de> {
        todo!()
    }
}
