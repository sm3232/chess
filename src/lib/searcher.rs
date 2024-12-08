use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use crate::lib::{
    chessbyte::ChessByte, cutil::pretty_print::pretty_print_board, eval, motion::Motion, piece::Parity, searchtree::SearchTree
};
use super::{heap::{EvaluatedMotion, Heap}, mask::Mask, state::State};

pub struct SearchDriver {
    pub parity: Parity,
    pub depth: usize,
    pub nodes: usize,
    pub q_nodes: usize,
}

impl SearchDriver {
    pub fn clear(&mut self, side_to_move: Parity) {
        self.parity = side_to_move;
        self.depth = 0;
        self.nodes = 0;
        self.q_nodes = 0;
    }
}

pub struct Searcher {
    pub tree: Vec<Arc<Mutex<SearchTree>>>,
    pub ply: usize,
    pub primary: Parity,
    pub heap: Heap,
    pub echo_table: HashSet<u64>,
    pub tt: HashMap<u64, (EvaluatedMotion, usize)>,
    pub cache_saves: usize,
    pub analyzed: usize,
    pub driver: SearchDriver,
    pub mtm: Motion
}
impl Searcher {
    /*
    pub fn quiesce(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32, depth: usize) -> i32 {
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
                let moves = if ilsl.turn == Parity::WHITE { ilsl.moves.white_moves[i].clone() } else { ilsl.moves.black_moves[i].clone() };
                for m in moves.iter() {
                    if ilsl.board[m.to].is_piece() && ilsl.board[m.to].is_parity(!ilsl.board[m.from].get_parity()) {
                        ilsl.num_analyzed += 1;
                        ilsl.make_motion(m, false);
                        drop(ilsl);

                        let score = -self.quiesce(state.clone(), -beta, -alpha, depth + 1);
                        ilsl = state.lock().unwrap();
                        ilsl.unmake_last(true);
                        if score >= beta {
                            drop(ilsl);
                            return beta;
                        }
                        if score > alpha {
                            alpha = score;
                            /*
                            if ilsl.turn == self.primary {
                                self.heap.push(EvaluatedMotion { evaluation: score, motion: *m } );
                            }
                            */
                        }
                        
                    }
                }
            }
            drop(ilsl);
        }
        return alpha;
    }
