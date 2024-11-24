use crate::cutil::pretty_print::pretty_print_board;
use crate::shared::{
    boardarray::BoardArray,
    mask::Mask,
    piece::{ 
        PieceCachedMoves,
        PieceByte,
        Parity
    },
    chessbyte::ChessByte
};



pub struct MaskSet {
    pub ally: Mask,
    pub enemy: Mask,
    pub piece: Mask
}
impl Default for MaskSet {
    fn default() -> Self {
        Self {
            ally: Mask::default(),
            enemy: Mask::default(),
            piece: Mask::default()
        }
    }
}

pub struct State {
    pub board: [u8; 64],
    pub previous_board: [u8; 64],
    pub moves: [PieceCachedMoves; 64],
    pub turn: Parity,
    pub maskset: MaskSet,
    pub enpassant: Mask,
    pub threats: [PieceCachedMoves; 64],
    pub castles: u8,
    pub halfmove_clock: u64,
    pub fullmove_number: u64,
    pub branches: Vec<Box<State>>,
    pub is_accepted_state: bool,
    pub shallow_m: Mask,
    pub kings: (Mask, Mask)
}
impl Default for State {
    fn default() -> Self {
        Self {
            moves: [PieceCachedMoves::default(); 64],
            threats: [PieceCachedMoves::default(); 64],
            enpassant: Mask::default(),
            turn: Parity::NONE,
            board: [0u8; 64],
            previous_board: [0u8; 64],
            maskset: MaskSet::default(),
            castles: 0u8,
            fullmove_number: 0u64,
            halfmove_clock: 0u64,
            branches: Vec::new(),
            is_accepted_state: false,
            shallow_m: Mask::default(),
            kings: (Mask::default(), Mask::default())
        }
    }
}


impl State {
    #[inline(always)]
    pub fn white_kingside_can_castle(&self) -> bool {
        return (self.castles & 0b00000100) != 0;
    }
    #[inline(always)]
    pub fn white_queenside_can_castle(&self) -> bool {
        return (self.castles & 0b00001000) != 0;
    }
    #[inline(always)]
    pub fn black_kingside_can_castle(&self) -> bool {
        return (self.castles & 0b00000001) != 0;
    }
    #[inline(always)]
    pub fn black_queenside_can_castle(&self) -> bool {
        return (self.castles & 0b00000010) != 0;
    }
    #[inline(always)]
    pub fn get_piece_at_index(&self, index: usize) -> u8 {
        if index < 64 {
            return self.board[index];
        }
        return 0u8;
    }

    pub fn threatens(&self, index: usize) -> bool {
        let index_mask = Mask::from_index(index);
        let mut move_mask = Mask::default();
        for pcm in self.moves.iter() {
            move_mask |= pcm.moves;
        }
        if (move_mask & index_mask).any() {
            return true;
        }
        return false;
    }
    pub fn shallow(&mut self) {
        self.maskset = self.board.generate_maskset(self.turn);
        self.shallow_m = self.board.get_shallow_moves(self.turn, &self.maskset, &self.enpassant);
    }
    pub fn grow(&mut self) {
        self.maskset = self.board.generate_maskset(self.turn);
        (self.moves, self.shallow_m) = self.board.get_moves(self.turn, &self.maskset, &self.enpassant, self.castles);
        let mut cloned = self.moves;
        for (index, cache) in cloned.iter_mut().enumerate() {
            if cache.moves.any() {
                for &mut bit in cache.moves.isolated_bits().iter_mut() {
                    let mut branch = self.branch(Mask::from_index(index), bit);
                    branch.shallow();
                    if (branch.shallow_m & branch.kings.1).any() {
                        cache.moves ^= bit;
                    } else {
                        self.branches.push(Box::new(branch));
                    }
                }
            }
            if cache.castles != 0 && (self.shallow_m & self.kings.0).none() {
                if (cache.castles & 0b0000_0001) != 0 {
                    let mut branch = self.castle_branch(index, index + 3);
                    branch.shallow();
                    if (branch.shallow_m & Mask::from_index(index + 1)).none() && (branch.shallow_m & Mask::from_index(index + 2)).none() {
                        cache.castles &= 0b0000_0010;
                        self.branches.push(Box::new(branch));
                    }
                }
                if (cache.castles & 0b0000_0010) != 0 {
                    let mut branch = self.castle_branch(index, index - 4);
                    branch.shallow();
                    if (branch.shallow_m & Mask::from_index(index - 1)).none() && (branch.shallow_m & Mask::from_index(index - 2)).none() {
                        cache.castles &= 0b0000_0001;
                        self.branches.push(Box::new(branch));
                    }
                }
            }
        }
        self.moves = cloned;
    }

