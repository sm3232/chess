use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex, MutexGuard}, time};
use crate::lib::{
    chessbyte::ChessByte, eval, motion::Motion, piece::Parity, searchtree::SearchTree
};
use super::{heap::{EvaluatedMotion, Heap}, mask::Mask, state::State};

#[derive(Debug)]
pub struct SearchCheckIn {
    pub tree: SearchTree,
    pub cache_saves: usize,
    pub positions_looked_at: usize,
    pub mtm: Motion,
    pub considerations: Vec<EvaluatedMotion>
}
impl Default for SearchCheckIn {
    fn default() -> Self {
        Self {
            tree: SearchTree::default(),
            cache_saves: 0,
            positions_looked_at: 0,
            mtm: Motion::default(),
            considerations: Vec::new()
        }
    }
}

pub struct SearchDriver {
    pub parity: Parity,
    pub depth: u8,
    pub nodes: u64,
    pub q_nodes: u64,
    pub cache_saves: usize,
    pub positions_looked_at: usize,
    pub comm: Option<crossbeam_channel::Sender<SearchCheckIn>>,
    pub tree: SearchTree,
    pub time_remaining: time::Duration,
    pub time_start: time::Instant,
    pub considerations: Vec<EvaluatedMotion>
}

impl SearchDriver {
    pub fn clear(&mut self, side_to_move: Parity, time_limit: &time::Duration) {
        self.parity = side_to_move;
        self.depth = 0;
        self.nodes = 0;
        self.q_nodes = 0;
        self.cache_saves = 0;
        self.positions_looked_at = 0;
        self.tree = SearchTree::new(side_to_move);
        self.time_remaining = time_limit.clone();
        self.time_start = time::Instant::now();
        self.considerations.clear();
    }
    pub fn communicate_on(&mut self, comms: crossbeam_channel::Sender<SearchCheckIn>) -> () {
        self.comm = Some(comms);
    }
    pub fn communicate(&self, mtm: &Motion) -> () {
        if let Some(channel) = &self.comm {
            let ci = SearchCheckIn {
                positions_looked_at: self.positions_looked_at.clone(),
                cache_saves: self.cache_saves.clone(),
                tree: self.tree.clone(),
                mtm: *mtm,
                considerations: self.considerations.clone(),
            };
            let _ = channel.send(ci).unwrap();
        }
    }
}
impl Default for SearchDriver { fn default() -> Self { Self { cache_saves: 0, positions_looked_at: 0, depth: 0, nodes: 0, q_nodes: 0, parity: Parity::WHITE, comm: None, tree: SearchTree::default(), time_remaining: time::Duration::default(), time_start: time::Instant::now(), considerations: Vec::new() } } }

