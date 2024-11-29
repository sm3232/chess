use std::{cell::RefCell, rc::Rc};

use crate::{cutil::pretty_print::pretty_print_mask, shared::{
    chessbyte::{
        byte_is_kingside, byte_is_queenside, ChessByte
    }, mask::Mask, maskset::MaskSet, piece::{
        Parity, PieceByte
    }, point::Point
}};

use super::{motion::Motion, state::{RetainedStateInfo, ARRAY_REPEAT_VALUE}, zobrist::Zobrist};

const BISHOP_DIRS: [Point; 4] = [ Point {x: -1, y: -1 }, Point { x: -1, y: 1 }, Point { x: 1, y: 1 }, Point { x: 1, y: -1 } ];
const ROOK_DIRS: [Point; 4] = [ Point { x: 0, y: 1 }, Point { x: 0, y: -1 }, Point { x: 1, y: 0 }, Point { x: -1, y: 0 } ];

fn bishop_move(bishop_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut hits = 0b0000_0000;
    let mut move_mask = Mask::default();
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
    return move_mask;
}
fn rook_move(rook_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut hits = 0b0000_0000;
    let mut move_mask = Mask::default();
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
    return move_mask;
}
#[inline]
fn queen_move(queen_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    return rook_move(queen_index, enemy_mask, piece_mask) | bishop_move(queen_index, enemy_mask, piece_mask);
}
fn pawn_move(pawn_index: usize, enemy_mask: &Mask, piece_mask: &Mask, parity: Parity, ignore_diagonal_enemy_requirement: bool, enpassant: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    if pawn_index < 8 || pawn_index > 55 {
        return move_mask;
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
        if (*enemy_mask & diag1).any() || ignore_diagonal_enemy_requirement {
            move_mask |= diag1;
        } else if (*piece_mask & *enpassant).none() && (*enpassant & diag1).any() {
            move_mask |= diag1;
        }
    }
    let diag2 = pos_pos + Point { x: -1, y: ydir };
    if diag2.valid() {
        if (*enemy_mask & diag2).any() || ignore_diagonal_enemy_requirement {
            move_mask |= diag2;
        } else if (*piece_mask & *enpassant).none() && (*enpassant & diag2).any() {
            move_mask |= diag2;
        }
    }
    /*
    if pawn_index % 8 < 7 {
        let diag = Mask { raw: if parity == Parity::WHITE { basic.raw <> 1 } else { basic.raw << 1 } };
    }
    if pawn_index % 8 > 0 {
        let diag = Mask { raw: if parity == Parity::WHITE { basic.raw << 1 } else { basic.raw >> 1 } };
        if (*enemy_mask & diag).any() {
            move_mask |= diag;
        }
    }
    */
    return move_mask;
}
fn knight_move(knight_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
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
            // move_mask |= Mask { raw: if offset.0 < 0 { pos_mask.raw >> (offset.0 * -1) } else { pos_mask.raw << offset.0 } };
            // move_mask |= Mask { raw: if offset.0 < 0 { pos_mask.raw >> (offset.1 * -8) } else { pos_mask.raw << (offset.1 * 8) } };
        }
    }
    return move_mask & (enemy_mask.get_not() & *piece_mask).get_not();
    
}
fn king_move(king_index: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    let pos_mask = Mask::from_index(king_index);
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
            if (*piece_mask & thispos).none() || (*enemy_mask & thispos).any() {
                move_mask |= thispos;
            }
        }
    }

    return move_mask;
}

fn bishop_xray(bishop_index: usize) -> Mask {
    let bim8 = bishop_index % 8;
    let bid8 = bishop_index / 8;
    let mut mask = Mask::default();
    for i in 0..64 {
        let im8 = i % 8;
        let id8 = i / 8;
        if im8 + id8 == bim8 + bid8 || (bim8).abs_diff(im8) == (bid8).abs_diff(id8) {
            mask |= i;
        }
    }
    return mask;
}
fn rook_xray(rook_index: usize) -> Mask {
    let rim8 = rook_index % 8;
    let rid8 = rook_index / 8;
    let mut mask = Mask::default();
    for i in 0..64 {
        if i % 8 == rim8 || i / 8 == rid8 {
            mask |= i;
        }
    }
    return mask;
}
#[inline(always)]
fn queen_xray(queen_index: usize) -> Mask { bishop_xray(queen_index) | rook_xray(queen_index) }


