use serde::Serialize;
use serde::Deserialize;
use serde::de::VariantAccess;
use srvpru_proc_macros::Message;

use crate::constants::LocalPlayer;
use crate::constants::Location;
use crate::constants::Netplayer;
use crate::constants::Position;
use crate::data::Deck;

use super::Message;
use super::utils::build_it;

include!(concat!(env!("OUT_DIR"), "/game_message.rs"));

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 1)]
#[repr(C)]
pub struct Retry {
    pub last_msg: u8
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[message(gm, flag = 2)]
#[repr(C)]
pub struct Hint {
    pub _type: crate::constants::Hint,
    pub player: Netplayer,
    pub data: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 3)]
#[repr(C)]
pub struct Waiting;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 4)]
#[repr(C)]
pub struct Start {
    pub plyaer_type: u8,
    pub rule: i8,
    pub lp1: i32,
    pub lp2: i32,
    pub deck: Deck
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 5)]
#[repr(C)]
pub struct Win {
    pub winner: Netplayer,
    pub reason: u8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 6)]
#[repr(C)]
pub struct UpdateData {
    pub player: Netplayer,
    pub location: Location
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 7)]
#[repr(C)]
pub struct UpdateCard {
    pub player: Netplayer,
    pub location: Location,
    pub sequence: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 8)]
#[repr(C)]
pub struct RequestDeck;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 10)]
#[repr(C)]
pub struct SelectBattleCommand {
    pub selecting_player: Netplayer
    // todo: Vec here
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 11)]
#[repr(C)]
pub struct SelectIdleCommand {
    pub selecting_player: Netplayer,
    // todo: vec here
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 12)]
#[repr(C)]
pub struct SelectEffectYesNo {
    pub selecting_player: Netplayer,
    pub card_position: CardPosition,
    pub unknown: i8,
    pub description: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 13)]
#[repr(C)]
pub struct SelectYesNo {
    pub selecting_player: Netplayer,
    pub description: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 14)]
#[repr(C)]
pub struct SelectOption {
    pub selecting_player: Netplayer,
    pub options: Vec<i32>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 15)]
#[repr(C)]
pub struct SelectCard {
    pub selecting_player: Netplayer,
    pub select_cancelable: bool,
    pub select_min: i8,
    pub select_max: i8,
    pub count: i8,
    pub positions: Vec<CardPosition2>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 16)]
#[repr(C)]
pub struct SelectChain {
    pub selecting_player: Netplayer,
    pub count: i8,
    pub spec_count: i8,
    pub forced: i8,
    pub hint0: i32,
    pub hint1: i32,
    // count-length vec
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 18)]
#[repr(C)]
pub struct SelectPlace {
    pub selecting_player: Netplayer,
    pub count: i8,
    pub selectzble_field: i32,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 19)]
#[repr(C)]
pub struct SelectPosition {
    pub selecting_player: Netplayer,
    pub code: u32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 20)]
#[repr(C)]
pub struct SelectTribute {
    pub selecting_player: Netplayer,
    pub cancelable: bool,
    pub select_min: i8,
    pub select_max: i8,
    pub tributes: Vec<CardPosition2> // Here subsequence should be tribute
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 21)]
#[repr(C)]
pub struct SortChain;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 22)]
#[repr(C)]
pub struct SelectCounter {
    pub selecting_player: Netplayer,
    pub select_counter_type: i16,
    pub select_counter_count: i16,
    // Vec<CardPosition_Vec, i16>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 23)]
#[repr(C)]
pub struct SelectSum {
    pub select_mode: i8,
    pub selecting_player: Netplayer,
    pub select_sum_value: i32,
    pub select_min: i8,
    pub select_max: i8,
    pub must_select_count: i8,
    // Vec MustSelectCount, code + CardPosition
    // Vec SelectCount, code + CardPosition
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 24)]
#[repr(C)]
pub struct SelectDisableField {
    pub selecting_player: Netplayer,
    pub count: i8,
    pub selectzble_field: i32,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 25)]
#[repr(C)]
pub struct SortCard {
    pub player: Netplayer,
    pub cards: Vec<(i32, CardPosition)>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 26)]
#[repr(C)]
pub struct SelectUnselectCard {
    pub selecting_playuer: Netplayer,
    pub finishable: bool,
    pub cancelable: bool,
    pub select_min: i8,
    pub select_max: i8,
    pub positions1: Vec<CardPosition2>,
    pub positions2: Vec<CardPosition2>
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Message)]
#[message(gm, flag = 30)]
#[repr(C)]
pub struct ConfirmCard {
    pub code: i32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 31)]
