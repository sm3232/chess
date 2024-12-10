use std::sync::{Arc, Mutex};

use crate::lib::{
    chessbyte::ChessByte, mask::Mask, maskset::MaskSet, piece::{
        Parity, PieceByte
    }, point::Point
};

use super::{motion::{Motion, MotionSet}, state::RetainedStateInfo, zobrist::Zobrist};

const BISHOP_DIRS: [Point; 4] = [ Point {x: -1, y: -1 }, Point { x: -1, y: 1 }, Point { x: 1, y: 1 }, Point { x: 1, y: -1 } ];
const ROOK_DIRS: [Point; 4] = [ Point { x: 0, y: 1 }, Point { x: 0, y: -1 }, Point { x: 1, y: 0 }, Point { x: -1, y: 0 } ];

fn bishop_move(bishop_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> (Mask, Mask) {
    let mut hits = 0b0000_0000;
    let mut move_mask = Mask::default();
    let mut defense_mask = Mask::default();
    let pos_pos = Point::from_index(bishop_index);
    for i in 1..=8 {
        for k in 0..4 {
            if hits & (1 << k) == 0 {
                let desire = pos_pos + BISHOP_DIRS[k] * i;
                if desire.valid() {
                    let desire_mask = Mask::from_point(desire);
                    if (*piece_mask & desire_mask).any() {
                        if (*enemy_mask & desire_mask).any() {
                            move_mask |= desire_mask;
                        } else {
                            defense_mask |= desire_mask;
                        }
                        hits |= 1 << k;
                    } else {
                        move_mask |= desire_mask;
                    }
                } else {
                    hits |= 1 << k;
                }
            }
        }
    }
    return (move_mask, defense_mask);
}
fn rook_move(rook_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> (Mask, Mask) {
    let mut hits = 0b0000_0000;
    let mut move_mask = Mask::default();
    let mut defense_mask = Mask::default();
    let pos_pos = Point::from_index(rook_index);
    for i in 1..=8 {
        for k in 0..4 {
            if hits & (1 << k) == 0 {
                let desire = pos_pos + ROOK_DIRS[k] * i;
                if desire.valid() {
                    let desire_mask = Mask::from_point(desire);
                    if (*piece_mask & desire_mask).any() {
                        if (*enemy_mask & desire_mask).any() {
                            move_mask |= desire_mask;
                        } else {
                            defense_mask |= desire_mask;
                        }
                        hits |= 1 << k;
                    } else {
                        move_mask |= desire_mask;
                    }
                } else {
                    hits |= 1 << k;
                }
            }
        }
    }
    return (move_mask, defense_mask);
}
#[inline]
fn queen_move(queen_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> (Mask, Mask) {
    let r = rook_move(queen_index, enemy_mask, piece_mask);
    let b = bishop_move(queen_index, enemy_mask, piece_mask);
    return (r.0 | b.0, r.1 | b.1);
}
fn pawn_move(pawn_index: usize, enemy_mask: &Mask, piece_mask: &Mask, parity: Parity, ignore_diagonal_enemy_requirement: bool, enpassant: &Mask) -> (Mask, Mask) {
    let mut move_mask = Mask::default();
    let mut defense_mask = Mask::default();
    if pawn_index < 8 || pawn_index > 55 {
        return (move_mask, defense_mask);
    }
    let pos_mask = Mask::from_index(pawn_index);
    let basic = Mask { raw: if parity == Parity::WHITE { pos_mask.raw >> 8 } else { pos_mask.raw << 8 } };
    if (*piece_mask & basic).none() {
        move_mask |= basic;
        if pawn_index / 8 == 1 || pawn_index / 8 == 6 {
            let dbl = Mask { raw: if parity == Parity::WHITE { basic.raw >> 8 } else { basic.raw << 8 } };
            if dbl.any() && (*piece_mask & dbl).none() {
                move_mask |= dbl;
            }
        }
    }
    let pos_pos = Point::from_index(pawn_index);
    let ydir = if basic.raw > pos_mask.raw { 1 } else { -1 };

    let diag1 = pos_pos + Point { x: 1, y: ydir };
    if diag1.valid() {
        if (*piece_mask & diag1).any() && (*enemy_mask & diag1).none() {
            defense_mask |= diag1;
        }
        if (*enemy_mask & diag1).any() || ignore_diagonal_enemy_requirement {
            move_mask |= diag1;
        } else if (*piece_mask & *enpassant).none() && (*enpassant & diag1).any() {
            move_mask |= diag1;
        }
    }
    let diag2 = pos_pos + Point { x: -1, y: ydir };
    if diag2.valid() {
        if (*piece_mask & diag2).any() && (*enemy_mask & diag2).none() {
            defense_mask |= diag2;
        }
        if (*enemy_mask & diag2).any() || ignore_diagonal_enemy_requirement {
            move_mask |= diag2;
        } else if (*piece_mask & *enpassant).none() && (*enpassant & diag2).any() {
            move_mask |= diag2;
        }
    }
    return (move_mask, defense_mask);
}
fn knight_move(knight_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> (Mask, Mask) {
    let mut move_mask = Mask::default();
    let pos_pos = Point::from_index(knight_index);
    let offsets = [
        Point { x: -2, y: -1 }, 
        Point { x: -2, y: 1 }, 
        Point { x: -1, y: -2 }, 
        Point { x: -1, y: 2 }, 
        Point { x: 1, y: -2 }, 
        Point { x: 1, y: 2 }, 
        Point { x: 2, y: -1 }, 
        Point { x: 2, y: 1 }
    ];
    for offset in offsets {
        let pv = pos_pos + offset;
        if pv.valid() {
            move_mask |= Mask::from_point(pv);
        }
    }
    return (move_mask & (enemy_mask.get_not() & *piece_mask).get_not(), move_mask & *piece_mask & enemy_mask.get_not());
    
}
fn king_move(king_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> (Mask, Mask) {
    let mut move_mask = Mask::default();
    let mut defense_mask = Mask::default();
    let mut maxx = 2i32;
    let mut minx = -1i32;
    let mut maxy = 2i32;
    let mut miny = -1i32;
    let kim8 = king_index % 8;
    let kid8 = king_index / 8;
    if kim8 == 7 { maxx -= 1 };
    if kim8 == 0 { minx += 1 };
    if kid8 == 7 { maxy -= 1 };
    if kid8 == 0 { miny += 1 };

    for y in miny..maxy {
        for x in minx..maxx {
            if x == 0 && y == 0 { continue };
            let thispos = Mask::from_index((king_index as i32 + y * 8 + x) as usize);
            if (*piece_mask & thispos).any() {
                if (*enemy_mask & thispos).any() {
                    move_mask |= thispos;
                } else {
                    defense_mask |= thispos;
                }
            } else {
                move_mask |= thispos;
            }
        }
    }

    return (move_mask, defense_mask);
}

impl std::ops::Index<Mask> for [u8; 64] {
    type Output = u8;
    fn index(&self, index: Mask) -> &Self::Output {
        return &self[index.as_index()];
    }
}
impl std::ops::IndexMut<Mask> for [u8; 64] {
    fn index_mut(&mut self, index: Mask) -> &mut Self::Output {
        return &mut self[index.as_index()];
    }
}
impl std::ops::IndexMut<Point> for [u8; 64] {
    fn index_mut(&mut self, index: Point) -> &mut Self::Output {
        return &mut self[((index.y * 8) + index.x) as usize];
    }
}
impl std::ops::Index<Point> for [u8; 64] {
    type Output = u8;
    fn index(&self, index: Point) -> &Self::Output {
        return &self[((index.y * 8) + index.x) as usize];
    }
}
pub enum MoveType {
    ENPASSANT,
    PROMOTION,
    CASTLE,
    TAKE,
    MOVE
}
pub trait BoardArray {
    fn flipped(&self) -> Self;
    fn make(&mut self, from: usize, to: usize, zobrist: Arc<Mutex<Zobrist>>, current_info: &mut RetainedStateInfo, debugging_enabled: bool) -> ([u8; 64], RetainedStateInfo);
    fn make_soft(&mut self, from: usize, to: usize) -> [u8; 64]; 
    fn unmake(&mut self, original_board: &[u8; 64], original_info: &RetainedStateInfo, current_info: &mut RetainedStateInfo) -> ();
    fn index_in_check(&self, index: usize, ip: Parity, info: &RetainedStateInfo) -> bool;
    fn get_motions(&self, maskset: &MaskSet, enpassant: &Mask, castles: Option<u8>) -> MotionSet;
    fn get_specific_motions(&self, ally_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> Mask;
    fn king_in_check(&self, king_index: usize, maskset: &MaskSet, enpassant: &Mask) -> bool;
    fn get_kings(&self) -> [usize; 2];
}
impl BoardArray for [u8; 64] {
    #[cfg(not(feature = "use_asm"))]
    fn get_kings(&self) -> [usize; 2] {
        let mut kings = [65usize; 2];
        for i in 0..64 {
            if self[i].is_w_king() {
                kings[0] = i;
            } else if self[i].is_b_king() {
                kings[1] = i;
            }
        }
        return kings;
    }
    #[cfg(feature = "use_asm")]
    fn get_kings(&self) -> [usize; 2] {
        use std::arch::asm;
        let mut array = [65usize; 2];
        unsafe {
            asm!(
                "mov rcx, 64",
                // Start of loop
                "5:",
                
                // Check for 0b0000_1101
                "movzx r8, byte ptr [rax]",
                "and r8b, 0x0F",   // Mask lower 4 bits
                "cmp r8b, 0x0D",   // Compare with 0b0000_1101
                "je 2f",           // Jump if equal (first condition)
                
                // Check for 0b0000_0101
                "movzx r8, byte ptr [rax]",
                "and r8b, 0x0F",   // Mask lower 4 bits
                "cmp r8b, 0x05",   // Compare with 0b0000_0101
                "je 3f",           // Jump if equal (second condition)
                
                // Continue loop
                "4:",
                "inc rax",         // Move to next byte
                "loop 5b",
                "jmp 6f",          // Continue loop if not zero
                
                // Found first match (0b0000_1101)
                "2:",
                "mov r9, rcx",
                "jmp 4b",          // Continue loop
                
                // Found second match (0b0000_0101)
                "3:",
                "mov r10, rcx",
                "jmp 4b",          // Continue loop
                
                // Store results
                "6:",
                "mov [rsi], r9",   // Store first index
                "mov [rsi+8], r10", // Store second index
            
                in("rax") self.as_ptr(),
                in("rsi") array.as_mut_ptr(),
                lateout("rax") _,
                lateout("rcx") _,
                lateout("rsi") _,
                options(nostack)
            );
            return array;
        }
    }

    fn index_in_check(&self, index: usize, ip: Parity, info: &RetainedStateInfo) -> bool {
        return (Mask::from_index(index) & self.get_specific_motions(!ip, &info.maskset, &info.enpassant_mask)).any();
    }
    fn king_in_check(&self, king_index: usize, maskset: &MaskSet, enpassant: &Mask) -> bool {
        return (Mask::from_index(king_index) & self.get_specific_motions(!self[king_index].get_parity(), maskset, enpassant)).any();
    }
    fn unmake(&mut self, original_board: &[u8; 64], original_info: &RetainedStateInfo, current_info: &mut RetainedStateInfo) -> () {
        *self = *original_board;
        *current_info = RetainedStateInfo {
            maskset: MaskSet {
                all: original_info.maskset.all,
                white: original_info.maskset.white,
                black: original_info.maskset.black
            },
            king_indices: original_info.king_indices,
            halfmove_clock: original_info.halfmove_clock,
            fullmove_number: original_info.fullmove_number,
            zkey: original_info.zkey,
            allowed_castles: original_info.allowed_castles,
            enpassant_mask: original_info.enpassant_mask
        };
    }


    fn make_soft(&mut self, from: usize, to: usize) -> [u8; 64] {
        let og = self.clone();
        let is_enpassant = self[from].is_pawn() && !self[to].is_piece() && from % 8 != to % 8;
        let is_promotion = self[from].is_pawn() && (to < 8 || to > 55);
        let is_castle = self[to].is_piece() && self[from].same_parity(&self[to]) && self[from].is_king() && self[to].is_rook();
        let is_take = self[to].is_piece() && !self[to].same_parity(&self[from]);
        if is_enpassant {
            if from > to { self[to + 8] = 0 } else { self[to - 8] = 0 };
            self.swap(from, to);
            self[to] |= 0b1000_0000;
        } else if is_promotion {
            self.swap(from, to);
            self[to] ^= 0b0000_0110;
            self[to] |= 0b1000_0000;
            self[from] = 0;
        } else if is_castle {
            if from > to {
                self.swap(from, from - 2);
                self.swap(to, from - 1);
                self[from - 2] |= 0b1000_0000;
                self[from - 1] |= 0b1000_0000;
            } else {
                self.swap(from, from + 2);
                self.swap(to, from + 1);
                self[from + 2] |= 0b1000_0000;
                self[from + 1] |= 0b1000_0000;
            }
        } else if is_take {
            self.swap(from, to);
            self[from] = 0;
            self[to] |= 0b1000_0000;
        } else {
            self.swap(from, to);
            self[to] |= 0b1000_0000;
            self[from] = 0;
        }
        return og;
    }
    fn make(&mut self, from: usize, to: usize, zobrist: Arc<Mutex<Zobrist>>, current_info: &mut RetainedStateInfo, debugging_enabled: bool) -> ([u8; 64], RetainedStateInfo) {
        let original_info = current_info.clone();

        let original_board = self.clone();
        if from == 65 && to == 65 {
            if debugging_enabled {
                println!("Null move");
            }
            current_info.enpassant_mask = Mask::default();
            return (original_board, original_info);
        }
        if !self[from].is_piece() { 
            println!("Cannot make move. No from piece at index {from}! Returning original board");
            return (original_board, original_info);
        }
        let is_enpassant = self[from].is_pawn() && !self[to].is_piece() && from % 8 != to % 8;
        current_info.enpassant_mask = Mask::default();
        if self[from].is_pawn() && from.abs_diff(to) > 15 {
            let ep = Mask::from_index(to);
            current_info.enpassant_mask = if from > to { Mask { raw: ep.raw << 8 } } else { Mask { raw: ep.raw >> 8 } };
        }
        let is_promotion = self[from].is_pawn() && (to < 8 || to > 55);
        let is_castle = self[to].is_piece() && self[from].same_parity(&self[to]) && self[from].is_king() && self[to].is_rook();
        let is_take = self[to].is_piece() && !self[to].same_parity(&self[from]);
        if from == 60 || to == 60 {
            current_info.allowed_castles &= 0b0000_0011;
        } else if from == 4 || to == 4 {
            current_info.allowed_castles &= 0b0000_1100;
        }
        if from == 56 || to == 56 {
            current_info.allowed_castles &= 0b0000_0111;
        } else if from == 63 || to == 63 {
            current_info.allowed_castles &= 0b0000_1011;
        }
        if from == 0 || to == 0 {
            current_info.allowed_castles &= 0b0000_1101;
        } else if from == 7 || to == 7 {
            current_info.allowed_castles &= 0b0000_1110;
        }
        let zrist = &zobrist.lock().unwrap();
        if is_enpassant {
            if debugging_enabled {
                println!("Move {from} -> {to} is an enpassant.");
            }
            current_info.zkey ^= zrist.pieces(&(PieceByte::PAWN | self[from].get_parity()), from);
            current_info.zkey ^= zrist.pieces(&(PieceByte::PAWN | self[from].get_parity()), to);
            if from > to {
                current_info.zkey ^= zrist.pieces(&(PieceByte::PAWN | !self[from].get_parity()), to + 8);
                self[to + 8] = 0;
            } else {
                current_info.zkey ^= zrist.pieces(&(PieceByte::PAWN | !self[from].get_parity()), to - 8);
                self[to - 8] = 0;
            }
            self.swap(from, to);
            self[to] |= 0b1000_0000;
        } else if is_promotion {
            if debugging_enabled {
                println!("Move {from} -> {to} is a promotion.");
            }

            current_info.zkey ^= zrist.pieces(&(PieceByte::PAWN | self[from].get_parity()), from);
            current_info.zkey ^= zrist.pieces(&(PieceByte::QUEEN | self[from].get_parity()), to);
            self.swap(from, to);
            self[to] ^= 0b0000_0110;
            self[to] |= 0b1000_0000;
            self[from] = 0;
        
        } else if is_castle {
            if debugging_enabled {
                println!("Move {from} -> {to} is a castle.");
            }
            current_info.allowed_castles &= if from == 60 { 0b0000_0011 } else { 0b0000_1100 };
            current_info.zkey ^= zrist.pieces(&(PieceByte::KING | self[from].get_parity()), from);
            current_info.zkey ^= zrist.pieces(&(PieceByte::ROOK | self[to].get_parity()), to);
            if from > to {
                current_info.king_indices[if self[from].is_white() { 0 } else { 1 }] = from - 2;
                current_info.zkey ^= zrist.pieces(&(PieceByte::KING | self[from].get_parity()), from - 2);
                current_info.zkey ^= zrist.pieces(&(PieceByte::ROOK | self[from].get_parity()), from - 1);
                self.swap(from, from - 2);
                self.swap(to, from - 1);
                self[from - 2] |= 0b1000_0000;
                self[from - 1] |= 0b1000_0000;
            } else {
                current_info.king_indices[if self[from].is_white() { 0 } else { 1 }] = from + 2;
                current_info.zkey ^= zrist.pieces(&(PieceByte::KING | self[from].get_parity()), from + 2);
                current_info.zkey ^= zrist.pieces(&(PieceByte::ROOK | self[from].get_parity()), from + 1);
                self.swap(from, from + 2);
                self.swap(to, from + 1);
                self[from + 2] |= 0b1000_0000;
                self[from + 1] |= 0b1000_0000;
            }
        } else if is_take {
            if debugging_enabled {
                println!("Move {from} -> {to} is a take.");
            }
            if self[from].is_king() {
                current_info.king_indices[if self[from].is_white() { 0 } else { 1 }] = to;
            }
            current_info.zkey ^= zrist.pieces(&(self[from].get_piece() | self[from].get_parity()), from);
            current_info.zkey ^= zrist.pieces(&(self[to].get_piece() | self[to].get_parity()), to);
            self.swap(from, to);
            self[from] = 0;
            self[to] |= 0b1000_0000;
            current_info.zkey ^= zrist.pieces(&(self[to].get_piece() | self[to].get_parity()), to);
        } else {
            if debugging_enabled {
                println!("Move {from} -> {to} is a normal move.");
            }
            if self[from].is_king() {
                current_info.king_indices[if self[from].is_white() { 0 } else { 1 }] = to;
            }
            current_info.zkey ^= zrist.pieces(&(self[from].get_piece() | self[from].get_parity()), from);
            self.swap(from, to);
            current_info.zkey ^= zrist.pieces(&(self[to].get_piece() | self[to].get_parity()), to);
            self[to] |= 0b1000_0000;
            self[from] = 0;
        }

        current_info.maskset = MaskSet::from_board(self);
        return (original_board, original_info);
    }
    fn flipped(&self) -> Self {
        let mut array: [u8; 64] = [0u8; 64];
        let mut temp: [u64; 8] = [0u64; 8];
        for i in 0..8 {
            let index = i * 8;
            temp[i] = u64::from_ne_bytes([
                self[index],
                self[index + 1], 
                self[index + 2], 
                self[index + 3], 
                self[index + 4], 
                self[index + 5], 
                self[index + 6], 
                self[index + 7]
            ]);
        }
        temp.reverse();

        for i in 0..8 {
            let mut k = 0;
            for byte in &temp[i].to_ne_bytes(){
                array[i * 8 + k] = *byte;
                if array[i * 8 + k].is_piece() {
                    array[i * 8 + k] ^= 0b0000_1000;
                }
                k += 1;
            }
        }
        return array;
    }

    fn get_specific_motions(&self, of_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> Mask {
        let enemy = if of_parity == Parity::WHITE { &maskset.black } else { &maskset.white };
        let mut mask = Mask::default();
        for (index, byte) in self.iter().enumerate() {
            if byte.is_parity(of_parity) {
                mask |= match byte.get_piece() {
                    PieceByte::ROOK => rook_move(index, enemy, &maskset.all).0,
                    PieceByte::BISHOP => bishop_move(index, enemy, &maskset.all).0,
                    PieceByte::QUEEN => queen_move(index, enemy, &maskset.all).0,
                    PieceByte::PAWN => pawn_move(index, enemy, &maskset.all, of_parity, true, &enpassant).0,
                    PieceByte::KNIGHT => knight_move(index, enemy, &maskset.all).0,
                    PieceByte::KING => king_move(index, enemy, &maskset.all).0,
                    PieceByte::NONE => mask
                }
            }
        }
        return mask;
    }
    fn get_motions(&self, maskset: &MaskSet, enpassant: &Mask, castles: Option<u8>) -> MotionSet {
        let mut ms = MotionSet::default();
        let mut wking = 65;
        let mut bking = 65;
        for i in 0..64 {
            if self[i].is_w_king() {
                wking = i;
            } else if self[i].is_b_king() {
                bking = i;
            }
            if wking != 65 && bking != 65 { break };
        }
        if wking == 65 || bking == 65 { return ms };

        for (index, byte) in self.iter().enumerate() {
            let enemy = if byte.get_parity() == Parity::WHITE { &maskset.black } else { &maskset.white };
            let m = match byte.get_piece() {
                PieceByte::ROOK => rook_move(index, enemy, &maskset.all),
                PieceByte::BISHOP => bishop_move(index, enemy, &maskset.all),
                PieceByte::QUEEN => queen_move(index, enemy, &maskset.all),
                PieceByte::PAWN => pawn_move(index, enemy, &maskset.all, byte.get_parity(), false, &enpassant),
                PieceByte::KNIGHT => knight_move(index, enemy, &maskset.all),
                PieceByte::KING => king_move(index, enemy, &maskset.all),
                PieceByte::NONE => (Mask::default(), Mask::default())
            };
            if byte.is_parity(Parity::WHITE) {
                for bit in m.0.isolated_bits().iter() {
                    ms.white_moves[index].push(Motion { from: index, to: bit.as_index() });
                }
                for bit in m.1.isolated_bits().iter() {
                    ms.white_defensive_moves[index].push(Motion { from: index, to: bit.as_index() });
                }
            } else if byte.is_parity(Parity::BLACK) {
                for bit in m.0.isolated_bits().iter() {
                    ms.black_moves[index].push(Motion { from: index, to: bit.as_index() });
                }
                for bit in m.1.isolated_bits().iter() {
                    ms.black_defensive_moves[index].push(Motion { from: index, to: bit.as_index() });
                }
            }
        }
        let mut cloned = self.clone();
        for i in 0..64 {
            ms.white_moves[i].retain(|m| {
                let held = cloned.make_soft(m.from, m.to);
                let sp = cloned.get_specific_motions(Parity::BLACK, &MaskSet::from_board(&cloned), &Mask::default());
                let mut wk = wking;
                for wki in 0..64 {
                    if cloned[wki].is_w_king() {
                        wk = wki;
                        break;
                    }
                }

                cloned = held;
                if (Mask::from_index(wk) & sp).any() {
                    return false;
                }
                return true;
                 
            });
            ms.black_moves[i].retain(|m| {
                let held = cloned.make_soft(m.from, m.to);
                let sp = cloned.get_specific_motions(Parity::WHITE, &MaskSet::from_board(&cloned), &Mask::default());
                let mut bk = bking;
                for bki in 0..64 {
                    if cloned[bki].is_b_king() {
                        bk = bki;
                        break;
                    }
                }

                cloned = held;
                if (Mask::from_index(bk) & sp).any() {
                    return false;
                }
                return true;
            });
            for m in &ms.white_moves[i] {
                ms.white_vect.push(*m);
                let mtom = Mask::from_index(m.to);
                ms.white_flat |= mtom;
                ms.white_piecewise_flat[i] |= mtom;
            }
            for m in &ms.white_defensive_moves[i] {
                let mtom = Mask::from_index(m.to);
                ms.white_defensive_flat |= mtom;
                ms.white_defensive_piecewise_flat[i] |= mtom;
            }
            for m in &ms.black_moves[i] {
                ms.black_vect.push(*m);
                let mtom = Mask::from_index(m.to);
                ms.black_flat |= mtom;
                ms.black_piecewise_flat[i] |= mtom;
            }
            for m in &ms.black_defensive_moves[i] {
                let mtom = Mask::from_index(m.to);
                ms.black_defensive_flat |= mtom;
                ms.black_defensive_piecewise_flat[i] |= mtom;
            }
        }
        if let Some(allowed_castles) = castles {
            if !ms.white_moves[wking].is_empty() {
                let pcast = allowed_castles & if wking == 60 { 0b0000_0011 } else { 0b0000_1100 };
                if pcast > 0 {
                    let shallow = self.get_specific_motions(Parity::BLACK, maskset, enpassant);
                    if pcast & 0b0000_0101 != 0 && wking + 2 < 64 {
                        let mask = Mask::from_index(wking + 1) | Mask::from_index(wking + 2);
                        if (mask & maskset.all).none() && ((mask | Mask::from_index(wking)) & shallow).none() {
                            ms.white_moves[wking].push(Motion { from: wking, to: 63 });
                        }
                    }
                    if pcast & 0b0000_1010 != 0 && wking > 2 {
                        let mask = Mask::from_index(wking - 1) | Mask::from_index(wking - 2);
                        if ((mask | Mask::from_index(wking - 3)) & maskset.all).none() && ((mask | Mask::from_index(wking)) & shallow).none() {
                            ms.white_moves[wking].push(Motion { from: wking, to: 56 });
                        }
                    }
                }
            }
            if !ms.black_moves[bking].is_empty() {
                let pcast = allowed_castles & if bking == 60 { 0b0000_0011 } else { 0b0000_1100 };
                if pcast > 0 {
                    let shallow = self.get_specific_motions(Parity::WHITE, maskset, enpassant);
                    if pcast & 0b0000_0101 != 0 && bking + 2 < 64 {
                        let mask = Mask::from_index(bking + 1) | Mask::from_index(bking + 2);
                        if (mask & maskset.all).none() && ((mask | Mask::from_index(bking)) & shallow).none() {
                            ms.black_moves[bking].push(Motion { from: bking, to: 7 });
                        }
                    }
                    if pcast & 0b0000_1010 != 0 && bking > 2 {
                        let mask = Mask::from_index(bking - 1) | Mask::from_index(bking - 2);
                        if ((mask | Mask::from_index(bking - 3)) & maskset.all).none() && ((mask | Mask::from_index(bking)) & shallow).none() {
                            ms.black_moves[bking].push(Motion { from: bking, to: 0 });
                        }
                    }
                }
            }
        }
        return ms;
    }
}