    pub fn try_accept(&mut self, from: usize, to: usize) -> usize {
        let potential = if self.board[from].get_parity() == self.board[to].get_parity() && self.board[from].get_ptype() == PieceByte::KING && self.board[to].get_ptype() == PieceByte::ROOK {
            self.board.with_castle(from, to)
        } else {
            self.board.with_move_indexed(from, to, &self.enpassant)
        };
        for (index, branch) in self.branches.iter_mut().enumerate() {
            if branch.board == potential {
                return index;
            }
        }
        return usize::MAX;
    }

    pub fn purge(&mut self, keep: usize) -> Box<Self> {
        let keep = self.branches.swap_remove(keep);
        self.branches.clear();
        return keep;
    }


    pub fn accept(&mut self) {
        self.is_accepted_state = true;
        self.grow();
        pretty_print_board(&self.board);
    }

    pub fn castle_branch(&mut self, king: usize, rook: usize) -> Self {
        let mut branch = State {
            is_accepted_state: false,
            turn: !self.turn,
            previous_board: self.board,
            halfmove_clock: 0,
            fullmove_number: if self.turn == Parity::BLACK { self.fullmove_number + 1 } else { self.fullmove_number },
            branches: Vec::new(),
            castles: self.castles & if self.turn == Parity::WHITE { 0b0000_0011 } else { 0b0000_1100 },
            enpassant: Mask::default(),
            ..Default::default()
        };
        branch.board = self.board.with_castle(king, rook);
        for (index, byte) in branch.board.iter().enumerate() {
            if byte.get_ptype() == PieceByte::KING {
                if byte.get_parity() == branch.turn {
                    branch.kings.0 = Mask::from_index(index);
                } else {
                    branch.kings.1 = Mask::from_index(index);
                }
            }
        }
        return branch;
    }
    pub fn branch(&mut self, from: Mask, to: Mask) -> Self {
        let piece_from = &self.board[from];
        let ptype_from = piece_from.get_ptype();
        let mut branch = State {
            is_accepted_state: false,
            turn: !self.turn,
            previous_board: self.board,
            halfmove_clock: if ptype_from == PieceByte::PAWN { self.halfmove_clock + 1 } else { 0 },
            fullmove_number: if self.turn == Parity::BLACK { self.fullmove_number + 1 } else { self.fullmove_number },
            branches: Vec::new(),
            castles: self.board.get_allowed_castles(from.as_index(), self.castles),
            ..Default::default()
        };
        if ptype_from == PieceByte::PAWN {
            if from.raw < to.raw {
                if from.raw & 0x000000000000FFFF != 0 {
                    if from.raw << 16 == to.raw {
                        branch.enpassant = Mask { raw: from.raw << 8 };
                    }
                }
            } else {
                if from.raw >> 16 == to.raw {
                    branch.enpassant = Mask { raw: from.raw >> 8 };
                }
            }
        }

        branch.board = self.board.with_move(from, to, &self.enpassant);
        for (index, byte) in branch.board.iter().enumerate() {
            if byte.get_ptype() == PieceByte::KING {
                if byte.get_parity() == branch.turn {
                    branch.kings.0 = Mask::from_index(index);
                } else {
                    branch.kings.1 = Mask::from_index(index);
                }
            }
        }
        return branch;
    }
}
