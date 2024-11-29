use crate::shared::searchtree::SearchTree;
use crate::shared::zobrist::Zobrist;
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
use std::cell::RefCell;
use std::rc::Rc;


pub struct ChessGame {
    pub selected: usize,
    pub state: Rc<RefCell<State>>,
    pub game_over: bool,
    pub players: (Option<Rc<dyn Player>>, Option<Rc<dyn Player>>),
    pub human_player: Parity,
    pub state_history: Vec<Rc<RefCell<State>>>,
    pub trees: Vec<Rc<RefCell<SearchTree>>>,
    pub paused: bool,
    pub visual_weights: Option<[i32; 64]>
}


fn get_king_or_queenside(index: usize) -> u8 {
    let modded = index % 8;
    return if modded < 4 { 0b01000000u8 } else if modded > 4 { 0b00100000u8 } else { 0b00000000u8 };
}
impl ChessGame {
    pub fn init(fen: String, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessGame {
        let mut cg = ChessGame {
            selected: 65,
            state: Rc::new(RefCell::new(State::default())),
            state_history: Vec::new(),
            game_over: false,
            human_player: Parity::NONE,
            players: (player_white, player_black),
            trees: Vec::new(),
            paused: false ,
            visual_weights: Some([0i32; 64])
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
                    'r' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::ROOK | get_king_or_queenside(b_index),
                    'R' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::ROOK | get_king_or_queenside(b_index),

                    'n' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    'N' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    
                    'b' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    'B' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    
                    'q' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    'Q' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    
                    'k' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::KING | get_king_or_queenside(b_index),
                    
                    'K' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::KING | get_king_or_queenside(b_index),
                    
                    'p' => cg.state.borrow_mut().board[b_index] = Parity::BLACK | PieceByte::PAWN | get_king_or_queenside(b_index),
                    'P' => cg.state.borrow_mut().board[b_index] = Parity::WHITE | PieceByte::PAWN | get_king_or_queenside(b_index),

                    _ => ()
                    
                }
                b_index += 1;
            }

        }
        cg.state.borrow_mut().turn = if fen.chars().nth(index + 1).unwrap() == 'w' { Parity::WHITE } else { Parity::BLACK };
        index += 3; // Skip space, turn char, and another space
        while index < fen.len() && fen.chars().nth(index) != Some(' ') {
            match fen.chars().nth(index).unwrap(){
                'k' => cg.state.borrow_mut().info.allowed_castles |= 0b00000001,
                'q' => cg.state.borrow_mut().info.allowed_castles |= 0b00000010,
                'K' => cg.state.borrow_mut().info.allowed_castles |= 0b00000100,
                'Q' => cg.state.borrow_mut().info.allowed_castles |= 0b00001000,
                _ => ()
            }
            index += 1;
        }
        index += 1; // Skip space
        if fen.chars().nth(index) == Some('-') {
            index += 2;
        } else {
            cg.state.borrow_mut().info.enpassant_mask = Mask::from_point(algebraic_to_point(&fen[index..(index + 2)]));
            index += 3;
        }
        let mut move_counts = fen[index..].split(' ');
        cg.state.borrow_mut().info.halfmove_clock = move_counts.nth(0).unwrap_or("0").parse::<u64>().unwrap_or(0);
        cg.state.borrow_mut().info.fullmove_number = move_counts.nth(0).unwrap_or("1").parse::<u64>().unwrap_or(1);
    
        cg.state.borrow_mut().init();

        return cg;
    }

    pub fn request_move(&mut self, from: &Mask, to: &Mask) {
        let from_index = from.as_index();
        let from_piece = self.state.borrow().get_piece_at_index(from_index);
        if from_piece != 0 {
            // let result = self.state.borrow_mut().try_accept(from_index, to_index);
            // if result != usize::MAX {
                // self.state_history.push(self.state.borrow_mut().purge(result));
                // std::mem::swap(self.state_history.last_mut().unwrap(), &mut self.state);
                // self.state.borrow_mut().accept();
            // }
        }
    }
    pub fn poll_players(&mut self) -> bool {
        let option_player = if self.state.borrow().turn == Parity::WHITE { &self.players.0 } else { &self.players.1 };
        if let Some(player) = option_player {
            let optional_tree = player.your_turn(self.state.clone());
            if let Some(tree) = optional_tree {
                self.trees.clear();
                self.trees.push(tree);
            }
            // let pm = &player.your_turn());
            if !self.paused {
                // let result = self.state.borrow_mut().try_accept_via_board(&pm);
                // if result != usize::MAX {
                    // self.state_history.push(self.state.borrow_mut().purge(result));
                    // std::mem::swap(self.state_history.last_mut().unwrap(), &mut self.state);
                    // self.state.borrow_mut().accept();
                // }
            }
            return true;
        }
        return false;
    }
    pub fn human_input(&mut self, pos: Point, player_parity: Parity) -> () {
        if player_parity == self.state.borrow().turn || player_parity == Parity::BOTH {
            let pos_index = pos.to_index();
            let current_selection = self.state.borrow().get_piece_at_index(self.selected);
            let piece_at_input = self.state.borrow().get_piece_at_index(pos_index);
            if current_selection != 0 && self.selected != 65 {
                let board = self.state.borrow().board;
                if board[self.selected].get_parity() == self.state.borrow().turn {
                    let moves = self.state.borrow().moves[self.selected].clone();
                    for m in moves.iter() {
                        if m.to == pos_index {
                            self.state.borrow_mut().make_motion(m, true);
                            self.selected = 65;
                            break;
                        }
                    }
                    if self.selected != 65 {
                        if piece_at_input != 0 && pos_index != 65 && piece_at_input.get_parity() == self.state.borrow().turn{
                            self.selected = pos_index;
                        }
                    }
                }

                    /*
                    let result = self.state.borrow_mut().try_accept(self.selected, pos_index);
                    if result != usize::MAX {
                        self.selected = 65;
                        self.state_history.push(self.state.borrow_mut().purge(result));
                        std::mem::swap(self.state_history.last_mut().unwrap(), &mut self.state);
                        self.state.borrow_mut().accept();
                    } else {
                        if piece_at_input.get_parity() == self.state.borrow().turn {
                            self.selected = pos_index;
                        } else if piece_at_input == 0 {
                            self.selected = 65;
                        }
                    }
                    */
            } else {
                if piece_at_input != 0 {
                    if piece_at_input.get_parity() == self.state.borrow().turn {
                        self.selected = pos_index; 
                    }
                } else {
                    self.selected = 65;
                }
            }
        }
    }
}