*/
    const ABSOLUTELY_MAX_DEPTH: usize = 10;
    const ASPIRATION_ADJUSTMENT: i32 = 50;
    pub fn run(&mut self, state: Arc<Mutex<State>>) -> Motion {
        println!("Searcher starting");
        println!("Searcher getting lock");
        let lock = state.lock().unwrap();
        println!("Searcher got lock");
        let parity = lock.turn;
        drop(lock);
        println!("Searcher dropped lock");
        self.driver.clear(parity);
        println!("Searcher cleared driver");
        // calc_movetime();
        // age_history_table();
        println!("Searcher calling iterate");
        return self.iterate(state.clone());
        println!("Searcher returned from iterate");
    }
    fn analyze(&mut self, state: Arc<Mutex<State>>, mut depth: usize, ply: usize, mut alpha: i32, mut beta: i32, null: bool, pv: bool) -> i32 {
        println!("Analyzer start analyze");
        let mut val = i32::MIN + 1;
        let mate = i32::MAX - ply as i32;
        if alpha < -mate {
            alpha = -mate;
        }
        if beta > mate - 1 {
            beta = mate - 1;
        }
        if alpha >= beta {
            println!("Analyzer returning alpha bc alpha >= beta. {alpha} >= {beta}");
            return alpha;
        }
        println!("Analyzer getting lock");
        let mut lock = state.lock().unwrap();
        println!("Analyzer got lock");
        let in_check = (lock.moves.parity_flat(!self.driver.parity) & Mask::from_index(lock.info.king_indices[0])).any();
        if in_check { depth += 1 };
        if depth == 0 {
            drop(lock);
            println!("Analyzer dropped lock");
            println!("Analyzer calling quiet");
            return self.quiescence(state.clone(), alpha, beta);
            println!("Analyzer returned from quiet");
        }
        self.driver.nodes += 1;
        if !pv || (val > alpha && val < beta) {
            if let Some(saved) = self.tt.get(&lock.info.zkey) {
                drop(lock);
                println!("Analyzer dropping lock inside !pv || (val > alpha && val < beta)");
                println!("Analyzer returning {}", saved.0.evaluation);
                return saved.0.evaluation;
            }
        }
        if depth < 3 && !pv && !in_check && beta.abs_diff(beta - 1) as i32 > i32::MIN + 100 {
            let statice = eval::start_eval(&lock);
            if statice.eval - (120 * depth) as i32 >= beta {
                println!("Analyzer dropping lock insize depth < 3 && !pv && !in_check...");
                drop(lock);
                println!("Analyzer returning {}", statice.eval - 120 * depth as i32);
                return statice.eval - 120 * depth as i32;
            }
        }
        let moves = lock.moves.parity_vect(self.driver.parity);
        let mut heap = Heap::default();
        let mut raised = false;
        let mut ndepth = depth - 1;
        let mut reduce = 1;
        let mut best = EvaluatedMotion::default();
        println!("Analyzer sorting moves");
        for m in &moves {
            lock.make_motion(m, false);
            heap.push(EvaluatedMotion { evaluation: eval::start_eval(&lock).eval, motion: *m, key: lock.info.zkey });
            lock.unmake_last(true);
        }
        drop(lock);
        println!("Analyzer dropped lock");

        println!("Analyzer starting outer loop");
        loop {
            let motion = heap.pop();

            println!("Analyzer starting inner loop (research)");
            'research: loop {
                if !raised {

                    println!("Analyzer not raised, calling analyze");
                    {
                        println!("TESTBLOCK");
                        let l = state.lock().unwrap();
                        pretty_print_board("TEST BOARD PRINT", &l.board);
                        drop(l);
                    }
                    val = -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, pv);
                    println!("Analyzer returned from analyze in not raised");
                } else {
                    println!("Analyzer is raised, calling analyze if");
                    if -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, false) > alpha {
                        println!("Analyzer returned from analyze if in is raised");
                        println!("Analyzer calling analyze again in is raised");
                        val = -self.analyze(state.clone(), ndepth, ply + 1, -beta, -alpha, true, true);
                        println!("Analyzer returned from analyze again in is raised");
                    } else {
                        println!("Analyzer returned from analyze in is raised, not calling again");
                    }
                }
                if reduce > 0 && val > alpha {
                    println!("Analyzer reduce > 0 && val > alpha ({reduce} > 0 && {val} > {alpha})");
                    ndepth += reduce;
                    reduce = 0;
                    println!("Analyzer research-ing");
                    continue 'research;
                }
                if val > alpha {
                    println!("Analyzer val > alpha ({val} > {alpha})");
                    best = motion;
                    if val >= beta {
                        
                    }
                    raised = true;
                    alpha = val;
                }
                println!("Analyzer breaking re-search");
                break 'research;
            }
            println!("Analyzer out of inner loop");
            if heap.empty() { break };
        }
        println!("Analyzer out of outer loop");
        self.tt.insert(best.key, (best, depth));

        println!("Analyzer returning alpha {alpha}");
        return alpha;
    }
    // fn is_repetition(&self, state)
    fn quiescence(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32) -> i32 {
        println!("Quiet starting quiet");
        self.driver.nodes += 1;
        self.driver.q_nodes += 1;
        println!("Quiet getting lcok");
        let mut lock = state.lock().unwrap();
        println!("Quiet got lock");
        let mut val = eval::start_eval(&lock).eval;

        if val >= beta {
            println!("Quiet dropping lock bc val >= beta {val} >= {beta}");
            drop(lock);
            println!("Quiet returning beta {beta}");
            return beta;
        }
        if alpha < val {
            alpha = val;
        }
        let moves = lock.moves.parity_vect(self.driver.parity);
        println!("Quiet dropping lock");
        drop(lock);
        let mut i = 0;
        println!("Quiet looping moved");
        for m in &moves {
            println!("Quiet getting lock in move loop");
            lock = state.lock().unwrap();
            if !lock.board[m.to].is_piece() || lock.board[m.to].is_parity(lock.board[m.from].get_parity()) {
                println!("Quiet move is not capture, dropping lock and continue");
                drop(lock);
                continue;
            }
            lock.make_motion(m, false);
            println!("Quiet dropping lock after making motion");
            drop(lock);

            println!("Quiet calling self");
            val = -self.quiescence(state.clone(), -beta, -alpha);
            println!("Quiet returned from self");

            println!("Quiet getting lock to unmake");
            lock = state.lock().unwrap();
            lock.unmake_last(true);
            println!("Quiet dropping lock after unmake");
            drop(lock);

            if val > alpha {
                if val >= beta {
                    println!("Quiet returning bc val >= beta {val} >= {beta}");
                    return beta;
                }
                alpha = val;
            }
            i += 1;
        }
        println!("Quiet returning alpha {alpha}");
        return alpha;
    }
    fn sroot(&mut self, state: Arc<Mutex<State>>, mut depth: usize, mut alpha: i32, beta: i32) -> i32 {
        println!("Root starting root");
        let mut val = 0;
        let mut best = EvaluatedMotion::default();
        println!("Root getting lock");
        let mut lock = state.lock().unwrap();
        println!("Root got lock");
        let in_check = (lock.moves.parity_flat(!self.driver.parity) & Mask::from_index(lock.info.king_indices[0])).any();
        if in_check { depth += 1 };
        let moves = lock.moves.parity_vect(self.driver.parity);
        let mut heap = Heap::default();
        println!("Root sorting moves");
        for m in &moves {
            lock.make_motion(m, false);
            heap.push(EvaluatedMotion { evaluation: eval::start_eval(&lock).eval, motion: *m, key: lock.info.zkey });
            lock.unmake_last(true);
        }
        drop(lock);
        println!("Root dropped lock");
        let mut i = 0;
        println!("Root begin outer loop");
        loop {
            let motion = heap.pop();
            println!("Root calling analyze if {i}==0 or analyze call");
            if i == 0 || -self.analyze(state.clone(), depth - 1, 0, -alpha - 1, -alpha, true, false) > alpha {
                println!("Root calling analyze inside the first move check ({i})");
                val = -self.analyze(state.clone(), depth - 1, 0, -beta, -alpha, true, true);
                println!("Root returned from analyze inside the first move check ({i})");
            }
            println!("Root passed first move check ({i})");
            if val > alpha {
                println!("Root val > alpha, {val} > {alpha}");
                best = motion;
                self.mtm = motion.motion;
                if val >= beta {
                    println!("Root val >= beta, {val} >= {beta}");
                    self.tt.insert(motion.key, (motion, depth));
                    
                    println!("Root returning beta {beta}");
                    return beta;
                }
                alpha = val;
                self.tt.insert(motion.key, (motion, depth));

            }
            



            println!("Root end of outer loop, continuing if heap has items");
            if heap.empty() { break };
            i += 1;
        }
        self.tt.insert(best.key, (best, depth));
        println!("Root returning alpha {alpha}");
        return alpha;
    }

    fn iterate(&mut self, state: Arc<Mutex<State>>) -> Motion {
        println!("Iterator starting iterate");
        println!("Iterator getting lock");
        let lock = state.lock().unwrap();
        println!("Iterator got lock");
        let moves = lock.moves.parity_vect(self.driver.parity);
        let move_count = moves.len();
        drop(lock);
        println!("Iterator dropped lock");
        self.driver.depth = 1;
        println!("Iterator calling root");
        let mut val = self.sroot(state.clone(), self.driver.depth, i32::MIN + 1, i32::MAX - 1);
        println!("Iterator returned from root");
        println!("Iterator begin iterations");
        for i in 2..=Self::ABSOLUTELY_MAX_DEPTH {
            println!("Iterator iteration num {i}/{}", Self::ABSOLUTELY_MAX_DEPTH);
            if move_count == 1 && self.driver.depth > 4 { break };
            self.driver.depth = i;
            println!("Iterator calling widen");
            val = self.widen(state.clone(), val);
            println!("Iterator returned from widen");
        }
        println!("Iterator out of iterations, returning self.mtm ({} -> {})", self.mtm.from, self.mtm.to);
        return self.mtm;
    }
    fn widen(&mut self, state: Arc<Mutex<State>>, val: i32) -> i32 {
        println!("Widen start widen");
        let alpha = val - Self::ASPIRATION_ADJUSTMENT;
        let beta = val + Self::ASPIRATION_ADJUSTMENT;
        println!("Widen calling root");
        let mut temp = self.sroot(state.clone(), self.driver.depth, alpha, beta);
        println!("Widen returned from root");
        if temp <= alpha || temp >= beta {
            println!("Widen calling root bc temp <= alpha || temp >= beta. {temp} <= {alpha} || {temp} >= {beta}");
            temp = self.sroot(state.clone(), self.driver.depth, i32::MIN + 1, i32::MAX - 1);
            println!("Widen returned from root");
        }
        println!("Widen returning temp {temp}");
        return temp;
    }

    /*
int Quiesce(int alpha, int beta) {

  if (!time_over && !(sd.nodes & 0x3FF))
    time_over = time_stop();

  if (time_over) return 0;

  sd.nodes++;
  sd.q_nodes++;

  /* get a "stand pat" score */
  int val = eval(alpha, beta, 1);
  int stand_pat = val;

  /* check if stand-pat score causes a beta cutoff */
  if (val >= beta)
    return beta;

  /* check if stand-pat score may become a new alpha */
  if (alpha < val)
    alpha = val;

  /*********************************************************************
  *  We have taken into account rhe stand pat score, and it didn't let *
  *  us  come to a definite conclusion about the position. So we  must *
  *  do a real search.                                                 *
  *********************************************************************/

  smove movelist[256];
  U8 mcount = movegen_qs(movelist);

  for (U8 i = 0; i < mcount; i++) {

    movegen_sort(mcount, movelist, i);

    if (movelist[i].piece_cap == KING) return INF;

    /*****************************************************************
    *  Delta cutoff - a move guarentees the score well below alpha,  *
    *  so  there's no point in searching it. This heuristic is  not  *
    *  used  in the endgame, because of the  insufficient  material  *
    *  issues and special endgame evaluation heuristics.             *
    *****************************************************************/

    if ((stand_pat + e.PIECE_VALUE[movelist[i].piece_cap] + 200 < alpha) &&
      (b.PieceMaterial[!b.stm] - e.PIECE_VALUE[movelist[i].piece_cap] > e.ENDGAME_MAT) &&
      (!move_isprom(movelist[i])))
      continue;

    /*****************************************************************
    *  badCapture() replaces a cutoff based on the  Static Exchange  *
    *  Evaluation,  marking  the place where it ought to  be  coded. *
    *  Nevertheless, it saves quite a few nodes.                     *
    *****************************************************************/

    if (badCapture(movelist[i])
      && !move_canSimplify(movelist[i])
      && !move_isprom(movelist[i])
      )
      continue;

    /*****************************************************************
    *  Cutoffs  misfired, so the move in question can turn out well. *
    *  Let us try it, then.                                          *
    *****************************************************************/

    move_make(movelist[i]);

    val = -Quiesce(-beta, -alpha);

    move_unmake(movelist[i]);

    if (time_over) return 0;

    if (val > alpha) {
      if (val >= beta)
        return beta;

      alpha = val;
    }
  }
  return alpha;
}
     
     
     
     */


        /*
/* symbols used to enhance readability */
#define DO_NULL    1
#define NO_NULL    0
#define IS_PV      1
#define NO_PV      0

