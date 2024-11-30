
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
#[allow(dead_code)]
pub enum PieceByte {
    ROOK =      0b0000_0001,
    PAWN =      0b0000_0010,
    BISHOP =    0b0000_0011,
    QUEEN =     0b0000_0100,
    KING =      0b0000_0101,
    KNIGHT =    0b0000_0110,
    NONE =      0b0000_0000
}

impl From<u8> for PieceByte {
    fn from(value: u8) -> Self {
        return match value {
            0b0000_0001 => Self::ROOK,
            0b0000_0010 => Self::PAWN,
            0b0000_0011 => Self::BISHOP,
            0b0000_0100 => Self::QUEEN,
            0b0000_0101 => Self::KING,
            0b0000_0110 => Self::KNIGHT,
            _ => Self::NONE
        };
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum Parity {
    WHITE = 0b0000_1000u8,
    BLACK = 0b00000000u8,
    BOTH = 0b11111111u8,
    NONE = 0b01111111u8
}

impl Parity {
    pub fn at_depth(&self, depth: i32) -> Parity {
        let mut r = *self;
        for _ in 0..depth {
            r = !r;
        }
        return r;
    }
}

#[inline]
pub fn parity_to_string(parity: Parity) -> &'static str { if parity == Parity::WHITE { "WHITE" } else { "BLACK" } }

impl std::ops::BitAnd<Parity> for u8 {
    type Output = u8;
    #[inline(always)]
    fn bitand(self, rhs: Parity) -> Self::Output { self & (rhs as u8) }
}

impl std::ops::BitAnd<u8> for Parity {
    type Output = u8;
    #[inline(always)]
    fn bitand(self, rhs: u8) -> Self::Output { (self as u8) & rhs }
}

impl std::ops::BitAnd<PieceByte> for u8 {
    type Output = u8;
    #[inline(always)]
    fn bitand(self, rhs: PieceByte) -> Self::Output { self & (rhs as u8) }
}

impl std::ops::BitAnd<u8> for PieceByte {
    type Output = u8;
    #[inline(always)]
    fn bitand(self, rhs: u8) -> Self::Output { (self as u8) & rhs }
}

impl std::ops::BitOr<Parity> for PieceByte {
    type Output = u8;
    #[inline(always)]
    fn bitor(self, rhs: Parity) -> Self::Output { (self as u8) | (rhs as u8) }
}

impl std::ops::BitOr<PieceByte> for Parity {
    type Output = u8;
    #[inline(always)]
    fn bitor(self, rhs: PieceByte) -> Self::Output { (self as u8) | (rhs as u8) }
}

impl std::ops::Not for Parity {
    type Output = Self;
    #[inline]
    fn not(self) -> Self::Output {
        match self {
            Self::WHITE => Self::BLACK,
            Self::BLACK => Self::WHITE,
            Self::BOTH => Self::NONE,
            Self::NONE => Self::BOTH
        }
    }
}

impl PartialEq<bool> for Parity {
    #[inline(always)]
    fn eq(&self, other: &bool) -> bool { if *self == Parity::BLACK { *other == false } else { *other == true } }
}

impl PartialEq<Parity> for bool {
    #[inline(always)]
    fn eq(&self, other: &Parity) -> bool { if *other == Parity::BLACK { *self == false } else { *self == true } }
}

impl std::fmt::Display for PieceByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{:?}", self);
    }
}

impl std::fmt::Display for Parity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Parity::WHITE => write!(f, "White"),
            Parity::BLACK => write!(f, "Black"),
            Parity::BOTH => write!(f, "Both"),
            Parity::NONE => write!(f, "None")
        };
    }
}



/*
#[derive(Clone, Copy)]
pub struct Moves {
    pub moves: Mask,
    pub castles: u8,
}

pub fn print_moves_array(moves: &[Moves; 64]) -> () {
    let mut strg = "\n".to_owned();
    for i in 0..64 {
        if i % 8 == 0 && i != 0 {
            println!("{strg}");
            strg.clear();
        }
        strg.push(if moves[i].moves.any() { 'T' } else { 'F' });
    }
    println!("{strg}");
}

impl std::ops::Index<Point> for [Moves] {
    type Output = Moves;
    #[inline(always)]
    fn index(&self, index: Point) -> &Self::Output { &self[((index.y * 8) + index.x) as usize] }
}

impl std::ops::IndexMut<Point> for [Moves] {
    #[inline(always)]
    fn index_mut(&mut self, index: Point) -> &mut Self::Output { &mut self[((index.y * 8) + index.x) as usize] }
}

impl Default for Moves {
    fn default() -> Self { Moves { moves: Mask::default(), castles: 0u8 } }
}
*/
