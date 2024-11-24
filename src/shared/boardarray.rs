use crate::shared::{
    chessbyte::ChessByte, 
    mask::Mask, 
    piece::{
        Parity, 
        PieceByte, 
        PieceCachedMoves
    }, 
    point::Point, 
    state::MaskSet
};

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
pub trait BoardArray {
    fn get_moves(&self, ally_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> ([PieceCachedMoves; 64], Mask);
    fn generate_maskset(&self, ally_parity: Parity) -> MaskSet;
    fn get_castles(&self, current_castle_byte: Option<u8>) -> u8;
    fn with_move(&self, from: Mask, to: Mask, enpassant: &Mask) -> Self;
    fn bit_xor(&self, other: &[u8; 64]) -> [u8; 64];
    fn any(&self) -> bool;
    fn bit_and(&self, other: &[u8; 64]) -> [u8; 64];
    fn any_are(&self, are: PieceByte) -> bool;
    fn all_are(&self, are: PieceByte) -> bool;
    fn with_move_indexed(&self, from: usize, to: usize, enpassant: &Mask) -> Self;
    fn get_shallow_moves(&self, ally_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> Mask;
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
    fn with_move(&self, from: Mask, to: Mask, enpassant: &Mask) -> Self {
        let mut array = self.clone();
        array[to] = array[from] | 0b10000000;
        array[from] = array[from] ^ array[from];
        if enpassant.any() && enpassant.raw == to.raw {
            if from.raw < to.raw {
                array[to.as_index() - 8] = 0u8;
            } else {
                array[to.as_index() + 8] = 0u8;
            }
        }
        return array;
    }
    fn with_move_indexed(&self, from: usize, to: usize, enpassant: &Mask) -> Self {
        let mut array = self.clone();
        array[to] = array[from] | 0b10000000;
        array[from] ^= array[from];
        if enpassant.any() {
            let ei = enpassant.as_index();
            if ei == to {
                if from < to {
                    array[to - 8] = 0u8;
                } else {
                    array[to + 8] = 0u8;
                }
            }
        }
        return array;
    }
    fn get_castles(&self, current_castle_byte: Option<u8>) -> u8 {
        let mut castle_byte = current_castle_byte.unwrap_or(0b00001111u8);
        for &byte in self.iter() {
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
    fn get_shallow_moves(&self, ally_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> Mask {
        let mut moves = Mask::default();
        for (index, &byte) in self.iter().enumerate() {
            if byte.get_parity() != ally_parity {
                continue;
            }
            moves |= match byte.get_ptype() {
                PieceByte::ROOK | PieceByte::QUEEN | PieceByte::BISHOP => {
                    sliding_move(byte, index, self, &maskset.enemy, &maskset.piece)
                },
                PieceByte::PAWN => {
                    pawn_move(byte, index, self, &maskset.enemy, &maskset.piece, enpassant)
                },
                PieceByte::KNIGHT => {
                    knight_move(byte, index, self, &maskset.enemy, &maskset.piece)
                },
                PieceByte::KING => {
                    king_move(byte, index, self, &maskset.enemy, &maskset.piece)
                }
            };
        }
        return moves;
    }
    fn get_moves(&self, ally_parity: Parity, maskset: &MaskSet, enpassant: &Mask) -> ([PieceCachedMoves; 64], Mask) { 
        let mut array: [PieceCachedMoves; 64] = [PieceCachedMoves::default(); 64];
        let mut shallow_m = Mask::default();
        for (index, &byte) in self.iter().enumerate() {
            if byte.get_parity() != ally_parity {
                continue;
            }
            array[index] = match byte.get_ptype() {
                PieceByte::ROOK | PieceByte::QUEEN | PieceByte::BISHOP => {
                    PieceCachedMoves {
                        moves: sliding_move(byte, index, self, &maskset.enemy, &maskset.piece),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::PAWN => {
                    PieceCachedMoves {
                        moves: pawn_move(byte, index, self, &maskset.enemy, &maskset.piece, enpassant),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::KNIGHT => {
                    PieceCachedMoves {
                        moves: knight_move(byte, index, self, &maskset.enemy, &maskset.piece),
                        castles: 0u8,
                        state: Mask::default()
                    }
                },
                PieceByte::KING => {
                    PieceCachedMoves {
                        moves: king_move(byte, index, self, &maskset.enemy, &maskset.piece),
                        castles: 0u8,
                        state: Mask::default()
                    }
                }
            };
        }
        for i in array.iter() {
            shallow_m |= i.moves;
        }
        return (array, shallow_m);
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
            } else if byte.get_parity() == !ally_parity {
                ms.enemy |= Mask::from_index(index);
            }
        }
        ms.piece = ms.ally | ms.enemy;
        return ms;
    }
}
