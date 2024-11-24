use crate::shared::piece::{Parity, PieceByte};

pub trait ChessByte {
    fn get_parity(&self) -> Parity;
    fn get_ptype(&self) -> PieceByte;
    fn get_directions(&self) -> u8;
    fn has_moved(&self) -> bool;
    fn is_kingside(&self) -> bool;
    fn is_queenside(&self) -> bool;
    fn is_sided(&self) -> bool;
    fn to_letter(&self) -> char;
}


impl ChessByte for u8 {
    fn to_letter(&self) -> char {
        if (self & 0b0000_0111) != 0 {
            let c = match self.get_ptype() {
                PieceByte::KING => 'K',
                PieceByte::PAWN => 'P',
                PieceByte::ROOK => 'R',
                PieceByte::KNIGHT => 'N',
                PieceByte::QUEEN => 'Q',
                PieceByte::BISHOP => 'B',
            };
            return if self.get_parity() == Parity::WHITE { c } else { c.to_ascii_lowercase() };
        } else {
            return ' ';
        }
    }
    fn get_parity(&self) -> Parity {
        if *self == 0 {
            return Parity::NONE;
        }
        return if (self & 0b00001000) != 0 { Parity::WHITE } else { Parity::BLACK };
    }
    fn get_ptype(&self) -> PieceByte {
        return (self & 0b00000111).into()
    }
    fn get_directions(&self) -> u8 {
        return match self.get_ptype() {
            PieceByte::ROOK => 0b00001111,
            PieceByte::BISHOP => 0b11110000,
            PieceByte::QUEEN => 0b11111111,
            _ => 0b00000000
        };
    } 
    fn has_moved(&self) -> bool {
        return (self & 0b10000000) != 0;
    }
    fn is_kingside(&self) -> bool {
        return (self & 0b00100000u8) != 0;
    }
    fn is_queenside(&self) -> bool {
        return (self & 0b01000000u8) != 0;
    }
    fn is_sided(&self) -> bool {
        return (self & 0b01100000u8) != 0;
    }
}
