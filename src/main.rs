use std::cell::RefCell;
use std::rc::Rc;

use eframe::egui;
use final_1::cutil::pretty_print::{pretty_print_board, pretty_print_mask, pretty_print_masks, pretty_print_moveset};
use final_1::shared::boardarray::BoardArray;
use final_1::shared::chessbyte::ChessByte;
use final_1::shared::eval;
use final_1::shared::heap::{EvaluatedMotion, Heap};
use final_1::shared::motion::Motion;
use final_1::shared::searchtree::SearchTree;
use final_1::shared::mask::Mask;
use final_1::shared::piece::Parity;
use final_1::shared::state::State;
use final_1::ChessApp;
use final_1::player::Player;


const FENS: [&str; 7] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // Default
    "rnbqkbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1", // No Pawns
    "r/8/8/8/8/8/8/R w KQkq - 0 1", // Just rooks,
    "r4k/8/8/8/8/8/8/R4K w KQkq - 0 1", // Check,
    "r4k/8/8/8/8/8/8/RR4K w KQkq - 0 1", // Check,
    "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", //Castling 
    "rnbqkbnr/pppppppp/8/8/8/8/P7/K7 w KQkq - 0 1", // White only pawns
    
];

const WINDOW_SIZE: [f32; 2] = [1800.0, 800.0];
const PLAYING_AREA: f32 = 600.0;


struct PlayerB {
    parity: Parity
}


struct Searcher {
    pub state: Rc<RefCell<State>>,
    pub tree: Vec<Rc<RefCell<SearchTree>>>,
    pub ply: usize,
    pub primary: Parity,
    pub heap: Heap
}
impl Searcher {
    pub fn quiesce(&mut self, state: Rc<RefCell<State>>, mut alpha: i32, beta: i32) -> i32 {
        let evl = eval::start_eval(&state.borrow()).eval;
        if evl >= beta { 
            return beta; 
        }
        if alpha < evl { alpha = evl };
        for i in 0..64 {
            if state.borrow().board[i].is_parity(state.borrow().turn) {
                let moves = state.borrow().moves[i].clone();
                for m in moves.iter() {
                    if state.borrow().board[m.to].is_piece() && state.borrow().board[m.to].is_parity(!state.borrow().board[m.from].get_parity()) {
                        state.borrow_mut().make_motion(m, false);
                        let score = -self.quiesce(state.clone(), -beta, -alpha);
                        state.borrow_mut().unmake_last(true);
                        if score >= beta {
                            return beta;
                        }
                        if score > alpha {
                            alpha = score;
                            // if state.borrow().board[m.from].is_parity(self.primary) {
                                // self.heap.push(EvaluatedMotion { evaluation: alpha, motion: *m } );
                            // }
                        }
                        
                    }
                }
            }
        }
        return alpha;
    }
    pub fn pv(&mut self, state: Rc<RefCell<State>>, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        if self.tree.len() == 0 {
            self.tree.push(SearchTree::root(state.borrow().turn));
        } else {
            let newtree = self.tree[self.ply - depth - 1].borrow_mut().extend(state.borrow().turn);
            self.tree.push(newtree);
        }
        if depth == 0 {
            return eval::start_eval(&state.borrow()).eval;
            // return self.quiesce(state.clone(), alpha, beta);
        }
        let mut best_score = i32::MIN;
        for i in 0..64 {
            if state.borrow().board[i].is_parity(state.borrow().turn) {
                let moves = state.borrow().moves[i].clone();
                for (index, m) in moves.iter().enumerate() {
                    state.borrow_mut().make_motion(m, false);
                    let score = if index == 0 {
                        self.pv(state.clone(), -beta, -alpha, depth - 1)
                    } else {
                        let nws = -self.pv(state.clone(), -alpha - 1, -alpha, depth - 1);
                        if nws > alpha && beta.checked_sub(alpha).is_some_and(|x| x > 1) {
                            self.pv(state.clone(), -beta, -alpha, depth - 1)
                        } else {
                            nws
                        }
                    };
                    state.borrow_mut().unmake_last(true);
                    let current_score = -score;
                    if current_score >= beta {
                        return beta;
                    }
                    if current_score > alpha {
                        alpha = current_score;
                    }
                    if current_score > best_score {
                        best_score = current_score;
                        if depth == self.ply {
                            self.heap.push(EvaluatedMotion { evaluation: best_score, motion: *m } );
                        }
                    }
                }
            }
        }
        return alpha;
    }
}

impl Player for PlayerB {
    fn your_turn(&self, state: Rc<RefCell<State>>) -> Option<Rc<RefCell<SearchTree>>> {
        let mut s = Searcher {
            state: state.clone(),
            tree: Vec::new(),
            ply: 3,
            primary: self.parity,
            heap: Heap::default()
        };
        s.pv(s.state.clone(), i32::MIN + 100, i32::MAX - 100, s.ply);
        println!("Heap peek eval: {}, {} -> {}", s.heap.peek().evaluation, s.heap.peek().motion.from, s.heap.peek().motion.to);
        
        if s.heap.empty() {
            panic!("No move");
        } else {
            state.borrow_mut().make_motion(&s.heap.pop().motion, true);
        }

        return Some(s.tree[0].clone());
    }
    fn get_parity(&self) -> Parity { self.parity }
}

fn main() -> () {
    let pw = PlayerB {
        parity: Parity::WHITE
    };
    let pb = PlayerB {
        parity: Parity::BLACK
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(WINDOW_SIZE).with_position([0.0, 0.0]),
        ..Default::default()
    };
    let finish = eframe::run_native(
        "Chess",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ChessApp::new(
                        cc,
                        PLAYING_AREA,
                        WINDOW_SIZE[0] - PLAYING_AREA,
                        FENS[0],
                        None,
                        // None,
                        // Some(Rc::new(pw)),
                        Some(Rc::new(pb)) 
            )))
        }),
    );

    if let Err(e) = finish {
        dbg!("App exited with error: {:?}", e);
    } else {
        dbg!("Shut down gracefully");
    }
}