sSearchDriver sd;

int draw_opening = -10; // middlegame draw value
int draw_endgame = 0;   // endgame draw value
int ASPIRATION = 50;  // size of the aspiration window ( val-ASPITATION, val+ASPIRATION )

bool time_over = 0;

enum eproto {
  PROTO_NOTHING,
  PROTO_XBOARD,
  PROTO_UCI
} extern mode;

U8 bestmove;         // move id passed between iterations for sorting purposes
smove move_to_make;	 // move to be returned when search runs out of time

/***************************************************************
*  search_run() is the interface of all the search functions,  *
*  the only function called outside search.cpp. It does some   *
*  preparatory work, and then calls search_iterate();          *
***************************************************************/

void search_run() {

  if (chronos.flags & (FTIME | FINC | FMOVESTOGO)) {
    if (getBookMove(BOOK_BROAD)) return;
  }

  search_clearDriver();
  time_calc_movetime();
  ageHistoryTable();
  if (mode == PROTO_NOTHING) printSearchHeader();

  search_iterate();
}

void search_clearDriver() {
  sd.myside = b.stm;         // remember color - needed in contempt()
  sd.starttime = gettime();
  sd.movetime = 0;
  sd.depth = 0;

  // now clear all the statistical data
  sd.nodes = 0;
  sd.q_nodes = 0;
}

