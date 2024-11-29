use std::cell::RefCell;
use std::rc::Rc;

use crate::{cutil::pretty_print::{pretty_print_board, pretty_print_mask, pretty_print_moveset}, shared::{
    boardarray::BoardArray, chessbyte::ChessByte, mask::Mask, maskset::MaskSet, piece::{ 
        Parity, PieceByte
    }, zobrist::Zobrist,
    motion::Motion
}};


pub struct RetainedStateInfo {
    pub zkey: u64,
    pub allowed_castles: u8,
    pub enpassant_mask: Mask,
    pub king_indices: [usize; 2],
    pub maskset: MaskSet,
    pub halfmove_clock: u64,
    pub fullmove_number: u64,
}
impl Default for RetainedStateInfo {
    fn default() -> Self { Self { 
        king_indices: [65usize; 2],
        enpassant_mask: Mask::default(),
        allowed_castles: 0u8,
        zkey: 0u64,
        maskset: MaskSet::default(),
        fullmove_number: 0u64,
        halfmove_clock: 0u64
    } }
}
impl Clone for RetainedStateInfo {
    fn clone(&self) -> Self {
        return Self {
            allowed_castles: self.allowed_castles.clone(),
            enpassant_mask: self.enpassant_mask.clone(),
            zkey: self.zkey.clone(),
            king_indices: self.king_indices.clone(),
            maskset: self.maskset,
            halfmove_clock: self.halfmove_clock.clone(),
            fullmove_number: self.fullmove_number.clone()
        };
    }
}


pub struct State {
    pub board: [u8; 64],
    pub moves: [Vec<Motion>; 64],
    pub turn: Parity,
    pub zobrist: Rc<RefCell<Zobrist>>,
    pub info: RetainedStateInfo,
    held_info: Vec<RetainedStateInfo>,
    held_boards: Vec<[u8; 64]>
}
pub const ARRAY_REPEAT_VALUE: Vec<Motion> = Vec::new();
impl Default for State {
    fn default() -> Self {
        Self {
            moves: [ARRAY_REPEAT_VALUE; 64],
            turn: Parity::NONE,
            board: [0u8; 64],
            zobrist: Rc::new(RefCell::new(Zobrist::init())),
            held_info: Vec::new(),
            info: RetainedStateInfo::default(),
            held_boards: Vec::new()
        }
    }
}


impl State {
    #[inline(always)]
    pub fn white_kingside_can_castle(&self) -> bool { (self.info.allowed_castles & 0b00000100) != 0 }
    #[inline(always)]
    pub fn white_queenside_can_castle(&self) -> bool { (self.info.allowed_castles & 0b00001000) != 0 }
    #[inline(always)]
    pub fn black_kingside_can_castle(&self) -> bool { (self.info.allowed_castles & 0b00000001) != 0 }
    #[inline(always)]
    pub fn black_queenside_can_castle(&self) -> bool { (self.info.allowed_castles & 0b00000010) != 0 }
    #[inline(always)]
    pub fn get_piece_at_index(&self, index: usize) -> u8 { return if index < 64 { self.board[index] } else { 0u8 } }

