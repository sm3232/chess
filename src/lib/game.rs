
use crate::lib::{
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
    state::State,
    searchtree::SearchTree,
    player::Player

};
use std::{cell::RefCell, sync::{Arc, Mutex}};
use std::rc::Rc;



pub struct ChessGame {
    pub selected: usize,
    pub state: Arc<Mutex<State>>,
    pub game_over: bool,
    pub players: (Option<Arc<dyn Player>>, Option<Arc<dyn Player>>),
    pub human_player: Parity,
    pub state_history: Vec<Arc<Mutex<State>>>,
    pub tree: Option<Arc<Mutex<SearchTree>>>,
    pub paused: bool,
    pub visual_weights: Option<[i32; 64]>
}


fn get_king_or_queenside(index: usize) -> u8 {
    let modded = index % 8;
    return if modded < 4 { 0b01000000u8 } else if modded > 4 { 0b00100000u8 } else { 0b00000000u8 };
}
impl ChessGame {
    pub fn init(fen: String) -> ChessGame {
        let cg = ChessGame {
            selected: 65,
            state: Arc::new(Mutex::new(State::default())),
            state_history: Vec::new(),
            game_over: false,
            human_player: Parity::NONE,
            players: (None, None),
            tree: None,
            paused: false ,
            visual_weights: Some([0i32; 64])
        };
        let mut locked = cg.state.lock().unwrap();


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
                    'r' => locked.board[b_index] = Parity::BLACK | PieceByte::ROOK | get_king_or_queenside(b_index),
                    'R' => locked.board[b_index] = Parity::WHITE | PieceByte::ROOK | get_king_or_queenside(b_index),

                    'n' => locked.board[b_index] = Parity::BLACK | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    'N' => locked.board[b_index] = Parity::WHITE | PieceByte::KNIGHT | get_king_or_queenside(b_index),
                    
                    'b' => locked.board[b_index] = Parity::BLACK | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    'B' => locked.board[b_index] = Parity::WHITE | PieceByte::BISHOP | get_king_or_queenside(b_index),
                    
                    'q' => locked.board[b_index] = Parity::BLACK | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    'Q' => locked.board[b_index] = Parity::WHITE | PieceByte::QUEEN | get_king_or_queenside(b_index),
                    
                    'k' => locked.board[b_index] = Parity::BLACK | PieceByte::KING | get_king_or_queenside(b_index),
                    
                    'K' => locked.board[b_index] = Parity::WHITE | PieceByte::KING | get_king_or_queenside(b_index),
                    
                    'p' => locked.board[b_index] = Parity::BLACK | PieceByte::PAWN | get_king_or_queenside(b_index),
                    'P' => locked.board[b_index] = Parity::WHITE | PieceByte::PAWN | get_king_or_queenside(b_index),

                    _ => ()
                    
                }
                b_index += 1;
            }

        }

        locked.turn = if fen.chars().nth(index + 1).unwrap() == 'w' { Parity::WHITE } else { Parity::BLACK };
        index += 3; // Skip space, turn char, and another space
        while index < fen.len() && fen.chars().nth(index) != Some(' ') {
            match fen.chars().nth(index).unwrap(){
                'k' => locked.info.allowed_castles |= 0b00000001,
                'q' => locked.info.allowed_castles |= 0b00000010,
                'K' => locked.info.allowed_castles |= 0b00000100,
                'Q' => locked.info.allowed_castles |= 0b00001000,
                _ => ()
            }
            index += 1;
        }
        index += 1; // Skip space
        if fen.chars().nth(index) == Some('-') {
            index += 2;
        } else {
            locked.info.enpassant_mask = Mask::from_point(algebraic_to_point(&fen[index..(index + 2)]));
            index += 3;
        }
        let mut move_counts = fen[index..].split(' ');
        locked.info.halfmove_clock = move_counts.nth(0).unwrap_or("0").parse::<u64>().unwrap_or(0);
        locked.info.fullmove_number = move_counts.nth(0).unwrap_or("1").parse::<u64>().unwrap_or(1);
    
        locked.init();
        drop(locked);
        return cg;
    }

    /*
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
    */
    /*
    pub fn poll_players(&mut self) -> bool {
        let option_player = if self.state.borrow().turn == Parity::WHITE { &self.players.0 } else { &self.players.1 };
        if let Some(player) = option_player {
            let optional_tree = player.your_turn(self.state.clone());
            self.tree = optional_tree;
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
    */
    pub fn register_players(&mut self, p1: Option<Arc<dyn Player>>, p2: Option<Arc<dyn Player>>) -> () {
        self.players.0 = p1;
        self.players.1 = p2;
        match (self.players.0.is_some(), self.players.1.is_some()) {
            (true, true) => self.human_player = Parity::NONE,
            (true, false) => self.human_player = !self.players.0.clone().unwrap().get_parity(),
            (false, true) => self.human_player = !self.players.1.clone().unwrap().get_parity(),
            (false, false) => self.human_player = Parity::BOTH
        };

    }
    pub fn human_input(&mut self, pos: Point, player_parity: Parity) -> () {
        let locked = &mut self.state.lock().unwrap();
        if player_parity == locked.turn || player_parity == Parity::BOTH {
            let pos_index = pos.to_index();
            let current_selection = locked.get_piece_at_index(self.selected);
            let piece_at_input = locked.get_piece_at_index(pos_index);
            if current_selection != 0 && self.selected != 65 {
                if locked.board[self.selected].get_parity() == locked.turn {
                    let moves = locked.moves[self.selected].clone();
                    for m in moves.iter() {
                        if m.to == pos_index {
                            locked.make_motion(m, true);
                            self.selected = 65;
                            break;
                        }
                    }
                    if self.selected != 65 {
                        if piece_at_input != 0 && pos_index != 65 && piece_at_input.get_parity() == locked.turn{
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
                    if piece_at_input.get_parity() == locked.turn {
                        self.selected = pos_index; 
                    }
                } else {
                    self.selected = 65;
                }
            }
        }
        drop(locked);
    }
}