/**************************************************************
*  search_iterate() calls search_root() with increasing depth *
*  until allocated time is exhausted.                         *
**************************************************************/

void search_iterate() {
  int val, temp;

  // check the exact number of legal moves in the current position

  int move_count = move_countLegal();

  // do a full-window 1-ply search to get the first estimate of val 

  sd.depth = 1;
  val = search_root(sd.depth, -INF, INF);

  // main loop, increasing deph in steps of 1

  for (sd.depth = 2; sd.depth <= MAX_DEPTH; sd.depth++) {

    // breaking conditions - either expired time
    // or just one legal reply and position searched to depth 4

    if (time_stop_root() || time_over) break;
    if (move_count == 1 && sd.depth == 5) break;

    // this function deals with aspiration window
    val = search_widen(sd.depth, val);
  }

  // after the loop has finished, send the move to the interface
  com_sendmove(move_to_make);
}

int search_widen(int depth, int val) {
  int temp = val,
    alpha = val - 50,
    beta = val + 50;

  temp = search_root(sd.depth, alpha, beta);
  if (temp <= alpha || temp >= beta)
    temp = search_root(sd.depth, -INF, INF);
  return temp;
}

int search_root(U8 depth, int alpha, int beta) {

  int flagInCheck;
  smove movelist[256];
  int val = 0;

  U8 currmove_legal = 0;

  /* Check  extension is done also at  the  root*/

  flagInCheck = isAttacked(!b.stm, b.KingLoc[b.stm]);
  if (flagInCheck) ++depth;

  U8 mcount = movegen(movelist, bestmove);

  for (U8 i = 0; i < mcount; i++) {

    movegen_sort(mcount, movelist, i);

    if (movelist[i].piece_cap == KING) {
      alpha = INF;
      bestmove = movelist[i].id;
    }

    move_make(movelist[i]);

    // filter out illegal moves
    if (isAttacked(b.stm, b.KingLoc[!b.stm])) {
      move_unmake(movelist[i]);
      continue;
    }

    //	if ( mode == PROTO_UCI ) 
    //		info_currmove( movelist[i], currmove_legal ); 

    currmove_legal++;

    /* the "if" clause introduces PVS at root */

    if ((i == 0) ||
      (-Search(depth - 1, 0, -alpha - 1, -alpha, DO_NULL, NO_PV) > alpha))
      val = -Search(depth - 1, 0, -beta, -alpha, DO_NULL, IS_PV);

    move_unmake(movelist[i]);

    if (time_over) break;

    // see CCC Discussion, Re: Debugging a transposition table by Vivien Clauzon

    if (val > alpha) { 

      bestmove = movelist[i].id;
      move_to_make = movelist[i];

      if (val > beta) { // should be >=, see post
        tt_save(depth, beta, TT_BETA, bestmove);
        info_pv(beta);
        return beta;
      }

      alpha = val;
      tt_save(depth, alpha, TT_ALPHA, bestmove);

      info_pv(val);
    } // changing node value finished
  }

  tt_save(depth, alpha, TT_EXACT, bestmove);
  return alpha;
}

