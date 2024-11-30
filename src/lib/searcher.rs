use std::sync::{Arc, Mutex};
use crate::lib::{
    eval,
    searchtree::SearchTree,
    chessbyte::ChessByte,
    piece::Parity,
};
use super::{heap::{EvaluatedMotion, Heap}, state::State};

pub struct Searcher {
    pub tree: Vec<Arc<Mutex<SearchTree>>>,
    pub ply: usize,
    pub primary: Parity,
    pub heap: Heap
}
impl Searcher {
    pub fn quiesce(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32) -> i32 {
        let lock = state.lock().unwrap();
        let evl = eval::start_eval(&lock).eval;
        drop(lock);
        if evl >= beta {
            return beta; 
        }
        if alpha < evl { alpha = evl };
        for i in 0..64 {
            let mut ilsl = state.lock().unwrap();
            if ilsl.board[i].is_parity(ilsl.turn) {
                let moves = ilsl.moves[i].clone();
                for m in moves.iter() {
                    if ilsl.board[m.to].is_piece() && ilsl.board[m.to].is_parity(!ilsl.board[m.from].get_parity()) {
                        ilsl.make_motion(m, false);
                        let score = -self.quiesce(state.clone(), -beta, -alpha);
                        ilsl.unmake_last(true);
                        if score >= beta {
                            drop(ilsl);
                            return beta;
                        }
                        if score > alpha {
                            alpha = score;
                            // if state.board[m.from].is_parity(self.primary) {
                                // self.heap.push(EvaluatedMotion { evaluation: alpha, motion: *m } );
                            // }
                        }
                        
                    }
                }
            }
            drop(ilsl);
        }
        return alpha;
    }
    pub fn pv(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        let mut state_lock = state.lock().unwrap();
        if self.tree.len() == 0 {
            let r = SearchTree::root(state_lock.turn);
            self.tree.push(r.clone());
            state_lock.tree_root = Some(r.clone());
        } else {
            let mut lock = self.tree[self.ply - depth - 1].lock().unwrap();
            let nt = lock.extend(state_lock.turn);
            drop(lock);
            self.tree.push(nt);
        }
        if depth == 0 {
            return eval::start_eval(&state_lock).eval;
            // return self.quiesce(state.clone(), alpha, beta);
        }
        drop(state_lock);
        let mut best_score = i32::MIN;
        for i in 0..64 {
            let mut ilsl = state.lock().unwrap();
            if ilsl.board[i].is_parity(ilsl.turn) {
                let moves = ilsl.moves[i].clone();
                for (index, m) in moves.iter().enumerate() {
                    ilsl.make_motion(m, false);
                    drop(ilsl);
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
                    ilsl = state.lock().unwrap();
                    ilsl.unmake_last(true);
                    let current_score = -score;
                    if current_score >= beta {
                        drop(ilsl);
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
            drop(ilsl);
        }
        return alpha;
    }
}
