use std::{cell::RefCell, rc::Rc, collections::HashMap};

use rand_chacha::{rand_core::{RngCore, SeedableRng}, ChaCha8Rng};

use super::{chessbyte::ChessByte, motion::Motion, piece::{Parity, PieceByte}, state::{RetainedStateInfo, State}};

pub struct Zobrist {
    pub zpieces: [[u64; 12]; 64],
    pub zcastles: [u64; 16],
    pub zpassant: [u64; 9],
    pub zside: u64,
    table: HashMap<u64, (RetainedStateInfo, [Vec<Motion>; 64])>
}

impl Zobrist {
    pub fn init() -> Self {
        let seed = 123;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut zob = Zobrist {
            zpieces: [[0u64; 12]; 64],
            zcastles: [0u64; 16],
            zpassant: [0u64; 9],
            zside: rng.next_u64(),
            table: HashMap::new()
        };
        for i in 0..64 {
            for p in 0..12 {
                zob.zpieces[i][p] = rng.next_u64();
            }
        }
        for i in 0..16 {
            zob.zcastles[i] = rng.next_u64();
        }
        zob.zpassant[0] = 0;
        for i in 1..9 {
            zob.zpassant[i] = rng.next_u64();
        }
        return zob;
    }
    pub fn pieces(&self, byte: &u8, index: usize) -> u64 {
        return self.zpieces[index][Self::index_from_byte(&(byte & 0b0000_1111))];
    }
    pub fn index_from_byte(byte: &u8) -> usize {
        return if byte.is_white() { 6 } else { 0 } + match byte.get_piece() {
            PieceByte::ROOK => 0,
            PieceByte::PAWN => 1,
            PieceByte::BISHOP => 2,
            PieceByte::QUEEN => 3,
            PieceByte::KING => 4,
            PieceByte::KNIGHT => 5,
            _ => 0
        };
    }
    pub fn kof_board(&self, state: &State) -> u64 {
        let mut k = 0u64;
        let board = &state.board;
        for i in 0..64 {
            if board[i].is_piece() {
                k ^= self.zpieces[i][Zobrist::index_from_byte(&board[i])];
            }
        }
        k ^= self.zpassant[state.info.enpassant_mask.as_index() % 8];
        if state.turn == Parity::BLACK {
            k ^= self.zside;
        }
        k ^= self.zcastles[state.info.allowed_castles as usize];
        return k;
    }

    pub fn save(&mut self, stuff: (RetainedStateInfo, [Vec<Motion>; 64])) -> () {
        self.table.insert(stuff.0.zkey, stuff);
    }

    pub fn pull(&self, zkey: u64) -> Option<(RetainedStateInfo, [Vec<Motion>; 64])> {
        return self.table.get(&zkey).cloned();
    }
}