int Search(U8 depth, U8 ply, int alpha, int beta, int can_null, int is_pv) {

  int  val = -INF;
  char bestmove;
  char tt_move = INVALID;
  char tt_flag = TT_ALPHA;
  int  flagInCheck;
  int  legal_move = 0;
  int  raised_alpha = 0;
  int  f_prune = 0;
  int  reduction_depth = 0;
  int  moves_tried = 0;
  int  new_depth;
  int  mate_value = INF - ply; // will be used in mate distance pruning
  smove move;

  /************************************************************************
  *  Probably later we will want to probe the transposition table.        *
  *  Tell the cpu to prepare for that event. This is just a minor         *
  *  speed optimization and program would run fine without that.          *
  ************************************************************************/

  _mm_prefetch((char *)&tt[b.hash & tt_size], _MM_HINT_NTA);

  /************************************************************************
  * Check for timeout. This is quite time-consuming, so we do it only     *
  * every so often. The side effect is that if we want  to limit search   *
  * by number of nodes, it will be slightly inexact.                      *
  ************************************************************************/

  if (!time_over && !(sd.nodes & 4095))
    time_over = time_stop();
  if (time_over) return 0;

  /************************************************************************
  * MATE DISTANCE PRUNING - a minor improvement that helps to shave off   *
  * some nodes when the checkmate is near. Basically it prevents looking  *
  * for checkmates taking longer than one we have already found. No Elo   *
  * gain expected, but it's a nice feature. Don't use it at the root,     *
  * since  this code  doesn't return a move, only a value.                *
  ************************************************************************/

  if (alpha < -mate_value) alpha = -mate_value;
  if (beta > mate_value - 1) beta = mate_value - 1;
  if (alpha >= beta) return alpha;

  /************************************************************************
  *  Are we in check? If so, extend. It also means that program will      *
  *  never enter quiescence search while in check.                        *
  ************************************************************************/

  flagInCheck = (isAttacked(!b.stm, b.KingLoc[b.stm]));
  if (flagInCheck) ++depth;

  /************************************************************************
  *  At leaf nodes we do quiescence search (captures only) to make sure   *
  *  that only relatively quiet positions with no hanging pieces will be  *
  *  evaluated.                                                           *
  ************************************************************************/

  if (depth == 0) return Quiesce(alpha, beta);

  sd.nodes++;

  if (isRepetition()) return contempt();

  /************************************************************************
  *  Read the transposition table. We may have already searched current   *
  *  position. If depth was sufficient, then we might use the score       *
  *  of that search. If not, hash move still is expected to be good       *
  *  and should be sorted first.                                          *
  *                                                                       *
  *  NOTE: current implementation is sub-standard, since tt_move is just  *
  *  an index showing move's location on a move list. We should be able   *
  *  to retrieve move without generating full move list instead.          *
  ************************************************************************/

  if ((val = tt_probe(depth, alpha, beta, &tt_move)) != INVALID) {
    // in pv nodes we return only in case of an exact hash hit
    if (!is_pv || (val > alpha && val < beta))
      return val;
  }

  /************************************************************************
  * EVAL PRUNING / STATIC NULL MOVE                                       *
  ************************************************************************/

  if (depth < 3
    && (!is_pv)
    && (!flagInCheck)
    && (abs(beta - 1) > -INF + 100))
  {
    int static_eval = eval(alpha, beta, 1);

    int eval_margin = 120 * depth;
    if (static_eval - eval_margin >= beta)
      return static_eval - eval_margin;
  }

  /************************************************************************
  *  Here  we introduce  NULL MOVE PRUNING. It  means  allowing opponent  *
  *  to execute two moves in a row, i.e. capturing something and escaping *
  *  a recapture. If this cannot  wreck our position, then it is so good  *
  *  that there's  no  point in searching further. The flag "can_null"    *
  *  ensures we don't do  two null moves in a row. Null move is not used  *
  *  in  the endgame because of the risk of zugzwang.                     *
  ************************************************************************/

  if ((depth > 2)
    && (can_null)
    && (!is_pv)
    && (eval(alpha, beta, 1) > beta) //should be >=, see post
    && (b.PieceMaterial[b.stm] > e.ENDGAME_MAT)
    && (!flagInCheck))
  {
    char ep_old = b.ep;
    move_makeNull();

    /********************************************************************
    *  We use so-called adaptative null move pruning. Size of reduction *
    *  depends on remaining  depth.                                     *
    ********************************************************************/

    char R = 2;
    if (depth > 6) R = 3;

    val = -Search(depth - R - 1, ply, -beta, -beta + 1, NO_NULL, NO_PV);

    move_unmakeNull(ep_old);

    if (time_over) return 0;
    if (val >= beta) return beta;
  }

  /************************************************************************
  *  Decide  if FUTILITY PRUNING  is  applicable. If we are not in check, *
  *  not searching for a checkmate and eval is below  (alpha - margin),   *
  *  it  might  mean that searching non-tactical moves at  low depths     *
  *  is futile, so we set a flag allowing this pruning.                   *
  ************************************************************************/

  int fmargin[4] = { 0, 200, 300, 500 };

  if (depth <= 3
    && !is_pv
    && !flagInCheck
    &&	 abs(alpha) < 9000
    && eval(alpha, beta, 1) + fmargin[depth] <= alpha)
    f_prune = 1;

  /* generate moves */

  smove movelist[256];
  U8 mcount = movegen(movelist, tt_move);

  ReorderMoves(movelist, mcount, ply);

  bestmove = movelist[0].id;

  /************************************************************************
  *  Now it's time to loop through the move list.                         *
  ************************************************************************/

  for (int i = 0; i < mcount; i++) {

    movegen_sort(mcount, movelist, i); // pick the best of untried moves
    move = movelist[i];
    move_make(move);

    // filter out illegal moves
    if (isAttacked(b.stm, b.KingLoc[!b.stm])) {
      move_unmake(move);
      continue;
    }
    moves_tried++;

    /********************************************************************
    *  When the futility pruning flag is set, prune moves which do not  *
    *  give  check and do not change material balance.  Some  programs  *
    *  prune insufficient captures as well, but that seems too risky.   *
    ********************************************************************/

    if (f_prune
      &&	 legal_move
      && !move_iscapt(move)
      && !move_isprom(move)
      && !isAttacked(!b.stm, b.KingLoc[b.stm])) {
      move_unmake(move);
      continue;
    }

    reduction_depth = 0;   // this move has not been reduced yet
    new_depth = depth - 1; // decrease depth by one ply

    /********************************************************************
    *  Late move reduction. Typically a cutoff occurs on trying one of  *
    *  the first moves. If it doesn't, we are probably in an all-node,  *
    *  which means that all moves will fail low. So we might as well    *
    *  spare some effort, searching to reduced depth. Of course this is *
    *  not a foolproof method, but it works more often than not. Still, *
    *  we  need to exclude certain moves from reduction, in  order  to  *
    *  filter out tactical moves that may cause a late cutoff.          *
    ********************************************************************/

    if (!is_pv
      && new_depth > 3
      && legal_move
      && moves_tried > 3
      && !isAttacked(!b.stm, b.KingLoc[b.stm])
      && !flagInCheck
      && (move.from != sd.killers[0][ply].from || move.to != sd.killers[0][ply].to)
      && (move.from != sd.killers[1][ply].from || move.to != sd.killers[1][ply].to)
      && !move_iscapt(move)
      && !move_isprom(move)) {

      /****************************************************************
      * Real programs tend use more advanced formulas to calculate    *
      * reduction depth. Typically they calculate it from both        *
      * remaining depth and move count. Formula used here is very     *
      * basic and gives only a minimal improvement over uniform       *
      * one ply reduction, and is included for the sake of complete-  *
      * ness only.                                                    *
      ****************************************************************/

      reduction_depth = 1;
      if (moves_tried > 8) reduction_depth += 1;

      new_depth -= reduction_depth;
    }

  re_search:

    /********************************************************************
    *  The code below introduces principal variation search. It  means  *
    *  that once we are in a PV-node (indicated by IS_PV flag) and  we  *
    *  have  found a move that raises alpha, we assume that  the  rest  *
    *  of moves ought to be refuted. This is done  relatively  cheaply  *
    *  by using  a null-window search centered around alpha.  Only  if  *
    *  this search fails high, we are forced repeat it with full window.*
    *                                                                   *
    *  Understanding the shorthand in the first two lines is a bit      *
    *  tricky. If alpha has not been raised, we might be either in      *
    *  a  zero window (scout) node or in an open window (pv)  node,     *
    *  entered after a scout search failed high. In both cases, we      *
    *  need to search with the same alpha, the same beta AND the same   *
    *  node type.                                                       *
    ********************************************************************/

    if (!raised_alpha)
      val = -Search(new_depth, ply + 1, -beta, -alpha, DO_NULL, is_pv);
    else {
      // first try to refute a move - if this fails, do a real search
      if (-Search(new_depth, ply + 1, -alpha - 1, -alpha, DO_NULL, NO_PV) > alpha)
        val = -Search(new_depth, ply + 1, -beta, -alpha, DO_NULL, IS_PV);
    }

    /********************************************************************
    *  Sometimes reduced search brings us above alpha. This is unusual, *
    *  since we expected reduced move to be bad in first place. It is   *
    *  not certain now, so let's search to the full, unreduced depth.   *
    ********************************************************************/

    if (reduction_depth && val > alpha) {
      new_depth += reduction_depth;
      reduction_depth = 0;
      goto re_search;
    }

    move_unmake(move);

    /********************************************************************
    *  If  the  move doesn't return -INF, it means that  the  King      *
    *  couldn't be captured immediately. So the move was legal. In this *
    *  case we increase the legal_move counter, to look afterwards,     *
    *  whether there were any legal moves on the board at all.          *
    ********************************************************************/

    legal_move += (val != -INF);

    if (time_over) return 0;

    /********************************************************************
    *  We can improve over alpha, so we change the node value together  *
    *  with  the expected move. Also the raised_alpha flag, needed  to  *
    *  control PVS, is set. In case of a beta cuoff, when our position  *
    *  is  so good that the score will not be accepted one ply before,  *
    *  we return it immediately.                                        *
    ********************************************************************/

    if (val > alpha) {

      bestmove = movelist[i].id;

      if (val >= beta) {

        /*************************************************************
        *  On a quiet move update killer moves and history table     *
        *  in order to enhance move ordering.                        *
        *************************************************************/

        if (!move_iscapt(move)
          && !move_isprom(move)) {
          setKillers(movelist[i], ply);
          sd.history[move.from][move.to] += depth*depth;

          /*********************************************************
          *  With super deep search history table would overflow   *
          *  - let's prevent it.                                   *
          *********************************************************/

          if (sd.history[move.from][move.to] > SORT_KILL) {
            for (int a = 0; a < 128; a++)
              for (int b = 0; b < 128; b++) {
                sd.history[a][b] = sd.history[a][b] / 2;
              }
          }
        }
        tt_flag = TT_BETA;
        alpha = beta;
        break; // no need to search any further
      }

      raised_alpha = 1;
      tt_flag = TT_EXACT;
      alpha = val;

    } // changing the node value is finished

  }   // end of looping through the moves

      /************************************************************************
      *  Checkmate and stalemate detection: if we can't find a legal move     *
      *  in the current position, we test if we are in check. If so, mate     *
      *  score relative to search depth is returned. If not, we use  draw     *
      *  evaluation provided by contempt() function.                          *
      ************************************************************************/

  if (!legal_move) {
    bestmove = -1;

    if (flagInCheck) alpha = -INF + ply;
    else               alpha = contempt();
  }

  /* tt_save() does not save anything when the search is timed out */
  tt_save(depth, alpha, tt_flag, bestmove);

  return alpha;
}