pub struct Searcher {
    pub tree: Vec<Arc<Mutex<SearchTree>>>,
    pub tt: HashMap<u64, EvaluatedMotion>,
    pub driver: SearchDriver,
    pub mtm: Motion,
    pub echo: HashSet<u64>,
    pub time_limit: time::Duration
}
impl Searcher {
    const ABSOLUTELY_MAX_DEPTH: u8 = 100;
    const ASPIRATION_ADJUSTMENT: i32 = 50;
    const MATERIAL_EVAL_CUTOFF: i32 = 1300;
    const CONTEMPT_VAL: i32 = -10;
    pub fn run(&mut self, state: Arc<Mutex<State>>) -> Motion {
        let lock = state.lock().unwrap();
        let parity = lock.turn;
        drop(lock);
        self.driver.clear(parity, &self.time_limit);
        // calc_movetime();
        // age_history_table();
        self.driver.communicate(&self.mtm);

        let result = self.iterate(state.clone());
        self.driver.communicate(&self.mtm);
        return result;
    }
    fn analyze(&mut self, state: Arc<Mutex<State>>, mut depth: u8, ply: usize, mut alpha: i32, mut beta: i32, null: bool, pv: bool) -> i32 {

        self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
        if self.driver.time_remaining.is_zero() {
            return 0;
        }
        self.driver.positions_looked_at += 1;
        self.driver.communicate(&self.mtm);
        let mut val = i32::MIN + 1;
        let mate = i32::MAX - ply as i32;
        if alpha < -mate {
            alpha = -mate;
        }
        if beta > mate - 1 {
            beta = mate - 1;
        }
        if alpha >= beta {
            return alpha;
        }
        let mut lock = state.lock().unwrap();
        let in_check = (lock.moves.parity_flat(!lock.turn) & Mask::from_index(lock.get_king(lock.turn))).any();
        if in_check { depth += 1 };
        if depth == 0 {
            drop(lock);
            return self.quiescence(state.clone(), alpha, beta);
        }
        self.driver.nodes += 1;

        let scalar = if lock.turn == self.driver.parity { -1 } else { 1 };

        if self.echo.contains(&lock.info.zkey) {
            // Consider drawing. Have we seen this position before?
            self.driver.cache_saves += 1;
            if scalar * eval::material::price_parity(&lock.board, self.driver.parity) < Self::MATERIAL_EVAL_CUTOFF {
                drop(lock);
                return 0;
            } else {
                if lock.turn == self.driver.parity {
                    drop(lock);
                    return Self::CONTEMPT_VAL;
                } else {
                    drop(lock);
                    return -Self::CONTEMPT_VAL;
                }
            }
        }

        if !pv || (val > alpha && val < beta) {
            // Try and save time by caching moves
            if let Some(saved) = self.tt.get(&lock.info.zkey) {
                self.driver.cache_saves += 1;
                drop(lock);
                return saved.evaluation;
            }
        }
        if depth < 3 && !pv && !in_check {
            // Reverse futility prune 
            // When at a low depth, if the motion doens't do much for us (margin), then just estimate
            // the value and move on
            let eval_static = scalar * eval::start_eval(&lock).eval;
            let margin = 120 * depth as i32;
            if eval_static - margin >= beta {
                drop(lock);
                return eval_static - margin;
            }
        }
        if depth > 2 && null && !pv && !in_check && scalar * eval::start_eval(&lock).eval >= beta && scalar * eval::material::price_parity(&lock.board, lock.turn) > Self::MATERIAL_EVAL_CUTOFF {
            // Null move
            // If allowing the opponent to move twice in a row isn't horrible for us, then we can
            // assume there is no point in searching further.
            lock.make_motion(&Motion { from: 65, to: 65 }, false);
            drop(lock);
            val = -self.analyze(state.clone(), depth - if depth > 6 { 4 } else { 3 }, ply, -beta, -beta + 1, false, false);
            lock = state.lock().unwrap();
            lock.unmake_last(true);
            if val >= beta {
                drop(lock);
                return beta;
            }
        }

        let futile_margin: [i32; 4] = [0, 200, 300, 500];
        // Futility prune flag 
        // If true, we don't really focus on non-tactical moves 
        // tactical = captures, promotions, moves that change material value of the board.
        let futility_prune = depth < 4 && !pv && !in_check && alpha.abs() < 9000 && scalar * eval::start_eval(&lock).eval + futile_margin[depth as usize] <= alpha;

        let mut heap = Heap::default();
        let mut raised = false;
        let mut best = EvaluatedMotion::default();
        let mut moves_tried = 0;

        let moves = lock.moves.parity_vect(lock.turn);
        // Evaluate moves
        for m in &moves {
            lock.make_motion(m, false);
            if (lock.moves.parity_flat(lock.turn) & Mask::from_index(lock.get_king(!lock.turn))).none() {
                heap.push(EvaluatedMotion { evaluation: scalar * eval::start_eval(&lock).eval, motion: *m, key: lock.info.zkey });
            }
            lock.unmake_last(true);
        }
        // Sort state's vector
        lock.set_sorted_motions(heap.to_sorted_motions());
        drop(lock);
        'outer: loop {
            if heap.empty() { break 'outer };
            let motion = heap.pop();
            let mut lock = state.lock().unwrap();
            let promotion = self.is_promotion(&lock, &motion.motion);
            let capture = self.is_capture(&lock, &motion.motion);
            lock.make_motion(&motion.motion, false);
            moves_tried += 1;
            SearchTree::leaf(&mut self.driver.tree, lock.turn);
            drop(lock);


            if futility_prune && !capture && !promotion {
                // Move is futile. Undo
                let mut lock = state.lock().unwrap();
                lock.unmake_last(true);
                SearchTree::back(&mut self.driver.tree, false);
                drop(lock);
                continue 'outer;
            }
            
            let mut reduce = 0;
            let mut ndepth = depth - 1;

            if !pv && ndepth > 3 && moves_tried > 3 && !capture && !promotion {
                reduce = 1;
                if moves_tried > 8 {
                    reduce += 1;
                }
                ndepth -= reduce;
            }

            'research: loop {
                // Principal variation search
                if !raised {
                    val = -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, pv);
                } else {
                    if -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, false) > alpha {
                        val = -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, true);
                    }
                }
                if reduce > 0 && val > alpha {
                    // We reduced and val > alpha? Uncertain- re-search
                    ndepth += reduce;
                    reduce = 0;
                    continue 'research;
                }
                let mut lock = state.lock().unwrap();
                lock.unmake_last(true);
                SearchTree::back(&mut self.driver.tree, false);
                drop(lock);
                if val > alpha {
                    best = motion;
                    if val >= beta {
                        /*
                        if !capture && !promotion {
                            self.driver.killer(motion, ply);
                        }
                        */
                        alpha = beta;
                        self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
                        if self.driver.time_remaining.is_zero() {
                            return 0;
                        }
                        break 'outer;
                    }
                    raised = true;
                    alpha = val;
                }
                break 'research;
            }
            self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
            if self.driver.time_remaining.is_zero() {
                return 0;
            }
        }
        self.tt.insert(best.key, best);

        return alpha;
    }
    fn is_promotion(&self, state: &MutexGuard<'_, State>, m: &Motion) -> bool {
        return state.board[m.from].is_pawn() && (m.to < 8 || m.to > 55);
    }
    fn is_capture(&self, state: &MutexGuard<'_, State>, m: &Motion) -> bool {
        return state.board[m.to].is_piece() && state.board[m.to].is_parity(!state.board[m.from].get_parity());
    }
    fn is_capture_bad(&self, state: &MutexGuard<'_, State>, m: &Motion) -> bool {
        if state.board[m.from].is_pawn() { return false };
        let scalar = if state.turn == self.driver.parity { -1 } else { 1 };
        if scalar * eval::material::price_piece(state.board[m.to]) >= scalar * eval::material::price_piece(state.board[m.from]) - 50 {
            return false;
        }
        let mask = Mask::from_index(m.to);
        if state.board[m.from].is_white() {
            let dgs = mask.get_diags_above();
            if (dgs.0.any() && state.board[dgs.0].is_b_pawn()) || (dgs.1.any() && state.board[dgs.1].is_b_pawn()) {
                return true;
            }
        } else {
            let dgs = mask.get_diags_below();
            if (dgs.0.any() && state.board[dgs.0].is_w_pawn()) || (dgs.1.any() && state.board[dgs.1].is_w_pawn()) {
                return true;
            }
        }
        return false;
    }
    // fn is_repetition(&self, state)
    fn quiescence(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32) -> i32 {
        self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
        if self.driver.time_remaining.is_zero() {
            return 0;
        }
        self.driver.positions_looked_at += 1;
        self.driver.communicate(&self.mtm);
        if self.driver.nodes % 1024 == 0 {

            return 0;
        }
        self.driver.nodes += 1;
        self.driver.q_nodes += 1;

        let mut lock = state.lock().unwrap();
        let scalar = if lock.turn == self.driver.parity { -1 } else { 1 };
        let mut val = scalar * eval::start_eval(&lock).eval;
        let standing = val;

        if val >= beta {
            drop(lock);
            return beta;
        }
        if alpha < val {
            alpha = val;
        }
        let moves = lock.moves.parity_vect(lock.turn);
        let mut heap = Heap::default();
        for m in &moves {
            if !lock.board[m.to].is_piece() || lock.board[m.to].is_parity(lock.board[m.from].get_parity()) {
                continue;
            }
            if lock.board[m.to].is_king() {
                drop(lock);
                return i32::MAX - 1;
            }
            let is_promo = self.is_promotion(&lock, &m);
            if standing - scalar * eval::material::price_piece(lock.board[m.to]) + 200 < alpha &&
                scalar * eval::material::price_parity(&lock.board, !lock.turn) - scalar * eval::material::price_piece(lock.board[m.to]) > Self::MATERIAL_EVAL_CUTOFF && 
                    !is_promo {
                        continue;
            }
            if !is_promo && self.is_capture_bad(&lock, &m) {
                continue;
            }
            lock.make_motion(m, false);
            if (lock.moves.parity_flat(lock.turn) & Mask::from_index(lock.get_king(!lock.turn))).none() {
                heap.push(EvaluatedMotion { evaluation: scalar * eval::start_eval(&lock).eval, motion: *m, key: lock.info.zkey });
            }
            lock.unmake_last(true);
        }
        drop(lock);
        loop {
            if heap.empty() { break };
            let motion = heap.pop();
            lock = state.lock().unwrap();
            lock.make_motion(&motion.motion, false);
            SearchTree::leaf(&mut self.driver.tree, lock.turn);
            drop(lock);
            val = -self.quiescence(state.clone(), -beta, -alpha);
            lock = state.lock().unwrap();
            lock.unmake_last(true);
            SearchTree::back(&mut self.driver.tree, false);
            drop(lock);
            self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
            if self.driver.time_remaining.is_zero() {
                return 0;
            }
            if val > alpha {
                if val >= beta { return beta };
                alpha = val;
            }
        }
        return alpha;
    }
    fn sroot(&mut self, state: Arc<Mutex<State>>, mut depth: u8, mut alpha: i32, beta: i32) -> i32 {
        self.driver.considerations.clear();
        self.driver.communicate(&self.mtm);
        let mut val = 0;
        let mut best = EvaluatedMotion::default();
        let mut lock = state.lock().unwrap();
        let in_check = (lock.moves.parity_flat(!lock.turn) & Mask::from_index(lock.get_king(lock.turn))).any();
        if in_check { depth += 1 };
        let moves = lock.moves.parity_vect(lock.turn);
        let mut heap = Heap::default();
        let scalar = if lock.turn == self.driver.parity { -1 } else { 1 };
        for m in &moves {
            lock.make_motion(m, false);
            if (lock.moves.parity_flat(lock.turn) & Mask::from_index(lock.get_king(!lock.turn))).none() {
                heap.push(EvaluatedMotion { evaluation: scalar * eval::start_eval(&lock).eval, motion: *m, key: lock.info.zkey });
            }
            lock.unmake_last(true);
        }
        lock.set_sorted_motions(heap.to_sorted_motions());
        self.driver.considerations = heap.to_sorted_evaluated_motions();

        drop(lock);
        let mut i = 0;
        loop {
            if heap.empty() { break };
            let motion = heap.pop();
            lock = state.lock().unwrap();
            if lock.board[motion.motion.to].is_king() {
              alpha = i32::MAX - 1;
              best = motion;
            }
            lock.make_motion(&motion.motion, false);
            SearchTree::leaf(&mut self.driver.tree, lock.turn);
            drop(lock);
            if i == 0 || self.analyze(state.clone(), depth - 1, 0, -alpha - 1, -alpha, true, false).checked_neg().unwrap_or(i32::MIN + 1) > alpha {
                val = self.analyze(state.clone(), depth - 1, 0, -beta, -alpha, true, true).checked_neg().unwrap_or(i32::MIN + 1);
            }
            lock = state.lock().unwrap();
            lock.unmake_last(true);
            drop(lock);
            self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
            if self.driver.time_remaining.is_zero() {
                break;
            }
            
            if val > alpha {
                best = motion;
                self.mtm = motion.motion;
                SearchTree::highlight_last(&mut self.driver.tree);
                SearchTree::back(&mut self.driver.tree, false);
                if val >= beta {
                    self.tt.insert(motion.key, motion);
                    return beta;
                }
                alpha = val;
                self.tt.insert(motion.key, motion);

            } else {
                SearchTree::back(&mut self.driver.tree, false);
            }
            



            
            i += 1;
        }
        self.tt.insert(best.key, best);
        
        return alpha;
    }

    fn iterate(&mut self, state: Arc<Mutex<State>>) -> Motion {
        
        
        let lock = state.lock().unwrap();
        
        let moves = lock.moves.parity_vect(lock.turn);
        let move_count = moves.len();
        drop(lock);
        
        self.driver.depth = 1;
        let mut val = self.sroot(state.clone(), self.driver.depth, i32::MIN + 1, i32::MAX - 1);
        for i in 2..=Self::ABSOLUTELY_MAX_DEPTH {
            self.driver.time_remaining = self.time_limit.checked_sub(time::Instant::now().duration_since(self.driver.time_start)).unwrap_or(time::Duration::ZERO);
            if self.driver.time_remaining.is_zero() { break };
            if move_count == 1 && self.driver.depth > 4 { break };
            self.driver.depth = i;
            val = self.widen(state.clone(), val);
        }
        return self.mtm;
    }
    fn widen(&mut self, state: Arc<Mutex<State>>, val: i32) -> i32 {
        self.driver.communicate(&self.mtm);
        let alpha = val.checked_sub(Self::ASPIRATION_ADJUSTMENT).unwrap_or(i32::MIN + 1);
        let beta = val.checked_add(Self::ASPIRATION_ADJUSTMENT).unwrap_or(i32::MAX - 1);
        let mut temp = self.sroot(state.clone(), self.driver.depth, alpha, beta);
        if temp <= alpha || temp >= beta {
            temp = self.sroot(state.clone(), self.driver.depth, i32::MIN + 1, i32::MAX - 1);
        }
        return temp;
    }
}