fn sliding_move(piece: u8, index: usize, _everyone_else: &[u8; 64], enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let index_mask = Mask::from_index(index);
    
    let mut move_mask = Mask::default();
    let dirs = piece.get_directions();
    let mut hits = 0b00000000u8;
    for i in 1..=8 {
        for k in 0..8 {
            if dirs & (1 << k) != 0 {
                let pos = match k {
                    0 => Mask::point_add(&index_mask, &(Point { x: 1, y: 0 } * i)),
                    1 => Mask::point_add(&index_mask, &(Point { x: -1, y: 0 } * i)),
                    2 => Mask::point_add(&index_mask, &(Point { x: 0, y: -1 } * i)),
                    3 => Mask::point_add(&index_mask, &(Point { x: 0, y: 1 } * i)),
                    4 => Mask::point_add(&index_mask, &(Point { x: -1, y: 1 } * i)),
                    5 => Mask::point_add(&index_mask, &(Point { x: -1, y: -1 } * i)),
                    6 => Mask::point_add(&index_mask, &(Point { x: 1, y: -1 } * i)),
                    7 => Mask::point_add(&index_mask, &(Point { x: 1, y: 1 } * i)),
                    _ => Mask::point_add(&index_mask, &(Point::default()))
                };
                if pos.any() {
                    if hits & (1 << k) != 0 { continue };
                    if (*piece_mask & pos).any() {
                        if (*enemy_mask & pos).any() {
                            move_mask |= pos;
                        }
                        hits |= 1 << k;
                    } else {
                        move_mask |= pos;
                    }
                } else {
                    hits |= 1 << k;
                }
            }
        }
    }
    return move_mask;
}
fn pawn_xray(piece: u8, pos: usize, enemy_mask: &Mask, piece_mask: &Mask, enpassant: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    if pos / 8 > 47 || pos / 8 < 16 {
        return move_mask;
    }
    let pos_mask = Mask::from_index(pos);
    let mut par = 0;
    move_mask |= Mask { raw: if piece.is_white() { pos_mask.raw >> 8 } else { pos_mask.raw << 8 }};
    if !piece.has_moved() {
        move_mask |= Mask { raw: if piece.is_white() { pos_mask.raw >> 16 } else { pos_mask.raw << 16 } };
    }
    let d1 = Mask { raw: if piece.is_white() { pos_mask.raw >> 9 } else { pos_mask.raw << 9 } };
    let d2 = Mask { raw: if piece.is_white() { pos_mask.raw >> 7 } else { pos_mask.raw << 7 } };
    if (*enemy_mask & d1).any() || enpassant.raw == d1.raw {
        move_mask |= d1;
    }
    if (*enemy_mask & d2).any() || enpassant.raw == d2.raw {
        move_mask |= d2;
    }
    return move_mask;
}
fn knight_xray(_piece: u8, pos: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    let pos_mask = Mask::from_index(pos);
    let pos_pos = Point::from_index(pos);
    let offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
    for offset in offsets {
        let xv = pos_pos.x + offset.0;
        let yv = pos_pos.y + offset.1;
        if xv >= 0 && xv < 8 && yv >= 0 && yv < 8 {
            move_mask |= Mask { raw: if offset.0 < 0 { pos_mask.raw << (offset.0 * -1) } else { pos_mask.raw >> offset.0 } };
            move_mask |= Mask { raw: if offset.0 < 0 { pos_mask.raw << (offset.1 * -8) } else { pos_mask.raw >> (offset.1 * 8) } };
        }
    }
    return move_mask;
}
fn king_xray(_piece: u8, pos: usize, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    let pos_mask = Mask::from_index(pos);
    let pos_pos = Point::from_index(pos);
    for y in -1..2 {
        for x in -1..2 {
            if x == 0 && y == 0 { continue };
            let xv = pos_pos.x + x;
            let yv = pos_pos.y + y;
            if xv >= 0 && xv < 8 && yv >= 0 && yv < 8 {
                move_mask |= Mask::from_point(Point { x: xv, y: yv });
            }
        }
    }
    return move_mask;
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
pub trait BoardArray {
    fn get_xrays(&self, ally_parity: Parity, info: &RetainedStateInfo) -> [Vec<Motion>; 64];
    fn get_moves(&self, ally_parity: Parity, info: &RetainedStateInfo) -> [Vec<Motion>; 64];
    fn flipped(&self) -> Self;
    fn make(&mut self, from: usize, to: usize, zobrist: Rc<RefCell<Zobrist>>, current_info: &mut RetainedStateInfo, debugging_enabled: bool) -> ([u8; 64], RetainedStateInfo);
    fn unmake(&mut self, original_board: &[u8; 64], original_info: &RetainedStateInfo, current_info: &mut RetainedStateInfo) -> ();
    fn get_moves_shallow_ipd(&self, ally_parity: Parity, info: &RetainedStateInfo) -> Mask;
    fn index_in_check(&self, index: usize, ip: Parity, info: &RetainedStateInfo) -> bool;
}
impl BoardArray for [u8; 64] {
    fn index_in_check(&self, index: usize, ip: Parity, info: &RetainedStateInfo) -> bool {
        return (Mask::from_index(index) & self.get_moves_shallow_ipd(!ip, info)).any()
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
    fn make(&mut self, from: usize, to: usize, zobrist: Rc<RefCell<Zobrist>>, current_info: &mut RetainedStateInfo, debugging_enabled: bool) -> ([u8; 64], RetainedStateInfo) {
        let original_info = current_info.clone();

        let original_board = self.clone();
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
        let is_move = !(is_enpassant || is_promotion || is_castle || is_take); 
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
        if is_enpassant {
            if debugging_enabled {
                println!("Move {from} -> {to} is an enpassant.");
            }
            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::PAWN | self[from].get_parity()), from);
            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::PAWN | self[from].get_parity()), to);
            if from > to {
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::PAWN | !self[from].get_parity()), to + 8);
                self[to + 8] = 0;
            } else {
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::PAWN | !self[from].get_parity()), to - 8);
                self[to - 8] = 0;
            }
            self.swap(from, to);
            self[to] |= 0b1000_0000;
        } else if is_promotion {
            if debugging_enabled {
                println!("Move {from} -> {to} is a promotion.");
            }

            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::PAWN | self[from].get_parity()), from);
            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::QUEEN | self[from].get_parity()), to);
            self.swap(from, to);
            self[to] ^= 0b0000_0110;
            self[to] |= 0b1000_0000;
            self[from] = 0;
        
        } else if is_castle {
            if debugging_enabled {
                println!("Move {from} -> {to} is a castle.");
            }
            current_info.allowed_castles &= if from == 60 { 0b0000_0011 } else { 0b0000_1100 };
            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::KING | self[from].get_parity()), from);
            current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::ROOK | self[to].get_parity()), to);
            if from > to {
                current_info.king_indices[if from == 60 { 0 } else { 1 }] += 2;
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::KING | self[from].get_parity()), from - 2);
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::ROOK | self[from].get_parity()), from - 1);
                self.swap(from, from - 2);
                self.swap(to, from - 1);
                self[from - 2] |= 0b1000_0000;
                self[from - 1] |= 0b1000_0000;
            } else {
                current_info.king_indices[if from == 60 { 0 } else { 1 }] -= 2;
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::KING | self[from].get_parity()), from + 2);
                current_info.zkey ^= zobrist.borrow().pieces(&(PieceByte::ROOK | self[from].get_parity()), from + 1);
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
            current_info.zkey ^= zobrist.borrow().pieces(&(self[from].get_piece() | self[from].get_parity()), from);
            current_info.zkey ^= zobrist.borrow().pieces(&(self[to].get_piece() | self[to].get_parity()), to);
            self.swap(from, to);
            self[from] = 0;
            self[to] |= 0b1000_0000;
            current_info.zkey ^= zobrist.borrow().pieces(&(self[to].get_piece() | self[to].get_parity()), to);
        } else {
            if debugging_enabled {
                println!("Move {from} -> {to} is a normal move.");
            }
            if self[from].is_king() {
                current_info.king_indices[if self[from].is_white() { 0 } else { 1 }] = to;
            }
            current_info.zkey ^= zobrist.borrow().pieces(&(self[from].get_piece() | self[from].get_parity()), from);
            self.swap(from, to);
            current_info.zkey ^= zobrist.borrow().pieces(&(self[to].get_piece() | self[to].get_parity()), to);
            self[to] |= 0b1000_0000;
            self[from] = 0;
        }

        current_info.maskset = MaskSet::from_board(self);
        return (original_board, original_info);
    }
    fn flipped(&self) -> Self {
        let mut array: [u8; 64] = [0u8; 64];
        for y in 0..8 {
            for x in 0..8 {
                array[(7 - y) * 8 + x] = self[y * 8 + x] ^ 0b0000_1000;
            }
        }
        return array;
    }

    fn get_moves_shallow_ipd(&self, ally_parity: Parity, info: &RetainedStateInfo) -> Mask {
        let enemy = if ally_parity == Parity::WHITE { &info.maskset.black } else { &info.maskset.white };
        let mut mask = Mask::default();
        for (index, byte) in self.iter().enumerate() {
            if byte.is_parity(ally_parity) {
                mask |= match byte.get_piece() {
                    PieceByte::ROOK => rook_move(index, enemy, &info.maskset.all),
                    PieceByte::BISHOP => bishop_move(index, enemy, &info.maskset.all),
                    PieceByte::QUEEN => queen_move(index, enemy, &info.maskset.all),
                    PieceByte::PAWN => pawn_move(index, enemy, &info.maskset.all, ally_parity, true, &info.enpassant_mask),
                    PieceByte::KNIGHT => knight_move(index, enemy, &info.maskset.all),
                    PieceByte::KING => king_move(index, enemy, &info.maskset.all),
                    PieceByte::NONE => mask
                }
            }
        }
        return mask;
    }

    fn get_moves(&self, ally_parity: Parity, info: &RetainedStateInfo) -> [Vec<Motion>; 64] {
        let enemy = if ally_parity == Parity::WHITE { &info.maskset.black } else { &info.maskset.white };
        let mut array: [Vec<Motion>; 64] = [ARRAY_REPEAT_VALUE; 64];
        for (index, byte) in self.iter().enumerate() {
            if byte.is_parity(ally_parity) {
                let m = match byte.get_piece() {
                    PieceByte::ROOK => rook_move(index, enemy, &info.maskset.all),
                    PieceByte::BISHOP => bishop_move(index, enemy, &info.maskset.all),
                    PieceByte::QUEEN => queen_move(index, enemy, &info.maskset.all),
                    PieceByte::PAWN => pawn_move(index, enemy, &info.maskset.all, ally_parity, false, &info.enpassant_mask),
                    PieceByte::KNIGHT => knight_move(index, enemy, &info.maskset.all),
                    PieceByte::KING => king_move(index, enemy, &info.maskset.all),
                    PieceByte::NONE => Mask::default()
                };
                for bit in m.isolated_bits().iter() {
                    array[index].push(Motion { from: index, to: bit.as_index() });
                }
            }
        }
        return array;
    }
    fn get_xrays(&self, ally_parity: Parity, info: &RetainedStateInfo) -> [Vec<Motion>; 64] {
        let enemy = if ally_parity == Parity::WHITE { &info.maskset.black } else { &info.maskset.white };
        let mut array: [Vec<Motion>; 64] = [ARRAY_REPEAT_VALUE; 64];
        for (index, &byte) in self.iter().enumerate() {
            if byte.is_parity(ally_parity) {
                let m = match byte.get_piece() {
                    PieceByte::ROOK => rook_xray(index),
                    PieceByte::BISHOP => bishop_xray(index),
                    PieceByte::QUEEN => queen_xray(index),
                    PieceByte::PAWN => pawn_xray(byte, index, enemy, &info.maskset.all, &info.enpassant_mask),
                    PieceByte::KNIGHT => knight_xray(byte, index, enemy, &info.maskset.all),
                    PieceByte::KING => king_xray(byte, index, enemy, &info.maskset.all),
                    PieceByte::NONE => Mask::default()
                };
                for bit in m.isolated_bits().iter() {
                    array[index].push(Motion { from: index, to: bit.as_index() });
                }
            }
        }
        return array;
    }
}