void setKillers(smove m, U8 ply) {

  /* if a move isn't a capture, save it as a killer move */
  if (m.piece_cap == PIECE_EMPTY) {

    /* make sure killer moves will be different
    before saving secondary killer move */
    if (m.from != sd.killers[ply][0].from ||
      m.to != sd.killers[ply][0].to
      )
      sd.killers[ply][1] = sd.killers[ply][0];

    /* save primary killer move */
    sd.killers[ply][0] = m;
  }
}

void ReorderMoves(smove * m, U8 mcount, U8 ply) {

  for (int j = 0; j<mcount; j++) {
    if ((m[j].from == sd.killers[ply][1].from)
      && (m[j].to == sd.killers[ply][1].to)
      && (m[j].score < SORT_KILL - 1)) {
      m[j].score = SORT_KILL - 1;
    }

    if ((m[j].from == sd.killers[ply][0].from)
      && (m[j].to == sd.killers[ply][0].to)
      && (m[j].score < SORT_KILL)) {
      m[j].score = SORT_KILL;
    }
  }
}

int info_currmove(smove m, int nr) {

  switch (mode) {
  case PROTO_UCI:

    char buffer[64];
    char move[6];

    algebraic_writemove(m, move);
    sprintf(buffer, "info depth %d currmove %s currmovenumber %d", sd.depth, move, nr + 1);

    com_send(buffer);
  }
  return 0;
}

