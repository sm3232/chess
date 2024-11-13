
use crate::cutil::piece::{self, PieceType };
use crate::cutil::mask::Mask;
use crate::cutil::point::{algebraic_to_point, point, Point};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use super::piece::{create_piece, PieceProperties};
use super::pretty_print_future;

pub struct MoveInfo {
    pub was_successful: bool,
    pub executor: Option<Rc<RefCell<Box<dyn piece::TPiece>>>>,
    pub victim: Option<Rc<RefCell<Box<dyn piece::TPiece>>>>
}


pub struct ChessGame {
    pub pvec: Vec<Rc<RefCell<Box<dyn piece::TPiece>>>>,
    pub turn: bool,
    pub castles: u8,
    pub enpassant: Option<Point>,
    pub enpassant_piece: Option<Rc<RefCell<Box<dyn piece::TPiece>>>>,
    pub selected: Option<Rc<RefCell<Box<dyn piece::TPiece>>>>,
    pub selected_moves: Option<Mask>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
    pub ally_mask: Mask,
    pub enemy_mask: Mask,
    pub piece_mask: Mask,
    pub threatened_squares: Mask
}
impl ChessGame {
    pub fn init(fen: String) -> ChessGame {
        let mut cg = ChessGame {
            pvec: Vec::new(),
            castles: 0b0000,
            turn: false,
            enpassant: None,
            enpassant_piece: None,
            selected: None,
            selected_moves: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            ally_mask: Mask::default(),
            enemy_mask: Mask::default(),
            piece_mask: Mask::default(),
            threatened_squares: Mask::default()
        };
        let mut b_index: i32 = 0;
        let mut index = 0;
        for (i, c) in fen.chars().enumerate() {
            index = i;
            if c == ' ' { 
                break;
            };
            if c == '/' { continue };
            if c.is_digit(10) {
                b_index += c as i32 - '0' as i32;
            } else {
                let point = point(
                    b_index % 8, 7 - b_index / 8
                );
                match c {
                    'r' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Rook { props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::ROOK, has_moved: false }})))),
                    'R' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Rook { props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::ROOK, has_moved: false  }})))),
                    'n' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Knight { props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::KNIGHT, has_moved: false  }})))),
                    'N' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Knight { props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::KNIGHT, has_moved: false  }})))),
                    'b' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Bishop { props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::BISHOP, has_moved: false  }})))),
                    'B' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Bishop { props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::BISHOP, has_moved: false  }})))),
                    'q' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Queen { props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::QUEEN, has_moved: false  }})))),
                    'Q' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Queen { props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::QUEEN, has_moved: false  }})))),
                    'k' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::King { props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::KING, has_moved: false  }})))),
                    'K' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::King { props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::KING, has_moved: false  }})))),
                    'p' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Pawn { 
                        props: piece::PieceProperties { parity: false, pos: point, ptype: PieceType::PAWN, has_moved: !(point.y == 6)}
                    })))),
                    'P' => cg.pvec.push(Rc::new(RefCell::new(Box::new(piece::Pawn { 
                        props: piece::PieceProperties { parity: true, pos: point, ptype: PieceType::PAWN, has_moved: !(point.y == 1)}
                    })))),
                    _ => ()
                }


                b_index += 1;
            }

        }
        cg.turn = fen.chars().nth(index + 1).unwrap() != 'w';
        index += 3; // Skip space, turn char, and another space
        while index < fen.len() && fen.chars().nth(index) != Some(' ') {
            match fen.chars().nth(index).unwrap(){
                'k' => cg.castles |= 0b0001,
                'q' => cg.castles |= 0b0010,
                'K' => cg.castles |= 0b0100,
                'Q' => cg.castles |= 0b1000,
                _ => ()
            }
            index += 1;
        }
        index += 1; // Skip space
        if fen.chars().nth(index) == Some('-') {
            cg.enpassant = None;
            index += 2;
        } else {
            cg.enpassant = Some(algebraic_to_point(&fen[index..(index + 2)]));
            index += 3;
        }
        let mut move_counts = fen[index..].split(' ');
        cg.halfmove_clock = move_counts.nth(0).unwrap_or("0").parse::<u32>().unwrap_or(0);
        cg.fullmove_number = move_counts.nth(0).unwrap_or("1").parse::<u32>().unwrap_or(1);

        cg.update_masks();
        cg.update_threatened_squares();
        return cg;
    }
    pub fn get_piece_at_pos(&self, pos: Point) -> Option<Rc<RefCell<Box<dyn piece::TPiece>>>> {
        for piece in self.pvec.iter() {
            if piece.borrow().get_props().pos == pos {
                return Some(piece.clone());
            }
        }
        return None;
    }

    fn make_move(&mut self, piece: Rc<RefCell<Box<dyn piece::TPiece>>>, to: &Mask) -> bool {
        match to.to_point() {
            Some(point) => {
                self.enpassant = None;
                self.enpassant_piece = None;
                let props = piece.borrow().get_props();
                if props.ptype == PieceType::PAWN {
                    if (point.y - props.pos.y) == 2 {
                        self.enpassant = Some(Point {x: point.x, y: point.y - 1 });
                        self.enpassant_piece = Some(piece.clone());
                    } else if (point.y - props.pos.y) == -2 {
                        self.enpassant = Some(Point {x: point.x, y: point.y + 1 });
                        self.enpassant_piece = Some(piece.clone());
                    }
                }
                let mut mutable = piece.deref().borrow_mut();
                mutable.set_props(
                    PieceProperties {
                        pos: point,
                        parity: props.parity,
                        ptype: props.ptype,
                        has_moved: true
                    }
                );
                return true;
            },
            None => return false
        }
    }
    fn take_piece(&mut self, using: Rc<RefCell<Box<dyn piece::TPiece>>>, take: Rc<RefCell<Box<dyn piece::TPiece>>>, take_position: &Mask) -> bool {
        let success = self.make_move(using, take_position);
        if success {
            self.pvec.retain(|x| {
                return !(**x.borrow() == **take.borrow());
            });
        }
        return success;
    }
    fn end_turn(&mut self, info: MoveInfo) {
        let mut reset_halfmove = false;
        if let Some(_victim) = info.victim {
            reset_halfmove = true;
        }
        if let Some(executor) = info.executor {
            if executor.borrow().get_props().ptype == PieceType::PAWN {
                reset_halfmove = true;
            }
        }
        self.halfmove_clock = if reset_halfmove { 0 } else { self.halfmove_clock + 1 };
        if !self.turn { self.fullmove_number += 1 };
        self.turn = !self.turn;
        self.selected_moves = None;
        self.selected = None;

        self.update_masks();
        self.update_threatened_squares();
    }


    pub fn select(&mut self, pos: Point){
        let selection = self.get_piece_at_pos(pos);
        let mut move_info = MoveInfo {
            was_successful: false,
            executor: None,
            victim: None
        };
        if let Some(next_selection) = &selection {
            if next_selection.borrow().get_props().parity == self.turn {
                self.selected = Some(next_selection.clone());
                self.selected_moves = Some(next_selection.borrow().get_moves(&self.ally_mask, &self.enemy_mask, &self.piece_mask, self));
                if let Some(moves) = self.selected_moves {
                    self.filter_illegals(next_selection.clone(), &moves);

                } 

            } else {
                if let (Some(current_selection), Some(moves)) = (&self.selected, &self.selected_moves) {
                    let mask = *moves & Mask::from_point(next_selection.borrow().get_props().pos);
                    if mask.any() {
                        move_info.executor = Some(current_selection.clone());
                        move_info.victim = Some(next_selection.clone());
                        move_info.was_successful = self.take_piece(current_selection.clone(), next_selection.clone(), &mask);
                    }
                }
            }
        } else {
            if let (Some(current_selection), Some(moves)) = (&self.selected, &self.selected_moves) {
                let move_to_mask = Mask::from_point(pos);
                if let (Some(enpassant_m), Some(enpassant_p)) = (&self.enpassant, &self.enpassant_piece) {
                    if *enpassant_m == pos && (*moves & *enpassant_m).any() {
                        move_info.executor = Some(current_selection.clone());
                        move_info.victim = Some(enpassant_p.clone());
                        move_info.was_successful = self.take_piece(current_selection.clone(), enpassant_p.clone(), &move_to_mask);
                    } else {
                        let mask = *moves & move_to_mask;
                        if mask.any() {
                            move_info.executor = Some(current_selection.clone());
                            move_info.was_successful = self.make_move(current_selection.clone(), &mask);
                        }
                    }
                } else {
                    let mask = *moves & move_to_mask;
                    if mask.any() {
                        move_info.executor = Some(current_selection.clone());
                        move_info.was_successful = self.make_move(current_selection.clone(), &mask);
                    }
                }
            }
        }
        if move_info.was_successful {
            self.end_turn(move_info);
        }
    }

    fn update_ally_mask(&mut self) -> () {
        self.ally_mask ^= self.ally_mask;
        for piece in self.pvec.iter() {
            if piece.borrow().get_props().parity == self.turn {
                self.ally_mask |= piece.borrow().get_props().pos;
            }
        }
    }
    fn update_enemy_mask(&mut self) -> () {
        self.enemy_mask ^= self.enemy_mask;
        for piece in self.pvec.iter() {
            if piece.borrow().get_props().parity != self.turn {
                self.enemy_mask |= piece.borrow().get_props().pos;
            }
        }
    }
    fn update_masks(&mut self) -> () {
        self.update_ally_mask();
        self.update_enemy_mask();
        self.piece_mask = self.ally_mask | self.enemy_mask;
    }
    fn in_check(&self) -> bool {
        for piece in self.pvec.iter() {
            if piece.borrow().get_props().parity == self.turn && piece.borrow().get_props().ptype == PieceType::KING {
                if (Mask::from_point(piece.borrow().get_props().pos) & self.threatened_squares).any() {
                    return true;
                }
            }
        }
        return false;
    }
    fn filter_illegals(&mut self, using: Rc<RefCell<Box<dyn piece::TPiece>>>, moves: &Mask) -> () {
        let piece_pos_mask = Mask::from_point(using.borrow().get_props().pos).not();
        let state = (self.ally_mask, self.enemy_mask, self.piece_mask);
        let future_pieces = self.piece_mask & piece_pos_mask;
        let future_allys_removed = self.ally_mask & piece_pos_mask;
        let future_enemy_removed = self.enemy_mask;


        for &bit in moves.isolated_bits().iter() {
            self.ally_mask = future_allys_removed | bit;
            if (future_enemy_removed & bit).any() {
                self.enemy_mask = future_enemy_removed ^ bit;
            }
            for piece in self.pvec.iter() {
                if piece.borrow().get_props().parity != self.turn {
                    let piece_moves = piece.borrow().get_moves(&self.ally_mask, &self.enemy_mask, &(self.ally_mask | self.enemy_mask), self);
                    pretty_print_future(using.clone(), &bit, piece.clone(), &piece_moves);

                }
            }
        }


    }


    pub fn update_threatened_squares(&mut self) {
        self.threatened_squares ^= self.threatened_squares;
        for piece in self.pvec.iter() {
            if piece.borrow().get_props().parity != self.turn {
                let moves = piece.borrow().get_moves(&self.enemy_mask, &self.ally_mask, &self.piece_mask, self);
                if piece.borrow().get_props().ptype == PieceType::PAWN {
                    self.threatened_squares |= Mask::of_column(piece.borrow().get_props().pos.x).not() & moves;
                } else {
                    self.threatened_squares |= moves;
                }
            }
        }
    }

}
