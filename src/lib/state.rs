use std::sync::Mutex;
use std::{cell::RefCell, sync::Arc};
use std::rc::Rc;

use crate::lib::{
    cutil::pretty_print::{pretty_print_board, pretty_print_mask, pretty_print_moveset},
    boardarray::BoardArray, chessbyte::ChessByte, mask::Mask, maskset::MaskSet, piece::{ 
        Parity, PieceByte
    }, zobrist::Zobrist,
    motion::Motion
};

use super::searchtree::SearchTree;

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
    pub zobrist: Arc<Mutex<Zobrist>>,
    pub info: RetainedStateInfo,
    pub tree_root: Option<Arc<Mutex<SearchTree>>>,
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
            zobrist: Arc::new(Mutex::new(Zobrist::init())),
            tree_root: None,
            held_info: Vec::new(),
            info: RetainedStateInfo::default(),
            held_boards: Vec::new()
        }
    }
}


impl State {
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
                let zrist = self.zobrist.lock().unwrap();
                if do_turn_switch { self.turn = !self.turn };
                self.board.unmake(&argsboard, &argsinfo, &mut self.info);
                let cached_option = zrist.pull(self.info.zkey);
                drop(zrist);
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
        let zrist = self.zobrist.lock().unwrap();
        self.info.zkey = zrist.kof_board(self);
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
        drop(zrist);
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
        let mut zrist = self.zobrist.lock().unwrap();
        zrist.save((self.info.clone(), self.moves.clone()));
        drop(zrist);
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
}
