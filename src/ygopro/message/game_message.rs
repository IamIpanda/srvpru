use serde::Serialize;
use serde::Deserialize;
use serde::ser::Serializer;
use serde::ser::SerializeStruct;
use serde::de::Deserializer;
use serde::de::Visitor;
use serde::de::SeqAccess;

use crate::ygopro::Location;
use crate::ygopro::Netplayer;
use crate::ygopro::Position;
use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::Empty;

pub type MessageType = crate::ygopro::GameMessage;

#[derive(Debug)]
pub struct GameMessage {
    pub kind: MessageType,
    pub message: Box<dyn Struct>,
}

impl Struct for GameMessage {}
impl MappedStruct for GameMessage {
    fn message() -> crate::ygopro::message::MessageType {
        return crate::ygopro::message::MessageType::STOC(crate::ygopro::message::stoc::MessageType::GameMessage);
    }
}

impl Serialize for GameMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("GameMessage", 2)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("message", &*self.message)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GameMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Kind, Data }

        struct GameMessageVisitor;
        impl GameMessageVisitor {
            fn deserialize_message<'de, S: Struct + Deserialize<'de>, V: SeqAccess<'de>>(&self, seq: &mut V) -> Result<Option<Box<dyn Struct>>, V::Error> {
                Ok(seq.next_element::<S>()?.map(|v| Box::new(v) as Box<dyn Struct>))
            }
        }
        impl<'de> Visitor<'de> for GameMessageVisitor {
            type Value = GameMessage;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct GameMessage")
            }
            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error> where V: SeqAccess<'de> {
                let kind = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let message = match kind {
                       MessageType::ShuffleDeck        => self.deserialize_message::<ShuffleDeck,        _>(&mut seq)?,
                    // MessageType::Retry              => self.deserialize_message::<Retry,              _>(&mut seq)?,
                       MessageType::Hint               => self.deserialize_message::<Hint,               _>(&mut seq)?,
                    // MessageType::Waiting            => self.deserialize_message::<Waiting,            _>(&mut seq)?,
                       MessageType::Start              => self.deserialize_message::<Start,              _>(&mut seq)?,
                       MessageType::Win                => self.deserialize_message::<Win,                _>(&mut seq)?,
                    // MessageType::UpdateData         => self.deserialize_message::<UpdateData,         _>(&mut seq)?,
                    // MessageType::UpdateCard         => self.deserialize_message::<UpdateCard,         _>(&mut seq)?,
                    // MessageType::RequestDeck        => self.deserialize_message::<RequestDeck,        _>(&mut seq)?,
                    // MessageType::SelectBattlecmd    => self.deserialize_message::<SelectBattlecmd,    _>(&mut seq)?,
                    // MessageType::SelectIdlecmd      => self.deserialize_message::<SelectIdlecmd,      _>(&mut seq)?,
                    // MessageType::SelectEffectyn     => self.deserialize_message::<SelectEffectyn,     _>(&mut seq)?,
                    // MessageType::SelectYesno        => self.deserialize_message::<SelectYesno,        _>(&mut seq)?,
                    // MessageType::SelectOption       => self.deserialize_message::<SelectOption,       _>(&mut seq)?,
                    // MessageType::SelectCard         => self.deserialize_message::<SelectCard,         _>(&mut seq)?,
                    // MessageType::SelectChain        => self.deserialize_message::<SelectChain,        _>(&mut seq)?,
                    // MessageType::SelectPlace        => self.deserialize_message::<SelectPlace,        _>(&mut seq)?,
                    // MessageType::SelectPosition     => self.deserialize_message::<SelectPosition,     _>(&mut seq)?,
                    // MessageType::SelectTribute      => self.deserialize_message::<SelectTribute,      _>(&mut seq)?,
                    // MessageType::SortChain          => self.deserialize_message::<SortChain,          _>(&mut seq)?,
                    // MessageType::SelectCounter      => self.deserialize_message::<SelectCounter,      _>(&mut seq)?,
                    // MessageType::SelectSum          => self.deserialize_message::<SelectSum,          _>(&mut seq)?,
                    // MessageType::SelectDisfield     => self.deserialize_message::<SelectDisfield,     _>(&mut seq)?,
                    // MessageType::SortCard           => self.deserialize_message::<SortCard,           _>(&mut seq)?,
                    // MessageType::SelectUnselectCard => self.deserialize_message::<SelectUnselectCard, _>(&mut seq)?,
                    // MessageType::ConfirmDecktop     => self.deserialize_message::<ConfirmDecktop,     _>(&mut seq)?,
                    // MessageType::ConfirmCards       => self.deserialize_message::<ConfirmCards,       _>(&mut seq)?,
                    // MessageType::ShuffleHand        => self.deserialize_message::<ShuffleHand,        _>(&mut seq)?,
                    // MessageType::RefreshDeck        => self.deserialize_message::<RefreshDeck,        _>(&mut seq)?,
                    // MessageType::SwapGraveDeck      => self.deserialize_message::<SwapGraveDeck,      _>(&mut seq)?,
                    // MessageType::ShuffleSetCard     => self.deserialize_message::<ShuffleSetCard,     _>(&mut seq)?,
                    // MessageType::ReverseDeck        => self.deserialize_message::<ReverseDeck,        _>(&mut seq)?,
                    // MessageType::DeckTop            => self.deserialize_message::<DeckTop,            _>(&mut seq)?,
                    // MessageType::MsgShuffleExtra    => self.deserialize_message::<MsgShuffleExtra,    _>(&mut seq)?,
                       MessageType::NewTurn            => self.deserialize_message::<NewTurn,            _>(&mut seq)?,
                    // MessageType::NewPhase           => self.deserialize_message::<NewPhase,           _>(&mut seq)?,
                    // MessageType::ConfirmExtratop    => self.deserialize_message::<ConfirmExtratop,    _>(&mut seq)?,
                    // MessageType::Move               => self.deserialize_message::<Move,               _>(&mut seq)?,
                       MessageType::PosChange          => self.deserialize_message::<PosChange,          _>(&mut seq)?,
                    // MessageType::Set                => self.deserialize_message::<Set,                _>(&mut seq)?,
                    // MessageType::Swap               => self.deserialize_message::<Swap,               _>(&mut seq)?,
                    // MessageType::FieldDisabled      => self.deserialize_message::<FieldDisabled,      _>(&mut seq)?,
                       MessageType::Summoning          => self.deserialize_message::<Summoning,          _>(&mut seq)?,
                    // MessageType::Summoned           => self.deserialize_message::<Summoned,           _>(&mut seq)?,
                       MessageType::Spsummoning        => self.deserialize_message::<Spsummoning,        _>(&mut seq)?,
                    // MessageType::Spsummoned         => self.deserialize_message::<Spsummoned,         _>(&mut seq)?,
                    // MessageType::Flipsummoning      => self.deserialize_message::<Flipsummoning,      _>(&mut seq)?,
                    // MessageType::Flipsummoned       => self.deserialize_message::<Flipsummoned,       _>(&mut seq)?,
                       MessageType::Chaining           => self.deserialize_message::<Chaining,           _>(&mut seq)?,
                    // MessageType::Chained            => self.deserialize_message::<Chained,            _>(&mut seq)?,
                    // MessageType::ChainSolving       => self.deserialize_message::<ChainSolving,       _>(&mut seq)?,
                    // MessageType::ChainSolved        => self.deserialize_message::<ChainSolved,        _>(&mut seq)?,
                    // MessageType::ChainEnd           => self.deserialize_message::<ChainEnd,           _>(&mut seq)?,
                    // MessageType::ChainNegated       => self.deserialize_message::<ChainNegated,       _>(&mut seq)?,
                    // MessageType::ChainDisabled      => self.deserialize_message::<ChainDisabled,      _>(&mut seq)?,
                    // MessageType::CardSelected       => self.deserialize_message::<CardSelected,       _>(&mut seq)?,
                    // MessageType::RandomSelected     => self.deserialize_message::<RandomSelected,     _>(&mut seq)?,
                    // MessageType::BecomeTarget       => self.deserialize_message::<BecomeTarget,       _>(&mut seq)?,
                    // MessageType::Draw               => self.deserialize_message::<Draw,               _>(&mut seq)?,
                       MessageType::Damage             => self.deserialize_message::<Damage,             _>(&mut seq)?,
                       MessageType::Recover            => self.deserialize_message::<Recover,            _>(&mut seq)?,
                    // MessageType::Equip              => self.deserialize_message::<Equip,              _>(&mut seq)?,
                    // MessageType::Lpupdate           => self.deserialize_message::<Lpupdate,           _>(&mut seq)?,
                    // MessageType::Unequip            => self.deserialize_message::<Unequip,            _>(&mut seq)?,
                    // MessageType::CardTarget         => self.deserialize_message::<CardTarget,         _>(&mut seq)?,
                    // MessageType::CancelTarget       => self.deserialize_message::<CancelTarget,       _>(&mut seq)?,
                    // MessageType::PayLpcost          => self.deserialize_message::<PayLpcost,          _>(&mut seq)?,
                    // MessageType::AddCounter         => self.deserialize_message::<AddCounter,         _>(&mut seq)?,
                    // MessageType::RemoveCounter      => self.deserialize_message::<RemoveCounter,      _>(&mut seq)?,
                    // MessageType::Attack             => self.deserialize_message::<Attack,             _>(&mut seq)?,
                    // MessageType::Battle             => self.deserialize_message::<Battle,             _>(&mut seq)?,
                    // MessageType::AttackDisabled     => self.deserialize_message::<AttackDisabled,     _>(&mut seq)?,
                    // MessageType::DamageStepStart    => self.deserialize_message::<DamageStepStart,    _>(&mut seq)?,
                    // MessageType::DamageStepEnd      => self.deserialize_message::<DamageStepEnd,      _>(&mut seq)?,
                    // MessageType::MissedEffect       => self.deserialize_message::<MissedEffect,       _>(&mut seq)?,
                    // MessageType::BeChainTarget      => self.deserialize_message::<BeChainTarget,      _>(&mut seq)?,
                    // MessageType::CreateRelation     => self.deserialize_message::<CreateRelation,     _>(&mut seq)?,
                    // MessageType::ReleaseRelation    => self.deserialize_message::<ReleaseRelation,    _>(&mut seq)?,
                    // MessageType::TossCoin           => self.deserialize_message::<TossCoin,           _>(&mut seq)?,
                    // MessageType::TossDice           => self.deserialize_message::<TossDice,           _>(&mut seq)?,
                    // MessageType::RockPaperScissors  => self.deserialize_message::<RockPaperScissors,  _>(&mut seq)?,
                    // MessageType::HandRes            => self.deserialize_message::<HandRes,            _>(&mut seq)?,
                    // MessageType::AnnounceRace       => self.deserialize_message::<AnnounceRace,       _>(&mut seq)?,
                    // MessageType::AnnounceAttrib     => self.deserialize_message::<AnnounceAttrib,     _>(&mut seq)?,
                    // MessageType::AnnounceCard       => self.deserialize_message::<AnnounceCard,       _>(&mut seq)?,
                    // MessageType::AnnounceNumber     => self.deserialize_message::<AnnounceNumber,     _>(&mut seq)?,
                    // MessageType::CardHint           => self.deserialize_message::<CardHint,           _>(&mut seq)?,
                    // MessageType::TagSwap            => self.deserialize_message::<TagSwap,            _>(&mut seq)?,
                    // MessageType::ReloadField        => self.deserialize_message::<ReloadField,        _>(&mut seq)?,
                    // MessageType::AiName             => self.deserialize_message::<AiName,             _>(&mut seq)?,
                    // MessageType::ShowHint           => self.deserialize_message::<ShowHint,           _>(&mut seq)?,
                    // MessageType::MatchKill          => self.deserialize_message::<MatchKill,          _>(&mut seq)?,
                    // MessageType::CustomMsg          => self.deserialize_message::<CustomMsg,          _>(&mut seq)?,
                    _                               => self.deserialize_message::<Empty,         _>(&mut seq)?,
                }.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(GameMessage { kind, message })
            }
        }
        deserializer.deserialize_struct("STOCGameMessage", &["kind", "message"], GameMessageVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[gm]
pub struct ShuffleDeck {
    pub player: crate::ygopro::Netplayer 
}

#[derive(Serialize, Deserialize, Debug, Struct, Clone)]
pub struct Hint {
    pub _type: crate::ygopro::Hint,
    pub player: crate::ygopro::Netplayer,
    pub data: i32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[gm]
pub struct Win {
    pub winner: crate::ygopro::Netplayer,
    pub reason: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
// #[gm]
pub struct Start {
    pub _type: u8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct NewTurn {
    pub player: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct PosChange {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub previous_position: Position,
    pub current_position: Position
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Summoning {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub position: Position
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Spsummoning {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub position: Position
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Chaining {
    pub card: u32,
    pub previous_controller: Netplayer,
    pub previous_location: Location,
    pub previous_sequence: i8,
    pub sub_sequence: i8,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub desc: i32,
    pub target: i8
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Damage {
    pub player: Netplayer,
    pub value: i32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Recover {
    pub player: Netplayer,
    pub value: i32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct Lpupdate {
    pub player: Netplayer,
    pub lp: i32
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct PayLpcost {
    pub player: Netplayer,
    pub cost: i32
}

pub fn generate_message_type(_type: MessageType) -> crate::ygopro::message::MessageType {
    crate::ygopro::message::MessageType::GM(_type)
}
