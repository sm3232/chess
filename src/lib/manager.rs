
use eframe::egui;
use std::{cell::RefCell, collections::{HashMap, HashSet}, ops::Deref, rc::Rc, sync::{mpsc, Arc, Mutex}, thread::{JoinHandle, Thread}, time::Duration};
use std::thread;

use crate::lib::{
    eval::{self, Evaluator}, 
    game::ChessGame, 
    heap::Heap, 
    piece::Parity, 
    player::Player, 
    searcher::Searcher, 
    searchtree::SearchTree
};

use super::{heap::EvaluatedMotion, motion::Motion, searcher::SearchDriver, state::State, ui::Input};
pub struct VisualInfo {
    pub visual_weights: Option<[i32; 64]>,
    pub cache_saves: Option<usize>,
    pub analyzed: Option<usize>,
    pub evaluation: Option<Evaluator>,
    pub tree: Option<Arc<Mutex<SearchTree>>>,
}
impl VisualInfo {
    pub fn none() -> Self { Self { analyzed: None, cache_saves: None, tree: None, evaluation: None, visual_weights: None } }
    pub fn weight_eval_tree(weights: &Option<[i32; 64]>, evaluator: Evaluator, tree: Arc<Mutex<SearchTree>>) -> Self {
        Self {
            visual_weights: *weights,
            evaluation: Some(evaluator),
            tree: Some(tree),
            cache_saves: None,
            analyzed: None
        }
    }
    pub fn all(weights: &[i32; 64], evaluator: Evaluator, tree: Arc<Mutex<SearchTree>>, cache: usize, analyze: usize) -> Self {
        Self {
            visual_weights: Some(*weights),
            evaluation: Some(evaluator),
            tree: Some(tree),
            analyzed: Some(analyze),
            cache_saves: Some(cache)
        }
    }
}
pub struct SharedState {
    pub board: [u8; 64],
    pub allowed_castles: u8,
    pub moves: [Vec<Motion>; 64],
    pub waiting_for_a_human_input: bool,
    pub turn: Parity,
    pub selected: usize,
    pub working: bool,
    pub game_over: bool,
    pub visuals: VisualInfo
}

pub struct Manager {
    pub game: ChessGame,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub current_eval: Evaluator,
    pub sender: mpsc::Sender<SharedState>,
    pub receiver: mpsc::Receiver<Input>,
    frame: egui::Context,
    worker: Option<JoinHandle<bool>>,
}

pub struct ManagerPlayer { parity: Parity, searcher: Searcher }

impl ManagerPlayer {
    fn new(parity: Parity) -> Self {
        return Self {
            parity,
            searcher: Searcher {
                tree: Vec::new(),
                ply: 2,
                primary: parity,
                heap: Heap::default(),
                echo_table: HashSet::new(),
                tt: HashMap::new(),
                cache_saves: 0,
                analyzed: 0,
                driver: SearchDriver {
                    q_nodes: 0,
                    nodes: 0,
                    depth: 0,
                    parity
                },
                mtm: Motion::default()
            }
        };
    }
}
impl Player for ManagerPlayer {
    fn get_cache_saves(&self) -> usize {
        return self.searcher.cache_saves;
    }
    fn get_analyzed(&self) -> usize {
        return self.searcher.analyzed;
    }
    fn get_parity(&self) -> Parity {
        return self.parity;
    }
    fn your_turn(&mut self, state: Arc<Mutex<State>>) -> bool {
        self.searcher.tree.clear();
        self.searcher.heap.clear();
        println!("Manager getting lock");
        let mut lock = state.lock().unwrap();
        println!("Manager got lock");
        lock.num_analyzed = 0;
        lock.num_cached = 0;
        drop(lock);
        println!("Manager dropped lock");
        println!("Manager starting search");
        let m = self.searcher.run(state.clone());
        println!("Manager ended search");

        println!("Manager getting lock AFTER SEARCH");
        let mut locked = state.lock().unwrap();
        println!("Manager got lock AFTER SEARCH");
        println!("Manager making motion");
        locked.make_motion(&m, true);
        println!("Manager made motion");
        drop(locked);
        println!("Manager dropped lock");
        return true;
        /*
        if self.searcher.heap.empty() {
            println!("No move! Conceding");
            return false;
        } else {
            let mut locked = state.lock().unwrap();
            println!("Max heap: {}, Motion: {} -> {}", self.searcher.heap.peek().evaluation, self.searcher.heap.peek().motion.from, self.searcher.heap.peek().motion.to);
            locked.make_motion(&self.searcher.heap.pop().motion, true);
            drop(locked);
            return true;
        }
        */
    }
}

