#![allow(dead_code)]
use serde_repr::Serialize_repr;
use serde_repr::Deserialize_repr;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u16)]
pub enum Network {
    ServerId = 29736,
    ClientId = 57078,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Netplayer {
    Player1 = 0,
    Player2 = 1,
    Player3 = 2,
    Player4 = 3,
    Player5 = 4,
    Player6 = 5,
    Observer = 7,
}

impl std::default::Default for Netplayer {
    fn default() -> Self {
        return Netplayer::Observer;
    }
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum PlayerChange {
    Observe = 8,
    Ready = 9,
    Notready = 10,
    Leave = 11,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum ErrorMessage {
    Joinerror = 1,
    Deckerror = 2,
    Sideerror = 3,
    Vererror = 4,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug, Hash)]
#[repr(u8)]
pub enum Mode {
    Single = 0,
    Match = 1,
    Tag = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Location {
    Deck = 1,
    Hand = 2,
    MZone = 4,
    SZone = 8,
    Grave = 16,
    Removed = 32,
    Extra = 64,
    Overlay = 128,
    OnField = 12,
    // FZone = 256,
    // PZone = 512,
    // DeckBot = 65537,
    // DeckShf = 131073,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Position {
    FaceupAttack = 1,
    FaceDownAttack = 2,
    FaceupDefense = 4,
    FacedownDefense = 8,
    Faceup = 5,
    Facedown = 10,
    Attack = 3,
    Defense = 12,
    // NoFlipEffect = 65536
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug, Hash)]
#[repr(u8)]
pub enum GameMessage {
    Retry = 1,
    Hint = 2,
    Waiting = 3,
    Start = 4,
    Win = 5,
    UpdateData = 6,
    UpdateCard = 7,
    RequestDeck = 8,
    SelectBattlecmd = 10,
    SelectIdlecmd = 11,
    SelectEffectyn = 12,
    SelectYesno = 13,
    SelectOption = 14,
    SelectCard = 15,
    SelectChain = 16,
    SelectPlace = 18,
    SelectPosition = 19,
    SelectTribute = 20,
    SortChain = 21,
    SelectCounter = 22,
    SelectSum = 23,
    SelectDisfield = 24,
    SortCard = 25,
    SelectUnselectCard = 26,
    ConfirmDecktop = 30,
    ConfirmCards = 31,
    ShuffleDeck = 32,
    ShuffleHand = 33,
    RefreshDeck = 34,
    SwapGraveDeck = 35,
    ShuffleSetCard = 36,
    ReverseDeck = 37,
    DeckTop = 38,
    MsgShuffleExtra = 39,
    NewTurn = 40,
    NewPhase = 41,
    ConfirmExtratop = 42,
    Move = 50,
    PosChange = 53,
    Set = 54,
    Swap = 55,
    FieldDisabled = 56,
    Summoning = 60,
    Summoned = 61,
    Spsummoning = 62,
    Spsummoned = 63,
    Flipsummoning = 64,
    Flipsummoned = 65,
    Chaining = 70,
    Chained = 71,
    ChainSolving = 72,
    ChainSolved = 73,
    ChainEnd = 74,
    ChainNegated = 75,
    ChainDisabled = 76,
    CardSelected = 80,
    RandomSelected = 81,
    BecomeTarget = 83,
    Draw = 90,
    Damage = 91,
    Recover = 92,
    Equip = 93,
    Lpupdate = 94,
    Unequip = 95,
    CardTarget = 96,
    CancelTarget = 97,
    PayLpcost = 100,
    AddCounter = 101,
    RemoveCounter = 102,
    Attack = 110,
    Battle = 111,
    AttackDisabled = 112,
    DamageStepStart = 113,
    DamageStepEnd = 114,
    MissedEffect = 120,
    BeChainTarget = 121,
    CreateRelation = 122,
    ReleaseRelation = 123,
    TossCoin = 130,
    TossDice = 131,
    RockPaperScissors = 132,
    HandRes = 133,
    AnnounceRace = 140,
    AnnounceAttrib = 141,
    AnnounceCard = 142,
    AnnounceNumber = 143,
    CardHint = 160,
    TagSwap = 161,
    ReloadField = 162,
    AiName = 163,
    ShowHint = 164,
    MatchKill = 170,
    CustomMsg = 180,
}

pub type GMMessageType = GameMessage;

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Timing {
    DrawPhase = 1,
    StandbyPhase = 2,
    MainEnd = 4,
    BattleStart = 8,
    BattleEnd = 16,
    EndPhase = 32,
    Summon = 64,
    Spsummon = 128,
    Flipsummon = 256,
    Mset = 512,
    Sset = 1024,
    PosChange = 2048,
    Attack = 4096,
    DamageStep = 8192,
    DamageCal = 16384,
    ChainEnd = 32768,
    Draw = 65536,
    Damage = 131072,
    Recover = 262144,
    Destroy = 524288,
    Remove = 1048576,
    Tohand = 2097152,
    Todeck = 4194304,
    Tograve = 8388608,
    BattlePhase = 16777216,
    Equip = 33554432,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Type {
    Monster = 1,
    Spell = 2,
    Trap = 4,
    Normal = 16,
    Effect = 32,
    Fusion = 64,
    Ritual = 128,
    Trapmonster = 256,
    Spirit = 512,
    Union = 1024,
    Dual = 2048,
    Tuner = 4096,
    Synchro = 8192,
    Token = 16384,
    Quickplay = 65536,
    Continuous = 131072,
    Equip = 262144,
    Field = 524288,
    Counter = 1048576,
    Flip = 2097152,
    Toon = 4194304,
    Xyz = 8388608,
    Pendulum = 16777216,
    Spsummon = 33554432,
    Link = 67108864,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Race {
    Warrior = 1,
    Spellcaster = 2,
    Fairy = 4,
    Fiend = 8,
    Zombie = 16,
    Machine = 32,
    Aqua = 64,
    Pyro = 128,
    Rock = 256,
    Windbeast = 512,
    Plant = 1024,
    Insect = 2048,
    Thunder = 4096,
    Dragon = 8192,
    Beast = 16384,
    Beastwarrior = 32768,
    Dinosaur = 65536,
    Fish = 131072,
    Seaserpent = 262144,
    Reptile = 524288,
    Psycho = 1048576,
    Devine = 2097152,
    Creatorgod = 4194304,
    Wyrm = 8388608,
    Cybers = 16777216,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Reason {
    Destroy = 0x1,
    Release = 0x2,
    Temporary = 0x4,
    Material = 0x8,
    Summon = 0x10,
    Battle = 0x20,
    Effect = 0x40,
    Cost = 0x80,
    Adjust = 0x100,
    LostTarget = 0x200,
    Rule = 0x400,
    Spsummon = 0x800,
    Dissummon = 0x1000,
    Flip = 0x2000,
    Discard = 0x4000,
    Rdamage = 0x8000,
    Rrecover = 0x10000,
    Return = 0x20000,
    Fusion = 0x40000,
    Synchro = 0x80000,
    Ritual = 0x100000,
    Xyz = 0x200000,
    Replace = 0x1000000,
    Draw = 0x2000000,
    Redirect = 0x4000000,
    Reveal = 0x8000000,
    Link = 0x10000000,
    LostOverlay = 0x20000000
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Status {
    Disabled = 0x0001,
    ToEnable = 0x0002,
    ToDisable = 0x0004,
    ProcComplete = 0x0008,
    SetTurn = 0x0010,
    NoLevel = 0x0020,
    BattleResult = 0x0040,
    SpsummonStep = 0x0080,
    FormChanged = 0x0100,
    Summoning = 0x0200,
    EffectEnabled = 0x0400,
    SummonTurn = 0x0800,
    DestroyConfirmed = 0x1000,
    LeaveConfirmed = 0x2000,
    BattleDestroyed = 0x4000,
    CopyingEffect = 0x8000,
    Chaining = 0x10000,
    SummonDisabled = 0x20000,
    ActivateDisabled = 0x40000,
    EffectReplaced = 0x80000,
    FutureFusion = 0x100000,
    AttackCanceled = 0x200000,
    Initializing = 0x400000,
    Activated = 0x800000,
    JustPos = 0x1000000,
    ContinuousPos = 0x2000000,
    Forbidden = 0x4000000,
    ActFromHand = 0x8000000,
    OppoBattle = 0x10000000,
    FlipSummonTurn = 0x20000000,
    SpsummonTurn = 0x40000000,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Query {
    Code = 0x1,
    Position = 0x2,
    Alias = 0x4,
    Type = 0x8,
    Level = 0x10,
    Rank = 0x20,
    Attribute = 0x40,
    Race = 0x80,
    Attack = 0x100,
    Defense = 0x200,
    BaseAttack = 0x400,
    BaseDefense = 0x800,
    Reason = 0x1000,
    ReasonCard = 0x2000,
    EquipCard = 0x4000,
    TargetCard = 0x8000,
    OverlayCard = 0x10000,
    Counters = 0x20000,
    Owner = 0x40000,
    Status = 0x80000,
    Lscale = 0x200000,
    Rscale = 0x400000,
    Link = 0x800000
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Attribute {
    Earth = 1,
    Water = 2,
    Fire = 4,
    Wind = 8,
    Light = 16,
    Dark = 32,
    Devine = 64,
}


#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Linkmarkers {
    BottomLeft = 1,
    Bottom = 2,
    BottomRight = 4,
    Left = 8,
    Right = 32,
    TopLeft = 64,
    Top = 128,
    TopRight = 256,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Duelstage {
    Begin = 0,
    Finger = 1,
    Firstgo = 2,
    Dueling = 3,
    Siding = 4,
    End = 5,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Colors {
    Observer = 7,
    Lightblue = 8,
    Red = 11,
    Green = 12,
    Blue = 13,
    Babyblue = 14,
    Pink = 15,
    Yellow = 16,
    White = 17,
    Gray = 18,
    Darkgray = 19,
}

impl std::default::Default for Colors {
    fn default() -> Self {
        return Colors::Observer;
    }
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Hint {
    Event = 1,
    Message = 2,
    SelectMessage = 3,
    Opselected = 4,
    Effect = 5,
    Race = 6,
    Attribite = 7,
    Code = 8,
    Number = 9,
    Card = 10,
    Zone = 11,
}

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Eq, PartialEq, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u32)]
pub enum Phase {
    Draw = 1,
    Standby = 2,
    Main1 = 4,
    BattleStart = 8,
    BattleStep = 16,
    Damage = 32,
    DamageCalculate = 64,
    Battle = 128,
    Main2 = 256,
    End = 512,
}
