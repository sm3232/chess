use crate::lib::piece::{Parity, PieceByte};

#[inline(always)]
pub fn byte_is_sided(byte: &u8) -> bool { byte.is_sided() }
#[inline(always)]
pub fn byte_is_kingside(byte: &u8) -> bool { byte.is_kingside() }
#[inline(always)]
pub fn byte_is_queenside(byte: &u8) -> bool { byte.is_queenside() }

pub trait ChessByte {
    fn get_parity(&self) -> Parity;
    fn get_piece(&self) -> PieceByte;
    fn get_directions(&self) -> u8;
    fn has_moved(&self) -> bool;
    fn is_kingside(&self) -> bool;
    fn is_queenside(&self) -> bool;
    fn is_king(&self) -> bool;
    fn is_w_king(&self) -> bool;
    fn is_b_king(&self) -> bool;
    fn is_rook(&self) -> bool;
    fn is_w_rook(&self) -> bool;
    fn is_b_rook(&self) -> bool;
    fn is_pawn(&self) -> bool;
    fn is_w_pawn(&self) -> bool;
    fn is_b_piece(&self) -> bool;
    fn is_w_piece(&self) -> bool;
    fn is_b_pawn(&self) -> bool;
    fn is_bishop(&self) -> bool;
    fn is_w_bishop(&self) -> bool;
    fn is_b_bishop(&self) -> bool;
    fn is_queen(&self) -> bool;
    fn is_w_queen(&self) -> bool;
    fn is_b_queen(&self) -> bool;
    fn is_knight(&self) -> bool;
    fn is_w_knight(&self) -> bool;
    fn is_b_knight(&self) -> bool;
    fn is_sided(&self) -> bool;
    fn to_letter(&self) -> char;
    fn is_castleable(&self, king_byte: u8, side_check: fn(&u8) -> bool) -> bool;
    fn is_white(&self) -> bool;
    fn is_black(&self) -> bool;
    fn is_piece(&self) -> bool;
    fn same_parity(&self, other: &u8) -> bool;
    fn same_piece(&self, other: &u8) -> bool;
    fn is_parity(&self, parity: Parity) -> bool;
}

impl ChessByte for u8 {
    #[inline(always)]
    fn is_w_piece(&self) -> bool { self.is_piece() && self.is_white() }
    #[inline(always)]
    fn is_b_piece(&self) -> bool { self.is_piece() && self.is_black() }
    #[inline(always)]
    fn is_w_king(&self) -> bool { self.is_white() && self.is_king() }
    #[inline(always)]
    fn is_b_king(&self) -> bool { self.is_black() && self.is_king() }
    #[inline(always)]
    fn is_w_rook(&self) -> bool { self.is_white() && self.is_rook() }
    #[inline(always)]
    fn is_b_rook(&self) -> bool { self.is_black() && self.is_rook() }
    #[inline(always)]
    fn is_w_pawn(&self) -> bool { self.is_white() && self.is_pawn() }
    #[inline(always)]
    fn is_b_pawn(&self) -> bool { self.is_black() && self.is_pawn() }
    #[inline(always)]
    fn is_w_queen(&self) -> bool { self.is_white() && self.is_queen() }
    #[inline(always)]
    fn is_b_queen(&self) -> bool { self.is_black() && self.is_queen() }
    #[inline(always)]
    fn is_w_bishop(&self) -> bool { self.is_white() && self.is_bishop() }
    #[inline(always)]
    fn is_b_bishop(&self) -> bool { self.is_black() && self.is_bishop() }
    #[inline(always)]
    fn is_w_knight(&self) -> bool { self.is_white() && self.is_knight() }
    #[inline(always)]
    fn is_b_knight(&self) -> bool { self.is_black() && self.is_knight() }

    #[inline(always)]
    fn is_white(&self) ->       bool { (self & 0b0000_1000) != 0 }
    
    #[inline(always)]
    fn is_black(&self) ->       bool { (self & 0b0000_0111) != 0 && (self & 0b0000_1000) == 0 }
    
    #[inline(always)]
    fn is_parity(&self, parity: Parity) -> bool { (self & 0b0000_1111) != 0 && self.get_parity() == parity }

    #[inline(always)]
    fn is_piece(&self) ->       bool { (self & 0b0000_0111) != 0 }
    
    #[inline(always)]
    fn has_moved(&self) ->      bool { (self & 0b1000_0000) != 0 }

    #[inline(always)]
    fn is_kingside(&self) ->    bool { (self & 0b0010_0111) != 0 }
    
    #[inline(always)]
    fn is_queenside(&self) ->   bool { (self & 0b0100_0111) != 0 }

    #[inline(always)]
    fn is_king(&self) ->        bool { (self & 0b0000_0111) == 0b0000_0101 }

    #[inline(always)]
    fn is_rook(&self) ->        bool { (self & 0b0000_0111) == 0b0000_0001 }

    #[inline(always)]
    fn is_pawn(&self) ->        bool { (self & 0b0000_0111) == 0b0000_0010 }

    #[inline(always)]
    fn is_bishop(&self) ->        bool { (self & 0b0000_0111) == 0b0000_0011 }

    #[inline(always)]
    fn is_queen(&self) ->        bool { (self & 0b0000_0111) == 0b0000_0100 }

    #[inline(always)]
    fn is_knight(&self) ->      bool { (self & 0b0000_0111) == 0b0000_0110 }
    
    #[inline(always)]
    fn get_piece(&self) ->      PieceByte { (self & 0b0000_0111).into() }
    
    #[inline(always)]
    fn is_sided(&self) ->       bool { (self & 0b01100000u8) != 0 }

    #[inline(always)]
    fn get_parity(&self) ->     Parity { if self.is_piece() { if self.is_white() { Parity::WHITE } else { Parity::BLACK } } else { Parity::NONE } }
    
    #[inline(always)]
    fn same_parity(&self, other: &u8) ->    bool { ((self ^ other) & 0b0000_1000) == 0 }

    #[inline(always)]
    fn same_piece(&self, other: &u8) ->     bool { ((self ^ other) & 0b0000_0111) == 0 }

    #[inline]
    fn is_castleable(&self, king_byte: u8, side_check: fn(&u8) -> bool) -> bool { ((self & 0b1000_0111) ^ 0b0000_0110) == 0b0000_0111 && side_check(self) && self.same_parity(&king_byte) }

    fn get_directions(&self) -> u8 {
        return match self.get_piece() {
            PieceByte::ROOK => 0b00001111,
            PieceByte::BISHOP => 0b11110000,
            PieceByte::QUEEN => 0b11111111,
            _ => 0b00000000
        };
    } 
    fn to_letter(&self) -> char {
        if self.is_piece() {
            let c = match self.get_piece() {
                PieceByte::KING => 'K',
                PieceByte::PAWN => 'P',
                PieceByte::ROOK => 'R',
                PieceByte::KNIGHT => 'N',
                PieceByte::QUEEN => 'Q',
                PieceByte::BISHOP => 'B',
                _ => ' '
            };
            return if self.is_white() { c } else { c.to_ascii_lowercase() };
        }
        return ' ';
    }
}
