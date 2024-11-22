use crate::Parity;
use crate::shared::{ mask::Mask, piece::PieceCachedMoves };

use super::piece::PieceByte;
use super::point::Point;


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
    pub threats: Mask,
    pub castles: u8,
    pub halfmove_clock: u64,
    pub fullmove_number: u64,
    pub branches: Vec<State>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            moves: [PieceCachedMoves::default(); 64],
            threats: Mask::default(),
            enpassant: Mask::default(),
            turn: Parity::NONE,
            board: [0u8; 64],
            previous_board: [0u8; 64],
            maskset: MaskSet::default(),
            castles: 0u8,
            fullmove_number: 0u64,
            halfmove_clock: 0u64,
            branches: Vec::new()
        }
    }
}


impl State {
    #[inline(always)]
    pub fn white_kingside_can_castle(&self) -> bool {
        return (self.castles & 0b00000001) != 0;
    }
    #[inline(always)]
    pub fn white_queenside_can_castle(&self) -> bool {
        return (self.castles & 0b00000010) != 0;
    }
    #[inline(always)]
    pub fn black_kingside_can_castle(&self) -> bool {
        return (self.castles & 0b00000100) != 0;
    }
    #[inline(always)]
    pub fn black_queenside_can_castle(&self) -> bool {
        return (self.castles & 0b00001000) != 0;
    }
    #[inline(always)]
    pub fn get_piece_at_index(&self, index: usize) -> u8 {
        if index < 64 {
            return self.board[index];
        }
        return 0u8;
    }
    pub fn refresh(&mut self, prev_board: Option<&[u8; 64]>) {
        self.previous_board = *prev_board.unwrap_or(&self.board.clone());
        let diff = self.previous_board.bit_xor(&self.board);
        if diff.all_are(PieceByte::PAWN) {
            let diff_masks = diff.to_mask().isolated_bits();
            if diff_masks.len() == 2 {
                if Mask::get_y_gap(&diff_masks[0], &diff_masks[1]) == 2 {
                    let (mut from, mut to) = if diff_masks[0].raw < diff_masks[1].raw {
                        (diff_masks[0].raw, diff_masks[1].raw)
                    } else {
                        (diff_masks[1].raw, diff_masks[0].raw)
                    };
                    self.enpassant = Mask { raw: from << 8 };
                }
            }
        }
        let mut cloned = self.moves;
        for (index, cache) in cloned.iter_mut().enumerate() {
            if cache.moves.any() {
                for &mut bit in cache.moves.isolated_bits().iter_mut() {
                    let branch = self.branch_off(Mask::from_index(index), bit);
                    if !branch.is_valid(!self.turn) {
                        cache.moves ^= bit;
                    }
                }
            }
        }
        self.moves = cloned;
        for (index, piece) in self.board.iter().enumerate() {
            if (piece & 0b10000000) != 0 {
                if piece.get_ptype() == PieceByte::KING {
                    self.castles &= if piece.get_parity() == Parity::WHITE { 0b00000011 } else { 0b00001100 };
                } else if piece.get_ptype() == PieceByte::ROOK {
                    if (index % 8) < 4 {
                        self.castles &= if piece.get_parity() == Parity::WHITE { 0b00000111 } else { 0b00001101 };
                    } else {
                        self.castles &= if piece.get_parity() == Parity::WHITE { 0b00001011 } else { 0b00001110 };
                    }
                }
            }
        }
    }

    pub fn is_valid(&self, valid_for: Parity) -> bool {
        for (index, &piece) in self.board.iter().enumerate() {
            if piece.get_parity() == valid_for {
                if (piece & PieceByte::KING) != 0 {
                    if (Mask::from_index(index) & self.threats).any() {
                        return false;
                    }
                }
            }
        }
        return true;
    }

    pub fn branch_off(&mut self, from: Mask, to: Mask) -> Self {
        let mut branch = State::default();
        branch.turn = !self.turn;
        branch.board = self.board.with_move(from, to);
        branch.maskset = branch.board.generate_maskset(branch.turn);
        (branch.moves, branch.threats) = branch.board.get_moves(!self.turn, &branch.maskset.enemy, &branch.maskset.piece, &self.enpassant);
        branch.halfmove_clock = if self.board[from].get_ptype() == PieceByte::PAWN { self.halfmove_clock + 1 } else { 0 };
        branch.fullmove_number = if self.turn == Parity::BLACK { self.fullmove_number + 1 } else { self.fullmove_number };
        branch.castles = branch.board.get_castles(Some(self.castles));
        branch.refresh(Some(&self.board));
        return branch;
    }
}

