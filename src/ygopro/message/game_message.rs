use serde::ser::SerializeStruct;
use crate::ygopro::constants::GMMessageType;

#[derive(Debug, Struct)]
pub struct STOCGameMessage {
    pub kind: GMMessageType,
    pub message: Box<dyn Struct>,
}

impl Serialize for STOCGameMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("GameMessage", 2)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("message", &*self.message)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for STOCGameMessage {
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
            type Value = STOCGameMessage;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct GameMessage")
            }
            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error> where V: SeqAccess<'de> {
                let kind = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let message = match kind {
                    GMMessageType::ShuffleDeck        => self.deserialize_message::<GMShuffleDeck, _>(&mut seq)?,
                    GMMessageType::Retry              => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Hint               => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Waiting            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Start              => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Win                => self.deserialize_message::<GMWin,         _>(&mut seq)?,
                    GMMessageType::UpdateData         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::UpdateCard         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::RequestDeck        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectBattlecmd    => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectIdlecmd      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectEffectyn     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectYesno        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectOption       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectCard         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectChain        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectPlace        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectPosition     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectTribute      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SortChain          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectCounter      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectSum          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectDisfield     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SortCard           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SelectUnselectCard => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ConfirmDecktop     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ConfirmCards       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ShuffleHand        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::RefreshDeck        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::SwapGraveDeck      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ShuffleSetCard     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ReverseDeck        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::DeckTop            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::MsgShuffleExtra    => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::NewTurn            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::NewPhase           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ConfirmExtratop    => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Move               => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::PosChange          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Set                => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Swap               => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::FieldDisabled      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Summoning          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Summoned           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Spsummoning        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Spsummoned         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Flipsummoning      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Flipsummoned       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Chaining           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Chained            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ChainSolving       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ChainSolved        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ChainEnd           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ChainNegated       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ChainDisabled      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CardSelected       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::RandomSelected     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::BecomeTarget       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Draw               => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Damage             => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Recover            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Equip              => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Lpupdate           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Unequip            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CardTarget         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CancelTarget       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::PayLpcost          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AddCounter         => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::RemoveCounter      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Attack             => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::Battle             => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AttackDisabled     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::DamageStepStart    => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::DamageStepEnd      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::MissedEffect       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::BeChainTarget      => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CreateRelation     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ReleaseRelation    => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::TossCoin           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::TossDice           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::RockPaperScissors  => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::HandRes            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AnnounceRace       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AnnounceAttrib     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AnnounceCard       => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AnnounceNumber     => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CardHint           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::TagSwap            => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ReloadField        => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::AiName             => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::ShowHint           => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::MatchKill          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                    GMMessageType::CustomMsg          => self.deserialize_message::<Empty,         _>(&mut seq)?,
                }.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(STOCGameMessage { kind, message })
            }
        }
        deserializer.deserialize_struct("STOCGameMessage", &["kind", "message"], GameMessageVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct GMShuffleDeck {
    pub player: crate::ygopro::constants::Netplayer 
}

#[derive(Serialize, Deserialize, Debug, Struct)]
pub struct GMWin {
    pub winner: crate::ygopro::constants::Netplayer,
    pub reason: u8
}