int info_pv(int val) {
  char buffer[2048];
  char score[10];
  char pv[2048];

  if (abs(val) < INF - 2000) {
    sprintf(score, "cp %d", val);
  }
  else {
    //the mating value is returned in moves not plies ( thats why /2+1)
    if (val > 0)
      sprintf(score, "mate %d", (INF - val) / 2 + 1);
    else
      sprintf(score, "mate %d", -(INF + val) / 2 - 1);
  }

  U32 nodes = (U32)sd.nodes;
  U32 time = gettime() - sd.starttime;

  util_pv(pv);

  if (mode == PROTO_NOTHING)
    sprintf(buffer, " %2d. %9d  %5d %5d %s", sd.depth, nodes, time / 10, val, pv);
  else
    sprintf(buffer, "info depth %d score %s time %u nodes %u nps %u pv %s", sd.depth, score, time, nodes, countNps(nodes, time), pv);

  com_send(buffer);

  return 0;
}

/***********************************************************
*  countNps() guards against overflow and thus cares  for  *
*  displaying  correct  nps during longer searches.  Node  *
*  count is converted from U64 to unsigned int because of  *
*  some problems with output.                              *
***********************************************************/

unsigned int countNps(unsigned int nodes, unsigned int time) {
  if (time == 0) return 0;

  if (time > 20000)
    return nodes / (time / 1000);
  else
    return (nodes * 1000) / time;
}