    pub fn make_motion(&mut self, motion: &Motion, debugging_enabled: bool) {
        let held = self.board.make(motion.from, motion.to, self.zobrist.clone(), &mut self.info, debugging_enabled);
        self.held_boards.push(held.0);
        self.held_info.push(held.1);
        self.turn = !self.turn;
        self.hydrate();
    }
    pub fn make_move(&mut self, from: usize, to: &Mask, debugging_enabled: bool) {
        let held = self.board.make(from, to.as_index(), self.zobrist.clone(), &mut self.info, debugging_enabled);
        self.held_boards.push(held.0);
        self.held_info.push(held.1);
        self.turn = !self.turn;
        self.hydrate();
    }
    pub fn unmake_last(&mut self, do_turn_switch: bool) {
        if let Some(argsinfo) = self.held_info.pop() {
            if let Some(argsboard) = self.held_boards.pop() {
                if do_turn_switch { self.turn = !self.turn };
                self.board.unmake(&argsboard, &argsinfo, &mut self.info);
                let cached_option = self.zobrist.borrow().pull(self.info.zkey);
                if let Some(cached) = cached_option {
                    self.moves = cached.1;
                } else {
                    self.hydrate();
                }
                return;
            }
        }
        panic!("No held state when expected!");
    }
    pub fn init(&mut self) {
        self.info.zkey = self.zobrist.borrow_mut().kof_board(self);
        self.info.maskset = MaskSet::from_board(&self.board);

        for i in 0..64 {
            if self.board[i].is_king() {
                if self.board[i].is_white() {
                    self.info.king_indices[0] = i;
                } else if self.board[i].is_black() {
                    self.info.king_indices[1] = i;
                }
            }
        }
        if self.info.king_indices[0] == 65 || self.info.king_indices[1] == 65 {
            panic!("Could not find kings in board! Attempted white index: {}, attempted black index: {}", self.info.king_indices[0], self.info.king_indices[1]);
        }
        self.hydrate();
    }
    pub fn threatened_by_enemy(&self, mask: &Mask) -> bool {
        return (*mask & self.board.get_moves_shallow_ipd(!self.turn, &self.info)).any();
    }
    pub fn king_in_check(&self, kp: Parity) -> bool {
        let mut kpi = 65;
        for i in 0..64 {
            if self.board[i].get_parity() == kp && self.board[i].is_king() {
                kpi = i;
                break;
            }
        }
        return kpi == 65 || (Mask::from_index(kpi) & self.board.get_moves_shallow_ipd(!kp, &self.info)).any();
    }
    pub fn hydrate(&mut self){
        self.moves = self.board.get_moves(self.turn, &self.info);
        let myking = self.info.king_indices[if self.turn == Parity::WHITE { 0 } else { 1 }];
        if myking == 65 {
            self.moves = [ARRAY_REPEAT_VALUE; 64];
            return;
        }

        let kingmoves = &self.moves[myking];
        if !kingmoves.is_empty() {
            let parity_castles = self.info.allowed_castles & if myking == 60 { 0b0000_1100 } else { 0b0000_0011 };
            if parity_castles & 0b0000_0101 != 0 && myking + 2 < 64 {
                let mask = Mask::from_index(myking + 1) | Mask::from_index(myking + 2);
                if (mask & self.info.maskset.all).none() && !self.threatened_by_enemy(&(Mask::from_index(myking) | mask)) {
                    self.moves[myking].push(Motion { from: myking, to: if myking == 4 { 7 } else { 63 } });
                }
            }
            if parity_castles & 0b0000_1010 != 0 && myking > 2 {
                let mask = Mask::from_index(myking - 1) | Mask::from_index(myking - 2);
                if ((mask | Mask::from_index(myking - 3)) & self.info.maskset.all).none() && !self.threatened_by_enemy(&(Mask::from_index(myking) | mask)) {
                    self.moves[myking].push(Motion { from: myking, to: if myking == 4 { 0 } else { 56 } });
                }
            }
        }

        for i in 0..64 {
            if self.moves[i].is_empty() { continue };
            self.moves[i].retain(|m| {
                let held = self.board.make(m.from, m.to, self.zobrist.clone(), &mut self.info, false);
                let was_in_check = self.board.index_in_check(self.info.king_indices[if self.turn == Parity::WHITE { 0 } else { 1 }], self.turn, &self.info);
                self.board.unmake(&held.0, &held.1, &mut self.info);
                return !was_in_check;
            });
        }
        self.zobrist.borrow_mut().save((self.info.clone(), self.moves.clone()));
    }
    pub fn flipped_moves(&self) -> [Vec<Motion>; 64] {
        let mut array: [Vec<Motion>; 64] = [ARRAY_REPEAT_VALUE; 64];
        for y in 0..8 {
            for x in 0..8 {
                array[(7 - y) * 8 + x] = self.moves[y * 8 + x].to_vec();
            }
        }
        return array;
    }
    /*
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
    pub fn soft_moves(&mut self) {
        if self.xrays.is_none() {
            self.xrays = Some(self.board.get_xrays(self.turn, &self.maskset, &self.enpassant, self.allowed_castles));
        }
        let xrays = &self.xrays.unwrap();
        let allymask = if self.turn == Parity::WHITE { self.maskset.white } else { self.maskset.black };
        let notally = allymask.get_not();
        let allybits = allymask.raw;
        for i in 0..64 {
            if self.board[i].is_parity(self.turn) {
                if self.board[i].get_piece() == PieceByte::ROOK {
                    let mut px = i % 8;
                    let mut py = i / 8;
                    let mut minx = 0;
                    let mut maxx = 7;
                    let mut miny = 0;
                    let mut maxy = 7;
                    for x in 0..8 {
                        if x == px { continue };
                        if self.board[i - px + x].is_piece() && self.board[i - px + x].is_parity(self.turn) {
                            if x < px { minx = x } else { maxx = x };
                        }
                    }
                    for y in 0..8 {
                        if y == py { continue };
                        if self.board[i - 8 * (py + y)].is_piece() && self.board[i - 8 * (py + y)].is_parity(self.turn) {
                            if y < py { miny = y } else { maxy = y };
                        }
                    }
                    xrays[i].raw = xrays[i].raw.wrapping_shr(((8 - maxy) * 8 + 8 - maxx) as u32) << 8 - maxy * 8 + 8 - maxx;
                    xrays[i].raw = xrays[i].raw.wrapping_shl((miny * 8 + minx) as u32) >> miny * 8 + minx;
                }
            }
        }
    }
    pub fn grow(&mut self) {
        self.maskset = MaskSet::from_board(&self.board);
        (self.moves, self.shallow_m) = self.board.get_moves(self.turn, &self.maskset, &self.enpassant, self.allowed_castles);
        let mut cloned = self.moves;
        for (index, cache) in cloned.iter_mut().enumerate() {
            if cache.moves.any() {
                for &mut bit in cache.moves.isolated_bits().iter_mut() {
                    let mut branch = self.branch(Mask::from_index(index), bit);
                    
                    branch.shallow();
                    if (branch.shallow_m & branch.kings.1).any() {
                        cache.moves ^= bit;
                    } else {
                        self.branches.push(Rc::new(RefCell::new(branch)));
                    }
                }
            }
            if cache.castles != 0 && (self.shallow_m & self.kings.0).none() {
                if (cache.castles & 0b0000_0001) != 0 {
                    let mut branch = self.castle_branch(index, index + 3);
                    branch.shallow();
                    if (branch.shallow_m & Mask::from_index(index + 1)).none() && (branch.shallow_m & Mask::from_index(index + 2)).none() {
                        cache.castles &= 0b0000_0010;
                        self.branches.push(Rc::new(RefCell::new(branch)));
                    }
                }
                if (cache.castles & 0b0000_0010) != 0 {
                    let mut branch = self.castle_branch(index, index - 4);
                    branch.shallow();
                    if (branch.shallow_m & Mask::from_index(index - 1)).none() && (branch.shallow_m & Mask::from_index(index - 2)).none() {
                        cache.castles &= 0b0000_0001;
                        self.branches.push(Rc::new(RefCell::new(branch)));
                    }
                }
            }
        }
        self.moves = cloned;
    }


    pub fn try_accept_via_board(&mut self, board: &[u8; 64]) -> usize {
        for (index, branch) in self.branches.iter_mut().enumerate() {
            if branch.borrow().board == *board {
                return index;
            }
        }
        return usize::MAX;
    }
    pub fn try_accept(&mut self, from: usize, to: usize) -> usize {
        let potential = if self.board[from].get_parity() == self.board[to].get_parity() && self.board[from].get_ptype() == PieceByte::KING && self.board[to].get_ptype() == PieceByte::ROOK {
            self.board.with_castle(from, to)
        } else {
            self.board.with_move_indexed(from, to, &self.enpassant)
        };
        for (index, branch) in self.branches.iter_mut().enumerate() {
            if branch.borrow().board == potential {
                return index;
            }
        }
        return usize::MAX;
    }
    pub fn purge(&mut self, keep: usize) -> Rc<RefCell<Self>> {
        let keep = self.branches.swap_remove(keep);
        self.branches.clear();
        return keep;
    }

    pub fn accept(&mut self) {
        self.is_accepted_state = true;
        self.grow();
    }
    pub fn unaccept(&mut self) {
        self.is_accepted_state = false;
    }
    pub fn castle_branch(&mut self, king: usize, rook: usize) -> Self {
        let mut branch = State {
            turn: !self.turn,
            halfmove_clock: 0,
            fullmove_number: if self.turn == Parity::BLACK { self.fullmove_number + 1 } else { self.fullmove_number },
            branches: Vec::new(),
            castles: self.allowed_castles & if self.turn == Parity::WHITE { 0b0000_0011 } else { 0b0000_1100 },
            enpassant: Mask::default(),
            ..Default::default()
        };
        branch.board = self.board.with_castle(king, rook);
        for (index, byte) in branch.board.iter().enumerate() {
            if byte.get_piece() == PieceByte::KING {
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
        let ptype_from = piece_from.get_piece();
        let mut branch = State {
            turn: !self.turn,
            halfmove_clock: if ptype_from == PieceByte::PAWN { self.halfmove_clock + 1 } else { 0 },
            fullmove_number: if self.turn == Parity::BLACK { self.fullmove_number + 1 } else { self.fullmove_number },
            branches: Vec::new(),
            castles: self.board.get_allowed_castles(from.as_index(), self.allowed_castles),
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
            if byte.get_piece() == PieceByte::KING {
                if byte.get_parity() == branch.turn {
                    branch.kings.0 = Mask::from_index(index);
                } else {
                    branch.kings.1 = Mask::from_index(index);
                }
            }
        }
        return branch;
    }
    */
}
