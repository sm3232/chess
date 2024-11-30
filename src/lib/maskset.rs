use crate::lib::{
    chessbyte::ChessByte,
    mask::Mask
};

#[derive(Clone, Copy)]
pub struct MaskSet {
    pub white: Mask,
    pub black: Mask,
    pub all: Mask
}
impl Default for MaskSet {
    fn default() -> Self {
        Self {
            white: Mask::default(),
            black: Mask::default(),
            all: Mask::default()
        }
    }
}

impl MaskSet {
    pub fn from_board(board: &[u8; 64]) -> Self {
        let mut ms = Self::default();
        for (index, byte) in board.iter().enumerate() {
            if byte.is_white() {
                ms.white |= Mask::from_index(index);
            } else if byte.is_black() {
                ms.black |= Mask::from_index(index);
            }
        }
        ms.all |= ms.white | ms.black;
        return ms;
    }

}