#[repr(C)]
pub struct ConfirmCards {
    pub player: Netplayer,
    pub count: i8,
    pub cards: Vec<ConfirmCard>
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 32)]
#[repr(C)]
pub struct ShuffleDeck {
    pub player: Netplayer 
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 33)]
#[repr(C)]
pub struct ShuffleHand;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 34)]
#[repr(C)]
pub struct RefreshDeck;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 35)]
#[repr(C)]
pub struct SwapGrave_deck;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 36)]
#[repr(C)]
pub struct ShuffleSet_card;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 37)]
#[repr(C)]
pub struct ReverseDeck;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 38)]
#[repr(C)]
pub struct DeckTop;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 39)]
#[repr(C)]
pub struct ShuffleExtra;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 40)]
#[repr(C)]
pub struct NewTurn {
    pub player: Netplayer
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 41)]
#[repr(C)]
pub struct NewPhase;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 42)]
#[repr(C)]
pub struct ConfirmExtraTop;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 50)]
#[repr(C)]
pub struct Move;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 53)]
#[repr(C)]
pub struct PosChange {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub previous_position: Position,
    pub current_position: Position
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 54)]
#[repr(C)]
pub struct Set;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 55)]
#[repr(C)]
pub struct Swap;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 56)]
#[repr(C)]
pub struct FieldDisabled;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 60)]
#[repr(C)]
pub struct Summoning {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub position: Position
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 61)]
#[repr(C)]
pub struct Summoned;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 62)]
#[repr(C)]
pub struct Spsummoning {
    pub card: u32,
    pub controller: Netplayer,
    pub location: Location,
    pub sequence: i8,
    pub position: Position
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 63)]
#[repr(C)]
pub struct Spsummoned;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 64)]
#[repr(C)]
pub struct Flipsummoning;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 65)]
#[repr(C)]
pub struct Flipsummoned;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 70)]
#[repr(C)]
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

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 71)]
#[repr(C)]
pub struct Chained {
    pub chain_index: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 72)]
#[repr(C)]
pub struct ChainSolving {
    pub chain_index: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 73)]
#[repr(C)]
pub struct ChainSolved {
    pub chain_index: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 74)]
#[repr(C)]
pub struct ChainEnd;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 75)]
#[repr(C)]
pub struct ChainNegated {
    pub chain_index: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 76)]
#[repr(C)]
pub struct ChainDisabled {
    pub chain_index: i8
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 80)]
#[repr(C)]
pub struct CardSelected;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 81)]
#[repr(C)]
pub struct RandomSelected;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 83)]
#[repr(C)]
pub struct BecomeTarget;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 91)]
#[repr(C)]
pub struct Damage {
    pub player: Netplayer,
    pub value: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 92)]
#[repr(C)]
pub struct Recover {
    pub player: Netplayer,
    pub value: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 93)]
#[repr(C)]
pub struct Equip;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 94)]
#[repr(C)]
pub struct Lpupdate {
    pub player: Netplayer,
    pub lp: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 95)]
#[repr(C)]
pub struct Unequip;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 96)]
#[repr(C)]
pub struct CardTarget;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 97)]
#[repr(C)]
pub struct CancelTarget;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 100)]
#[repr(C)]
pub struct PayLpcost {
    pub player: Netplayer,
    pub cost: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 101)]
#[repr(C)]
pub struct AddCounter;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 102)]
#[repr(C)]
pub struct RemoveCounter;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 110)]
#[repr(C)]
pub struct Attack;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 111)]
#[repr(C)]
pub struct Battle;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 112)]
#[repr(C)]
pub struct AttackDisabled;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 113)]
#[repr(C)]
pub struct DamageStep_start;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 114)]
#[repr(C)]
pub struct DamageStep_end;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 120)]
#[repr(C)]
pub struct MissedEffect;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 121)]
#[repr(C)]
pub struct BeChain_target;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 122)]
#[repr(C)]
pub struct CreateRelation;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 123)]
#[repr(C)]
pub struct ReleaseRelation;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 130)]
#[repr(C)]
pub struct TossCoin;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 131)]
#[repr(C)]
pub struct TossDice;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 132)]
#[repr(C)]
pub struct RockPaper_scissors;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 133)]
#[repr(C)]
pub struct HandRes;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 140)]
#[repr(C)]
pub struct AnnounceRace;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 141)]
#[repr(C)]
pub struct AnnounceAttrib;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 142)]
#[repr(C)]
pub struct AnnounceCard;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 143)]
#[repr(C)]
pub struct AnnounceNumber;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 160)]
#[repr(C)]
pub struct CardHint;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 161)]
#[repr(C)]
pub struct TagSwap;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 162)]
#[repr(C)]
pub struct ReloadField;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 163)]
#[repr(C)]
pub struct AiName;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 164)]
#[repr(C)]
pub struct ShowHint;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 165)]
#[repr(C)]
pub struct PlayerHint;

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 170)]
#[repr(C)]
pub struct MatchKill {
    pub reason: i32
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[message(gm, flag = 180)]
#[repr(C)]
pub struct CustomMsg;


#[derive(Serialize, Deserialize, Debug)]
pub struct CardPosition {
    pub controller: LocalPlayer,
    pub location: Location,
    pub sequence: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CardPosition2 {
    pub code: u32,
    pub controller: LocalPlayer,
    pub location: Location,
    pub sequence: i32,
    pub sub_sequence: i32
}
