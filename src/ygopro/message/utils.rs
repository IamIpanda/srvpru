use crate::ygopro::message::ctos;
use crate::ygopro::message::stoc;
use crate::ygopro::message::Struct;
use crate::ygopro::message::Direction;
use crate::ygopro::message::MessageType;

impl std::convert::Into<u8> for MessageType {
    fn into(self) -> u8 {
        match self {
            MessageType::STOC(message_type) => message_type.into(),
            MessageType::GM(_) => stoc::MessageType::GameMessage.into(),
            MessageType::CTOS(message_type) => message_type.into(),
            MessageType::SRVPRU(message_type) => message_type.into(),
        }
    }
}

pub fn try_get_message_type(direction: Direction, type_number: u8) -> Option<MessageType> {
    match direction {
        Direction::CTOS => ctos::MessageType::try_from(type_number).ok().map(|s| MessageType::CTOS(s)),
        Direction::STOC => stoc::MessageType::try_from(type_number).ok().map(|s| MessageType::STOC(s)),
        Direction::SRVPRU => None
    }
}

fn deserialize_struct<'a, T>(data: &'a [u8]) -> Option<Box<dyn Struct>> where T: serde::de::Deserialize<'a> + Struct {
    bincode::deserialize::<T>(data).ok().map(|data| Box::new(data) as Box<dyn Struct>)
}

pub fn deserialize_struct_by_type(direction: MessageType, data: &[u8]) -> Option<Box<dyn Struct>> {
    match direction {
        MessageType::CTOS(ctos_type) => {
            match ctos_type {
                // ctos::MessageType::Response     => deserialize_struct::<ctos::Response>(data),
                ctos::MessageType::UpdateDeck   => deserialize_struct::<ctos::UpdateDeck>(data),
                ctos::MessageType::HandResult   => deserialize_struct::<ctos::HandResult>(data),
                ctos::MessageType::TpResult     => deserialize_struct::<ctos::TpResult>(data),
                ctos::MessageType::PlayerInfo   => deserialize_struct::<ctos::PlayerInfo>(data),
                ctos::MessageType::CreateGame   => deserialize_struct::<ctos::CreateGame>(data),
                ctos::MessageType::JoinGame     => deserialize_struct::<ctos::JoinGame>(data),
                ctos::MessageType::LeaveGame    => deserialize_struct::<ctos::LeaveGame>(data),
                ctos::MessageType::Surrender    => deserialize_struct::<ctos::Surrender>(data),
                ctos::MessageType::TimeConfirm  => deserialize_struct::<ctos::TimeConfirm>(data),
                ctos::MessageType::Chat         => deserialize_struct::<ctos::Chat>(data),
                ctos::MessageType::HsTodueList  => deserialize_struct::<ctos::HsTodueList>(data),
                ctos::MessageType::HsToOBServer => deserialize_struct::<ctos::HsToOBServer>(data),
                ctos::MessageType::HsReady      => deserialize_struct::<ctos::HsReady>(data),
                ctos::MessageType::HsNotReady   => deserialize_struct::<ctos::HsNotReady>(data),
                ctos::MessageType::HsKick       => deserialize_struct::<ctos::HsKick>(data),
                ctos::MessageType::HsStart      => deserialize_struct::<ctos::HsStart>(data),
                ctos::MessageType::RequestField => deserialize_struct::<ctos::RequestField>(data),
                _ => Option::None
            }
        }
        MessageType::STOC(stoc_type) => {
            match stoc_type {
                stoc::MessageType::GameMessage    => deserialize_struct::<stoc::GameMessage>(data),
                stoc::MessageType::ErrorMessage   => deserialize_struct::<stoc::ErrorMessage>(data),
                stoc::MessageType::SelectHand     => deserialize_struct::<stoc::SelectHand>(data),
                stoc::MessageType::SelectTp       => deserialize_struct::<stoc::SelectTp>(data),
                stoc::MessageType::HandResult     => deserialize_struct::<stoc::HandResult>(data),
                stoc::MessageType::TpResult       => deserialize_struct::<stoc::TpResult>(data),
                stoc::MessageType::ChangeSide     => deserialize_struct::<stoc::ChangeSide>(data),
                stoc::MessageType::WaitingSide    => deserialize_struct::<stoc::WaitingSide>(data),
                stoc::MessageType::DeckCount      => deserialize_struct::<stoc::DeckCount>(data),
                stoc::MessageType::CreateGame     => deserialize_struct::<stoc::CreateGame>(data),
                stoc::MessageType::JoinGame       => deserialize_struct::<stoc::JoinGame>(data),
                stoc::MessageType::TypeChange     => deserialize_struct::<stoc::TypeChange>(data),
                stoc::MessageType::LeaveGame      => deserialize_struct::<stoc::LeaveGame>(data),
                stoc::MessageType::DuelStart      => deserialize_struct::<stoc::DuelStart>(data),
                stoc::MessageType::DuelEnd        => deserialize_struct::<stoc::DuelEnd>(data),
                stoc::MessageType::Replay         => deserialize_struct::<stoc::Replay>(data),
                stoc::MessageType::TimeLimit      => deserialize_struct::<stoc::TimeLimit>(data),
                stoc::MessageType::Chat           => deserialize_struct::<stoc::Chat>(data),
                stoc::MessageType::HsPlayerEnter  => deserialize_struct::<stoc::HsPlayerEnter>(data),
                stoc::MessageType::HsPlayerChange => deserialize_struct::<stoc::HsPlayerChange>(data),
                stoc::MessageType::HsWatchChange  => deserialize_struct::<stoc::HsWatchChange>(data),
                stoc::MessageType::FieldFinish    => deserialize_struct::<stoc::FieldFinish>(data),
                // _ => Option::None
            }
        }
        _ => { panic!("Try to deserialize an unreal message type.") }
    }
}

pub mod string {
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

    pub fn cast_to_c_array(message: &str) -> Vec<u16> {
        let mut vector: Vec<u16> = message.encode_utf16().collect();
        vector.push(0);
        vector
    }

    pub fn cast_to_fix_length_array<const N: usize>(message: &str) -> [u16; N] {
        let mut data = [0u16; N];
        for (index, chr) in message.encode_utf16().enumerate() {
            data[index] = chr;
        }
        data
    }
}

pub mod generate {
    use crate::ygopro::message::Struct;
    use crate::ygopro::message::MappedStruct;
    use crate::ygopro::message::MessageType;
    use crate::ygopro::message::srvpru;

    pub fn wrap_mapped_struct<T: Struct + MappedStruct + serde::Serialize>(data: &T) -> Vec<u8> {
        return wrap_data(T::message(), &bincode::serialize(&data).unwrap());
    }

    pub fn wrap_struct(message_type: MessageType, data: &(impl Struct + serde::Serialize)) -> Vec<u8> {
        return wrap_data(message_type, &bincode::serialize(&data).unwrap());
    }

    pub fn wrap_data(message_type: MessageType, data: &[u8]) -> Vec<u8> {
        if message_type == MessageType::SRVPRU(srvpru::MessageType::StructSequence) { return data.to_vec() };
        let size = data.len() + 1;
        let type_code: u8 = message_type.into();
        let mut message = vec!((size % 256) as u8, (size / 256) as u8, type_code);
        message.extend_from_slice(data);
        return message;
    }
}