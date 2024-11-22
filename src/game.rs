use crate::{player::Player, shared::{
    mask::Mask, piece::{
        Parity, PieceByte, PieceCachedMoves
    }, point::{
        algebraic_to_point, Point
    }, state::{ChessByte, State}
}};
use std::rc::Rc;



pub struct ChessGame {
    pub selected: usize,
    pub state: State,
    pub game_over: bool,
    pub players: (Option<Rc<dyn Player>>, Option<Rc<dyn Player>>),
    pub human_player: Parity,
    white_kingside_mask: Mask,
    white_queenside_mask: Mask,
    black_kingside_mask: Mask,
    black_queenside_mask: Mask
}

fn get_king_or_queenside(index: usize) -> u8 {
    let modded = index % 8;
    return if modded < 4 { 0b01000000u8 } else if modded > 4 { 0b00100000u8 } else { 0b00000000u8 };
}
impl ChessGame {
    pub fn init(fen: String, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessGame {
        let mut cg = ChessGame {
            selected: 65,
            state: State::default(),
            game_over: false,
            white_kingside_mask: Mask::from_point(Point { x: 4, y: 0 }) | Mask::from_point(Point { x: 5, y: 0 }) | Mask::from_point(Point { x: 6, y: 0 }),
            white_queenside_mask: Mask::from_point(Point { x: 2, y: 0 }) | Mask::from_point(Point { x: 3, y: 0 }) | Mask::from_point(Point { x: 4, y: 0 }),
            black_kingside_mask: Mask::from_point(Point { x: 4, y: 7 }) | Mask::from_point(Point { x: 5, y: 7 }) | Mask::from_point(Point { x: 6, y: 7 }),
            black_queenside_mask: Mask::from_point(Point { x: 2, y: 7 }) | Mask::from_point(Point { x: 3, y: 7 }) | Mask::from_point(Point { x: 4, y: 7 }),
            human_player: Parity::NONE,
            players: (player_white, player_black),
        };

        match (cg.players.0.is_some(), cg.players.1.is_some()) {
            (true, true) => cg.human_player = Parity::NONE,
            (true, false) => cg.human_player = !cg.players.0.clone().unwrap().get_parity(),
            (false, true) => cg.human_player = !cg.players.1.clone().unwrap().get_parity(),
            (false, false) => cg.human_player = Parity::BOTH
        };

        let mut b_index: usize = 0;
        let mut index = 0;
        for (i, c) in fen.chars().enumerate() {
            index = i;
            if c == ' ' { 
                break;
            };
            if c == '/' { continue };
            if c.is_digit(10) {
                b_index += (c as i32 - '0' as i32) as usize;
            } else {
                match c {
                    'r' => cg.state.board[b_index] = Parity::WHITE | PieceByte::ROOK | get_king_or_queenside(b_index),
                    'R' => cg.state.board[b_index] = Parity::BLACK | PieceByte::ROOK | get_king_or_queenside(b_index),

                    'n' => cg.state.board[b_index] = Parity::WHITE | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    'N' => cg.state.board[b_index] = Parity::BLACK | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    
                    'b' => cg.state.board[b_index] = Parity::WHITE | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    'B' => cg.state.board[b_index] = Parity::BLACK | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    
                    'q' => cg.state.board[b_index] = Parity::WHITE | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    'Q' => cg.state.board[b_index] = Parity::BLACK | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    
                    'k' => cg.state.board[b_index] = Parity::WHITE | PieceByte::KING | get_king_or_queenside(b_index),
                    'K' => cg.state.board[b_index] = Parity::BLACK | PieceByte::KING | get_king_or_queenside(b_index),
                    
                    'p' => cg.state.board[b_index] = Parity::WHITE | PieceByte::PAWN | get_king_or_queenside(b_index),
                    'P' => cg.state.board[b_index] = Parity::BLACK | PieceByte::PAWN | get_king_or_queenside(b_index),

                    _ => ()
                    
                }
                b_index += 1;
            }

        }
        cg.state.turn = if fen.chars().nth(index + 1).unwrap() == 'w' { Parity::WHITE } else { Parity::BLACK };
        index += 3; // Skip space, turn char, and another space
        while index < fen.len() && fen.chars().nth(index) != Some(' ') {
            match fen.chars().nth(index).unwrap(){
                'k' => cg.state.castles |= 0b00000001,
                'q' => cg.state.castles |= 0b00000010,
                'K' => cg.state.castles |= 0b00000100,
                'Q' => cg.state.castles |= 0b00001000,
                _ => ()
            }
            index += 1;
        }
        index += 1; // Skip space
        if fen.chars().nth(index) == Some('-') {
            index += 2;
        } else {
            cg.state.enpassant = Mask::from_point(algebraic_to_point(&fen[index..(index + 2)]));
            index += 3;
        }
        let mut move_counts = fen[index..].split(' ');
        cg.state.halfmove_clock = move_counts.nth(0).unwrap_or("0").parse::<u64>().unwrap_or(0);
        cg.state.fullmove_number = move_counts.nth(0).unwrap_or("1").parse::<u64>().unwrap_or(1);

        cg.state.refresh(None);

        return cg;
    }

    pub fn poll_players(&mut self) -> bool {
        /*
        if self.turn == Parity::WHITE {
            if let Some(player) = &self.players.0 {
                let (from_mask, to_mask) = player.your_turn(&self.cached_moves, &self.pvec, &self.ally_mask, &self.enemy_mask, &self.piece_mask);
                self.player_request_move(from_mask, to_mask);
                return true;
            }
        } else {
            if let Some(player) = &self.players.1 {
                let (from_mask, to_mask) = player.your_turn(&self.cached_moves, &self.pvec, &self.ally_mask, &self.enemy_mask, &self.piece_mask);
                self.player_request_move(from_mask, to_mask);
                return true;
            }
        }
        */
        return false;
    }
    fn remove_piece_from_game(&mut self, piece_to_remove: usize) {
        self.state.moves[piece_to_remove] = PieceCachedMoves::default();
        self.state.board[piece_to_remove] = 0u8;
    }
    fn try_take(&mut self, using: usize, take: usize) -> bool {
        if (self.state.moves[using].moves & Mask::from_index(take)).any() {
            self.remove_piece_from_game(take);
            self.state.board[take] = self.state.board[using];
            self.state.board[using] = 0u8;
            return true;
        }
        return false;
    }
    fn try_move(&mut self, using: usize, to: usize) -> bool {
        if (self.state.moves[using].moves & Mask::from_index(to)).any() {
            self.state.board[to] = self.state.board[using];
            self.state.board[using] = 0u8;
            return true;
        }
        return false;
    }
    fn try_castle(&mut self, king: usize, rook: usize) -> bool {
        let kp = self.state.board[king];
        let km = self.state.moves[king];
        let castle_bits = if kp.get_parity() == Parity::WHITE { km.castles & 0b00000011 } else { (km.castles & 0b00001100) >> 2 };
        if (rook % 8) > (king % 8) {
            if (castle_bits & 0b00000001) != 0 {
                self.state.board[king + 2] = self.state.board[king];
                self.state.board[king] = 0u8;
                self.state.board[rook - 2] = self.state.board[rook];
                self.state.board[rook] = 0u8;
                return true;
            }
        } else {
            if (castle_bits & 0b00000010) != 0 {
                self.state.board[king - 2] = self.state.board[king];
                self.state.board[king] = 0u8;
                self.state.board[rook + 3] = self.state.board[rook];
                self.state.board[rook] = 0u8;
                return true;
            }
        }
        return false;
    }
    pub fn human_input(&mut self, pos: Point, player_parity: Parity) -> () {
        if player_parity == self.state.turn || player_parity == Parity::BOTH {
            let piece_at_input = self.state.get_piece_at_index(pos.to_index());
            let current_selection = self.state.get_piece_at_index(self.selected);
            dbg!(self.state.turn);
            let mut result = (false, -1, 65);
            if current_selection != 0 {
                if piece_at_input != 0 {
                    if piece_at_input.get_parity() != current_selection.get_parity() {
                        result = (self.try_take(self.selected, pos.to_index()), 1, pos.to_index());
                    } else {
                        if piece_at_input.get_parity() == self.state.turn {
                            if pos.to_index() != self.selected && piece_at_input.get_ptype() == PieceByte::ROOK && current_selection.get_ptype() == PieceByte::KING && self.state.moves[self.selected].castles != 0 {
                                result = (self.try_castle(self.selected, pos.to_index()), 2, pos.to_index());
                            } else {
                                self.selected = pos.to_index();
                            }
                        }
                    }
                } else {
                    result = (self.try_move(self.selected, pos.to_index()), 0, pos.to_index());
                }
            } else {
                if piece_at_input != 0 && piece_at_input.get_parity() == self.state.turn {
                    self.selected = pos.to_index();
                }
            }
            if result.0 {
                self.end_turn(result.1, result.2);
            }
        }
    }
    fn end_turn(&mut self, move_code: i32, context: usize) {

        self.selected = 65;

        self.state.refresh(None);
    }
}