pub trait ChessByte {
    fn get_parity(&self) -> Parity;
    fn get_ptype(&self) -> PieceByte;
    fn get_directions(&self) -> u8;
    fn has_moved(&self) -> bool;
    fn is_kingside(&self) -> bool;
    fn is_queenside(&self) -> bool;
    fn is_sided(&self) -> bool;
}
impl ChessByte for u8 {
    fn get_parity(&self) -> Parity {
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

fn sliding_move(piece: u8, pos: usize, _everyone_else: &[u8; 64], enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let pos_mask = Mask::from_index(pos);
    
    let mut move_mask = Mask::default();
    let dirs = piece.get_directions();
    let mut hits = 0b00000000u8;
    for i in 1..=8 {
        for k in 0..8 {
            if dirs & (1 << k) != 0 {
                let pos = match k {
                    0 => Mask::point_add(&pos_mask, &(Point { x: 1, y: 0 } * i)),
                    1 => Mask::point_add(&pos_mask, &(Point { x: -1, y: 0 } * i)),
                    2 => Mask::point_add(&pos_mask, &(Point { x: 0, y: -1 } * i)),
                    3 => Mask::point_add(&pos_mask, &(Point { x: 0, y: 1 } * i)),
                    4 => Mask::point_add(&pos_mask, &(Point { x: -1, y: 1 } * i)),
                    5 => Mask::point_add(&pos_mask, &(Point { x: -1, y: -1 } * i)),
                    6 => Mask::point_add(&pos_mask, &(Point { x: 1, y: -1 } * i)),
                    7 => Mask::point_add(&pos_mask, &(Point { x: 1, y: 1 } * i)),
                    _ => Mask::point_add(&pos_mask, &(Point::default()))
                };
                if pos.any() {
                    if hits & (1 << k) != 0 { continue };
                    if (*piece_mask & pos_mask).any() {
                        if (*enemy_mask & pos_mask).any() {
                            move_mask |= pos_mask;
                        }
                        hits |= 1 << k;
                    } else {
                        move_mask |= pos_mask;
                    }
                } else {
                    hits |= 1 << k;
                }
            }
        }
    }
    return move_mask;
}
fn pawn_move(piece: u8, pos: usize, _everyone_else: &[u8; 64], enemy_mask: &Mask, piece_mask: &Mask, enpassant: &Mask) -> Mask {
    let pos_mask = Mask::from_index(pos);
    let mut move_mask = Mask::default();
    let par = if piece.get_parity() == Parity::WHITE { -1 } else { 1 };
    let basic = Mask::point_add(&pos_mask, &Point {x: 0, y: par});
    if basic.none() { return move_mask };

    if (*piece_mask & basic).none() {
        move_mask |= basic;
        if !piece.has_moved() {
            let dbl = Mask::point_add(&basic, &Point {x: 0, y: par});
            if dbl.any() {
                if (*piece_mask & dbl).none() {
                    move_mask |= dbl;
                }
            }
        }
    }
    let diags: [Mask; 2] = [
        Mask::point_add(&basic, &Point{x: -1, y: 0}),
        Mask::point_add(&basic, &Point{x: 1, y: 0}),
    ];
    for d in diags {
        if d.any() {
            if (*enemy_mask & d).any() {
                move_mask |= d;
            } else if (*piece_mask & d).none() && enpassant.any() && enpassant.raw == d.raw {
                move_mask |= d;
            }
        }
    }
    return move_mask;
}
fn knight_move(_piece: u8, pos: usize, _everyone_else: &[u8; 64], enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    let pos_mask = Mask::from_index(pos);

    let offsets = [
        Mask::point_add(&pos_mask, &Point { x: -2, y: -1 }),
        Mask::point_add(&pos_mask, &Point { x: -2, y: 1 }),
        Mask::point_add(&pos_mask, &Point { x: -1, y: -2 }),
        Mask::point_add(&pos_mask, &Point { x: -1, y: 2 }),
        Mask::point_add(&pos_mask, &Point { x: 1, y: -2 }),
        Mask::point_add(&pos_mask, &Point { x: 1, y: 2 }),
        Mask::point_add(&pos_mask, &Point { x: 2, y: -1 }),
        Mask::point_add(&pos_mask, &Point { x: 2, y: 1 })
    ];
    for offset in offsets {
        if offset.any() {
            if (*piece_mask & offset).any() {
                if (*enemy_mask & offset).any() {
                    move_mask |= offset;
                }
            } else {
                move_mask |= offset;
            }
        }
    }
    return move_mask;
}
fn king_move(_piece: u8, pos: usize, _everyone_else: &[u8; 64], enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask = Mask::default();
    let pos_mask = Mask::from_index(pos);
    for y in -1..2 {
        for x in -1..2 {
            if x == 0 && y == 0 { continue };
            let km_m = Mask::point_add(&pos_mask, &Point { x, y });
            if km_m.any() {
                if (*piece_mask & km_m).none() || (*enemy_mask & km_m).any() {
                    move_mask |= km_m;
                }
            }
        }
    }
    return move_mask;
}


trait BoardArray {
    fn get_moves(&self, ally_parity: Parity, enemy_mask: &Mask, piece_mask: &Mask, enpassant: &Mask) -> ([PieceCachedMoves; 64], Mask);
    fn generate_maskset(&self, ally_parity: Parity) -> MaskSet;
    fn get_castles(&self, current_castle_byte: Option<u8>) -> u8;
    fn with_move(&self, from: Mask, to: Mask) -> Self;
    fn bit_xor(&self, other: &[u8; 64]) -> [u8; 64];
    fn any(&self) -> bool;
    fn bit_and(&self, other: &[u8; 64]) -> [u8; 64];
    fn any_are(&self, are: PieceByte) -> bool;
    fn all_are(&self, are: PieceByte) -> bool;
    fn to_mask(&self) -> Mask;
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



impl BoardArray for [u8; 64] {
    fn to_mask(&self) -> Mask {
        let mut m = Mask::default();
        for (index, &byte) in self.iter().enumerate() {
            if byte != 0 {
                m.raw |= 1u64 << (index) << (index % 8);
            }
        }
        return m;
    }
    fn all_are(&self, are: PieceByte) -> bool {
        for &byte in self.iter() {
            if ((byte & 0b00000111) ^ are as u8) != 0 {
                return false;
            }
        }
        return true;
    }
    fn any_are(&self, are: PieceByte) -> bool {
        for &byte in self.iter() {
            if ((byte & 0b00000111) ^ are as u8) == 0 {
                return true;
            }
        }
        return false;
    }
    fn any(&self) -> bool {
        for &byte in self.iter() {
            if byte != 0 {
                return true;
            }
        }
        return false;
    }
    fn bit_and(&self, other: &[u8; 64]) -> [u8; 64] {
        let mut array = [0b00000000u8; 64];
        for (index, &byte) in self.iter().enumerate() {
            array[index] = byte & other[index];
        }
        return array;
    }
    fn bit_xor(&self, other: &[u8; 64]) -> [u8; 64] {
        let mut array = [0b00000000u8; 64];
        for (index, &byte) in self.iter().enumerate() {
            array[index] = byte ^ other[index];
        }
        return array;
    }
    fn with_move(&self, from: Mask, to: Mask) -> Self {
        let mut array = self.clone();
        array[to] = array[from] | 0b10000000;
        array[from] = array[from] ^ array[from];
        return array;
    }
    fn get_castles(&self, current_castle_byte: Option<u8>) -> u8 {
        let mut castle_byte = current_castle_byte.unwrap_or(0b00001111u8);
        for (index, &byte) in self.iter().enumerate() {
            if castle_byte == 0 { return castle_byte };
            if byte.has_moved() {
                if byte.get_ptype() == PieceByte::KING {
                    castle_byte &= if byte.get_parity() == Parity::WHITE { 0b00001100u8 } else { 0b00000011u8 };
                } else if byte.get_ptype() == PieceByte::ROOK {
                    if byte.get_parity() == Parity::WHITE {
                        if (castle_byte & 0b00000001) != 0 && byte.is_kingside() {
                            castle_byte &= 0b11111110;
                        }
                        if (castle_byte & 0b00000010) != 0 && byte.is_queenside() {
                            castle_byte &= 0b11111101;
                        }
                    } else {
                        if (castle_byte & 0b00000100) != 0 && byte.is_kingside() {
                            castle_byte &= 0b11111011;
                        }
                        if (castle_byte & 0b00001000) != 0 && byte.is_queenside() {
                            castle_byte &= 0b11110111;
                        }
                    }
                }
            }
        }
        return castle_byte;
    }
    fn get_moves(&self, ally_parity: Parity, enemy_mask: &Mask, piece_mask: &Mask, enpassant: &Mask) -> ([PieceCachedMoves; 64], Mask) {
        let mut array: [PieceCachedMoves; 64] = [PieceCachedMoves::default(); 64];
        let mut threats = Mask::default();
        for (index, &byte) in self.iter().enumerate() {
            let cache = match byte.get_ptype() {
                PieceByte::ROOK | PieceByte::QUEEN | PieceByte::BISHOP => {
                    PieceCachedMoves {
                        moves: sliding_move(byte, index, self, enemy_mask, piece_mask),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::PAWN => {
                    PieceCachedMoves {
                        moves: pawn_move(byte, index, self, enemy_mask, piece_mask, enpassant),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::KNIGHT => {
                    PieceCachedMoves {
                        moves: knight_move(byte, index, self, enemy_mask, piece_mask),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::KING => {
                    PieceCachedMoves {
                        moves: king_move(byte, index, self, enemy_mask, piece_mask),
                        castles: 0u8,
                        state: Mask::default()
                    }
                }
            };
            if byte.get_parity() == ally_parity {
                array[index] = cache;
            } else {
                if byte.get_ptype() == PieceByte::PAWN {
                    threats |= Mask::of_column((index % 8) as i32).not() & cache.moves;
                } else {
                    threats |= cache.moves;
                }
            }
        }
        return (array, threats);
    }
    fn generate_maskset(&self, ally_parity: Parity) -> MaskSet {
        let mut ms = MaskSet {
            ally: Mask::default(),
            enemy: Mask::default(),
            piece: Mask::default()
        };
        for (index, &byte) in self.iter().enumerate() {
            if byte.get_parity() == ally_parity {
                ms.ally |= Mask::from_index(index);
            } else {
                ms.enemy |= Mask::from_index(index);
            }
        }
        ms.piece = ms.ally | ms.enemy;
        return ms;
    }
}