/***********************************************************
*  Checking if the current position has been already       *
*  encountered on the current search path. Function        *
*  does NOT check the actual number of repetitions.        *
***********************************************************/

int isRepetition() {

  for (int i = 0; i < b.rep_index; i++) {
    if (b.rep_stack[i] == b.hash)
      return 1;
  }

  return 0;
}

/************************************************************
*  Clearing the history table is needed at the beginning    *
*  of a search starting from a new position, like at the    *
*  beginning of a new game.                                 *
************************************************************/

void clearHistoryTable() {
  for (int i = 0; i < 128; i++)
    for (int j = 0; j < 128; j++) {
      sd.history[i][j] = 0;
    }
}

/************************************************************
* ageHistoryTable() is run between searches  to  decrease   *
* the  history values used for move sorting. This  causes   *
* obsolete information to disappear gradually. Clearing     *
* the table was worse for the move ordering.                *
************************************************************/

void ageHistoryTable() {
  for (int i = 0; i < 128; i++)
    for (int j = 0; j < 128; j++) {
      sd.history[i][j] = sd.history[i][j] / 8;
    }
}

/************************************************************
*  contempt() returns a draw value (which may be non-zero)  *
*  relative  to  the side to move and to the  game  stage.  *
*  This  way  we may make our program play for a  draw  or  *
*  strive to avoid it.                                      *
************************************************************/

int contempt() {
  int value = draw_opening;

  if (b.PieceMaterial[sd.myside] < e.ENDGAME_MAT)
    value = draw_endgame;

  if (b.stm == sd.myside) return value;
  else                      return -value;
}

        return 0;
    }
/*
    pub fn search(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        let mut state_lock = state.lock().unwrap();
        if depth != self.ply {
            if let Some(ttv) = self.tt.get(&state_lock.info.zkey) {
                state_lock.num_cached += 1;
                return ttv.0.evaluation;
            }
        }
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
            drop(state_lock);
            return self.quiesce(state.clone(), alpha, beta, self.ply);
        }
        drop(state_lock);
        let mut bestv = i32::MIN;
        for i in 0..64 {
            let mut isl = state.lock().unwrap();
            if isl.board[i].is_parity(isl.turn) {
                let moves = if isl.turn == Parity::WHITE { isl.moves.white_moves[i].clone() } else { isl.moves.black_moves[i].clone() };
                for (index, m) in moves.iter().enumerate() {
                    isl.num_analyzed += 1;
                    isl.make_motion(m, false);
                    drop(isl);
                    let score = if index == 0 {
                        self.search(state.clone(), -beta, -alpha, depth - 1)
                    } else {
                        let nws = -self.search(state.clone(), -alpha - 1, -alpha, depth - 1);
                        if nws > alpha && beta.checked_sub(alpha).is_some_and(|x| x > 1) {
                            self.search(state.clone(), -beta, -alpha, depth - 1)
                        } else {
                            nws
                        }
                    };
                    isl = state.lock().unwrap();
                    self.tt.insert(isl.info.zkey, (EvaluatedMotion{motion: *m, evaluation: -score}, depth));
                    isl.unmake_last(true);
                    let current_score = -score;
                    if current_score >= beta {
                        drop(isl);
                        return beta;
                    }
                    if current_score > alpha {
                        alpha = current_score;
                    }
                    if current_score > bestv {
                        bestv = current_score;
                    }
                }
            }
            drop(isl);
        }
        return alpha;
    }
*/
    /*
    pub fn pv(&mut self, state: Arc<Mutex<State>>, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        let mut state_lock = state.lock().unwrap();
        if let Some(ttv) = self.tt.get(&state_lock.info.zkey) {
            state_lock.num_cached += 1;
            return ttv.0.evaluation;
        }
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
            drop(state_lock);
            return self.quiesce(state.clone(), alpha, beta);
        }
        drop(state_lock);
        let mut best_score = i32::MIN;
        for i in 0..64 {
            let mut ilsl = state.lock().unwrap();
            if ilsl.board[i].is_parity(ilsl.turn) {
                let moves = if ilsl.turn == Parity::WHITE { ilsl.moves.white_moves[i].clone() } else { ilsl.moves.black_moves[i].clone() };
                for (index, m) in moves.iter().enumerate() {
                    ilsl.num_analyzed += 1;
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
                        self.tt.insert(ilsl.info.zkey, (EvaluatedMotion{motion: *m, evaluation: best_score}, depth));
                        if ilsl.turn == self.primary {
                            self.heap.push(EvaluatedMotion { evaluation: best_score, motion: *m } );
                        }
                    }
                }
            }
            drop(ilsl);
        }
        return alpha;
    }
    */
*/
}
