use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::lib::cutil::pretty_print::pretty_print_masks;
use crate::lib::{
    cutil::pretty_print::pretty_print_board,
    boardarray::BoardArray, chessbyte::ChessByte, mask::Mask, maskset::MaskSet, piece::Parity,
    zobrist::Zobrist,
    motion::Motion
};

use super::cutil::pretty_print::pretty_print_maskset;
use super::motion::MotionSet;
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
pub struct PartialState {
    pub board: [u8; 64],
    pub allowed_castles: u8,
    pub enpassant_mask: Mask,
    pub maskset: MaskSet,
    pub king_indices: [usize; 2]
}


pub struct State {
    pub board: [u8; 64],
    pub cached_moves: HashMap<u64, MotionSet>,
    pub moves: MotionSet,
    pub turn: Parity,
    pub zobrist: Arc<Mutex<Zobrist>>,
    pub info: RetainedStateInfo,
    pub tree_root: Option<Arc<Mutex<SearchTree>>>,
    pub num_cached: usize,
    pub num_analyzed: usize,
    held_info: Vec<RetainedStateInfo>,
    held_boards: Vec<[u8; 64]>
}
pub const ARRAY_REPEAT_VALUE: Vec<Motion> = Vec::new();
impl Default for State {
    fn default() -> Self {
        Self {
            moves: MotionSet::default(),
            turn: Parity::NONE,
            board: [0u8; 64],
            zobrist: Arc::new(Mutex::new(Zobrist::init())),
            tree_root: None,
            held_info: Vec::new(),
            info: RetainedStateInfo::default(),
            held_boards: Vec::new(),
            num_analyzed: 0,
            num_cached: 0,
            cached_moves: HashMap::default()
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
        self.hydrate(debugging_enabled);
    }
    pub fn make_move(&mut self, from: usize, to: &Mask, debugging_enabled: bool) {
        let held = self.board.make(from, to.as_index(), self.zobrist.clone(), &mut self.info, debugging_enabled);
        self.held_boards.push(held.0);
        self.held_info.push(held.1);
        self.turn = !self.turn;
        self.hydrate(debugging_enabled);
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
                    self.hydrate(false);
                }
                return;
            }
        }
        panic!("No held state when expected!");
    }
    pub fn set_sorted_motions(&mut self, sorted: Vec<Motion>) -> () {
        if self.turn == Parity::WHITE {
            self.moves.white_vect = sorted;
        } else {
            self.moves.black_vect = sorted;
        }
        self.cached_moves.insert(self.info.zkey, self.moves.clone());
    }
    pub fn get_king(&self, parity: Parity) -> usize {
        return self.info.king_indices[if parity == Parity::WHITE { 0 } else { 1 }];
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
        self.hydrate(true);
    }
    pub fn partial_flipped(&self) -> PartialState {
        let flip = self.board.flipped();
        return PartialState {
            board: flip,
            enpassant_mask: self.info.enpassant_mask.flipped(),
            allowed_castles: {
                let low = self.info.allowed_castles & 0b0000_0011;
                let high = self.info.allowed_castles & 0b0000_1100;
                (low << 2) | (high >> 2)
            },
            maskset: MaskSet::from_board(&flip),
            king_indices: flip.get_kings()
        };
    }
    pub fn hydrate(&mut self, debug_log: bool){
        if debug_log {
            // pretty_print_board("Hydrating board", &self.board);
            // pretty_print_maskset("Maskset", &self.info.maskset);
        }
        if let Some(cached) = self.cached_moves.get(&self.info.zkey) {
            self.moves = cached.clone();
        } else {
            self.moves = self.board.get_motions(&self.info.maskset, &self.info.enpassant_mask, Some(self.info.allowed_castles));
            self.cached_moves.insert(self.info.zkey, self.moves.clone());
        }
        if debug_log {
            // pretty_print_masks("Flat moves", &vec![("White", &self.moves.white_flat), ("Black", &self.moves.black_flat)]);
        }
        let mut zrist = self.zobrist.lock().unwrap();
        zrist.save((self.info.clone(), self.moves.clone(), None));
        drop(zrist);
    }
}
