use crate::Player;
use crate::shared::{
    chessbyte::ChessByte, 
    mask::Mask, 
    piece::{
        Parity, 
        PieceByte
    }, 
    point::{
        algebraic_to_point, 
        Point
    }, 
    state::State

};
use std::rc::Rc;



pub struct ChessGame {
    pub selected: usize,
    pub state: Box<State>,
    pub game_over: bool,
    pub players: (Option<Rc<dyn Player>>, Option<Rc<dyn Player>>),
    pub human_player: Parity,
    pub state_history: Vec<Box<State>>
}

fn get_king_or_queenside(index: usize) -> u8 {
    let modded = index % 8;
    return if modded < 4 { 0b01000000u8 } else if modded > 4 { 0b00100000u8 } else { 0b00000000u8 };
}
impl ChessGame {
    pub fn init(fen: String, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessGame {
        let mut cg = ChessGame {
            selected: 65,
            state: Box::new(State::default()),
            state_history: Vec::new(),
            game_over: false,
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
        let mut black_king_index = 65usize;
        let mut white_king_index = 65usize;
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
                    'r' => cg.state.board[b_index] = Parity::BLACK | PieceByte::ROOK | get_king_or_queenside(b_index),
                    'R' => cg.state.board[b_index] = Parity::WHITE | PieceByte::ROOK | get_king_or_queenside(b_index),

                    'n' => cg.state.board[b_index] = Parity::BLACK | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    'N' => cg.state.board[b_index] = Parity::WHITE | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    
                    'b' => cg.state.board[b_index] = Parity::BLACK | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    'B' => cg.state.board[b_index] = Parity::WHITE | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    
                    'q' => cg.state.board[b_index] = Parity::BLACK | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    'Q' => cg.state.board[b_index] = Parity::WHITE | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    
                    'k' => {
                        black_king_index = b_index;
                        cg.state.board[b_index] = Parity::BLACK | PieceByte::KING | get_king_or_queenside(b_index);
                    },
                    'K' => {
                        white_king_index = b_index;
                        cg.state.board[b_index] = Parity::WHITE | PieceByte::KING | get_king_or_queenside(b_index);
                    },
                    'p' => cg.state.board[b_index] = Parity::BLACK | PieceByte::PAWN | get_king_or_queenside(b_index),
                    'P' => cg.state.board[b_index] = Parity::WHITE | PieceByte::PAWN | get_king_or_queenside(b_index),

                    _ => ()
                    
                }
                b_index += 1;
            }

        }
        cg.state.turn = if fen.chars().nth(index + 1).unwrap() == 'w' { Parity::WHITE } else { Parity::BLACK };
        if cg.state.turn == Parity::WHITE {
            if white_king_index < 64 {
                cg.state.kings = (Mask::from_index(white_king_index), Mask::from_index(black_king_index));
            }
        } else {
            if black_king_index < 64 {
                cg.state.kings = (Mask::from_index(black_king_index), Mask::from_index(white_king_index));
            }
        }
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
        
        cg.state.accept();
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
    pub fn human_input(&mut self, pos: Point, player_parity: Parity) -> () {
        if player_parity == self.state.turn || player_parity == Parity::BOTH {
            let pos_index = pos.to_index();
            let current_selection = self.state.get_piece_at_index(self.selected);
            let piece_at_input = self.state.get_piece_at_index(pos_index);
            if current_selection != 0 {
                let result = self.state.try_accept(self.selected, pos_index);
                if result != usize::MAX {
                    self.selected = 65;
                    self.state_history.push(self.state.purge(result));
                    std::mem::swap(self.state_history.last_mut().unwrap(), &mut self.state);
                    self.state.accept();
                } else {
                    if piece_at_input.get_parity() == self.state.turn {
                        self.selected = pos_index;
                    } else if piece_at_input == 0 {
                        self.selected = 65;
                    }
                }
            } else {
                if piece_at_input != 0 {
                    if piece_at_input.get_parity() == self.state.turn {
                        self.selected = pos_index; 
                    }
                } else {
                    self.selected = 65;
                }
            }

            

        }
    }
}