impl Manager {
    pub fn init(frame: egui::Context, sender: mpsc::Sender<SharedState>, receiver: mpsc::Receiver<Input>, init_fen: String, playing_area: f32, info_width: f32) {
        let mut mgr = Manager {
            frame,
            game: ChessGame::init(init_fen.to_string()),
            game_rect: egui::Rect {
                min: egui::Pos2 { x: 0.0, y: 0.0 },
                max: egui::Pos2 { x: playing_area, y: playing_area }
            },
            info_rect: egui::Rect {
                min: egui::Pos2 { x: playing_area, y: 0.0 },
                max: egui::Pos2 { x: playing_area + info_width, y: playing_area }
            },
            current_eval: Evaluator { eval: 0, scores: Vec::new() },
            sender,
            receiver,
            worker: None,
        };
        mgr.game.register_players(None, Some(Arc::new(Mutex::new(ManagerPlayer::new(Parity::BLACK)))));
        let tmplock = mgr.game.state.lock().unwrap();
        mgr.current_eval = eval::start_eval(&tmplock);
        drop(tmplock);
        mgr.begin();
    }
    pub fn begin(&mut self) -> () {
        let loc = self.game.state.lock().unwrap();
        let mut last_turn = loc.turn;
        let mut stale_eval = false;
        drop(loc);
        loop {
            let tmplock = self.game.state.lock().unwrap();
            if tmplock.turn != last_turn && self.worker.is_none() {
                stale_eval = true;
                last_turn = tmplock.turn;
            }
            if stale_eval {
                self.current_eval = eval::start_eval(&tmplock);
                stale_eval = false;
            }
            drop(tmplock);
            if self.game.game_over {
                thread::sleep(Duration::from_millis(16));
                let locked = self.game.state.lock().unwrap();
                let _ = self.sender.send(SharedState {
                    board: locked.board,
                    turn: locked.turn,
                    waiting_for_a_human_input: false,
                    allowed_castles: locked.info.allowed_castles,
                    selected: self.game.selected,
                    working: false,
                    game_over: true,
                    moves: if locked.turn == Parity::WHITE { locked.moves.white_moves.clone() } else { locked.moves.black_moves.clone() },
                    visuals: VisualInfo::weight_eval_tree(
                        &self.game.visual_weights, 
                        self.current_eval.clone(), 
                        Arc::clone(locked.tree_root.as_ref().unwrap())
                    )
                });
                drop(locked);
                self.frame.request_repaint();
                continue;
            }
            if self.worker.as_ref().is_some_and(|x| !x.is_finished()) {
                thread::sleep(Duration::from_millis(16));
                println!("Manager getting lock for painting purposes");
                let locked = self.game.state.lock().unwrap();
                println!("Manager got lock for painting");

                let (p1, p2) = &self.game.players;
                
                let p = match (p1.is_some(), p2.is_some()) {
                    (false, false) => None,
                    (true, false) => if locked.turn == Parity::WHITE { Some(p1.clone().unwrap()) } else { None },
                    (false, true) => if locked.turn == Parity::BLACK { Some(p2.clone().unwrap()) } else { None },
                    (true, true) => if locked.turn == Parity::WHITE { Some(p1.clone().unwrap()) } else { Some(p2.clone().unwrap()) }
                };
                if p.is_some() {
                    println!("Manager sending visual info");
                    let _ = self.sender.send(SharedState {
                        board: locked.board,
                        turn: locked.turn,
                        waiting_for_a_human_input: false,
                        moves: if locked.turn == Parity::WHITE { locked.moves.white_moves.clone() } else { locked.moves.black_moves.clone() },
                        allowed_castles: locked.info.allowed_castles,
                        working: true,
                        game_over: false,
                        selected: self.game.selected,
                        visuals: VisualInfo::all(
                            &self.game.visual_weights.unwrap(),
                            self.current_eval.clone(),
                            Arc::clone(locked.tree_root.as_ref().unwrap()),
                            locked.num_cached,
                            locked.num_analyzed
                        )
                    });
                    println!("Manager send visual info");
                }
                self.frame.request_repaint();
                drop(locked);
                println!("Manager dropped lock");
                continue;
            } else {
                if let Some(w) = self.worker.take() {
                    if !w.join().unwrap_or(false) {
                        self.game.game_over = true;
                    }
                }
                self.worker = None;
            }
            let locked = self.game.state.lock().unwrap();
            if locked.turn == self.game.human_player || self.game.human_player == Parity::BOTH {
                let _ = self.sender.send(SharedState {
                    waiting_for_a_human_input: true,
                    turn: self.game.human_player,
                    moves: if self.game.human_player == Parity::WHITE { locked.moves.white_moves.clone() } else { locked.moves.black_moves.clone() },
                    selected: self.game.selected,
                    board: locked.board,
                    allowed_castles: locked.info.allowed_castles,
                    working: false,
                    game_over: false,
                    visuals: VisualInfo::weight_eval_tree(&self.game.visual_weights, self.current_eval.clone(), SearchTree::root(locked.turn))
                }).unwrap();
                match self.receiver.recv() {
                    Ok(x) => {
                        if x.left {
                            drop(locked);
                            self.game.human_input(x.pos.unwrap_or_default(), self.game.human_player);
                        }
                    },
                    n @ Err(mpsc::RecvError) => {
                        panic!("{:?}", n);
                    }
                };
            } else {
                if self.worker.is_none() {
                    let option_player = if locked.turn == Parity::WHITE { &self.game.players.0 } else { &self.game.players.1 };
                    if let Some(p) = option_player {
                        let data = Arc::clone(&self.game.state);
                        
                        let player = Arc::clone(p);
                        let worker = thread::spawn(move || {
                            return player.lock().unwrap().your_turn(data);
                        });
                        drop(locked);
                        self.worker = Some(worker);
                    }
                }
            }
            thread::sleep(Duration::from_millis(16));
            self.frame.request_repaint();
        }
    }
}
