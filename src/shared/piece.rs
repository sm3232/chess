use crate::shared::{point::Point, mask::Mask};



#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
#[allow(dead_code)]
pub enum PieceByte {
    ROOK =      0b00000001u8,
    PAWN =      0b00000010u8,
    BISHOP =    0b00000011u8,
    QUEEN =     0b00000100u8,
    KING =      0b00000101u8,
    KNIGHT =    0b00000110u8
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum Parity {
    WHITE = 0b00001000u8,
    BLACK = 0b00000000u8,
    BOTH = 0b11111111u8,
    NONE = 0b01111111u8
}
impl std::ops::BitOr<Parity> for PieceByte {
    type Output = u8;
    fn bitor(self, rhs: Parity) -> Self::Output {
        return (self as u8) | (rhs as u8);
    }
}
impl std::ops::BitOr<PieceByte> for Parity {
    type Output = u8;
    fn bitor(self, rhs: PieceByte) -> Self::Output {
        return (self as u8) | (rhs as u8);
    }
}

impl From<u8> for PieceByte {
    fn from(value: u8) -> Self {
        return match value {
            1 => Self::ROOK,
            2 => Self::PAWN,
            3 => Self::BISHOP,
            4 => Self::QUEEN,
            5 => Self::KING,
            6|7 => Self::KNIGHT,
            _ => Self::PAWN
        };
    }
}

impl std::ops::BitAnd<Parity> for u8 {
    type Output = u8;
    fn bitand(self, rhs: Parity) -> Self::Output {
        return self & (rhs as u8);
    }
}
impl std::ops::BitAnd<u8> for Parity {
    type Output = u8;
    fn bitand(self, rhs: u8) -> Self::Output {
        return (self as u8) & rhs;
    }
}
impl std::ops::BitAnd<PieceByte> for u8 {
    type Output = u8;
    fn bitand(self, rhs: PieceByte) -> Self::Output {
        return self & (rhs as u8);
    }
}
impl std::ops::BitAnd<u8> for PieceByte {
    type Output = u8;
    fn bitand(self, rhs: u8) -> Self::Output {
        return (self as u8) & rhs;
    }
}

impl std::ops::Not for Parity {
    type Output = Self;
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
    fn eq(&self, other: &bool) -> bool {
        if *self == Parity::BLACK {
            return *other == false;
        } else {
            return *other == true;
        }
    }
}
impl PartialEq<Parity> for bool {
    fn eq(&self, other: &Parity) -> bool {
        if *other == Parity::BLACK {
            return *self == false;
        } else {
            return *self == true;
        }
    }
}
impl Into<bool> for Parity {
    fn into(self) -> bool {
        return self == Parity::WHITE;
    }
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

pub fn parity_to_string(parity: Parity) -> &'static str {
    if parity == Parity::WHITE { return "WHITE" } else { return "BLACK" };
}

#[derive(Clone, Copy)]
pub struct PieceCachedMoves {
    pub moves: Mask,
    pub state: Mask,
    pub castles: u8
}
impl std::ops::IndexMut<Point> for [PieceCachedMoves] {
    fn index_mut(&mut self, index: Point) -> &mut Self::Output {
        return &mut self[((index.y * 8) + index.x) as usize];
    }
}
impl std::ops::Index<Point> for [PieceCachedMoves] {
    type Output = PieceCachedMoves;
    fn index(&self, index: Point) -> &Self::Output {
        return &self[((index.y * 8) + index.x) as usize];
    }

}
impl Default for PieceCachedMoves {
    fn default() -> Self {
        return PieceCachedMoves {
            moves: Mask::default(),
            state: Mask::default(),
            castles: 0u8
        };
    }
}
