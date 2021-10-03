#[derive(Copy, Clone, Eq, PartialEq)]
#[derive(serde::Serialize)]
#[repr(u8)]
pub enum MatchScore {
    NotStarted = -5,
    Dropped = -9,
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3
}

#[derive(serde::Serialize)]
pub struct MatchResult<'a> {
    name: &'a str,
    score: MatchScore,
    deck: &'a str
}