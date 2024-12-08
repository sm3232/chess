
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

use super::{heap::EvaluatedMotion, motion::Motion, searcher::{SearchCheckIn, SearchDriver}, state::State, ui::Input};
pub struct VisualInfo {
    pub visual_weights: Option<[i32; 64]>,
    pub cache_saves: Option<usize>,
    pub analyzed: Option<usize>,
    pub evaluation: Option<Evaluator>,
    pub tree: Option<SearchTree>,
}
impl VisualInfo {
    pub fn none() -> Self { Self { analyzed: None, cache_saves: None, tree: None, evaluation: None, visual_weights: None } }
    pub fn weight_eval(weights: &Option<[i32; 64]>, evaluator: Evaluator) -> Self {
        Self {
            visual_weights: *weights,
            evaluation: Some(evaluator),
            tree: None,
            cache_saves: None,
            analyzed: None
        }
    }
    pub fn all(weights: &[i32; 64], evaluator: Evaluator, tree: SearchTree, cache: usize, analyze: usize) -> Self {
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
    pub board: Option<[u8; 64]>,
    pub allowed_castles: Option<u8>,
    pub moves: Option<[Vec<Motion>; 64]>,
    pub waiting_for_a_human_input: Option<bool>,
    pub turn: Option<Parity>,
    pub selected: Option<usize>,
    pub working: Option<bool>,
    pub game_over: Option<bool>,
    pub visuals: Option<VisualInfo>
}

pub struct Manager {
    pub game: ChessGame,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub current_eval: Evaluator,
    pub sender: mpsc::Sender<SharedState>,
    pub receiver: mpsc::Receiver<Input>,
    working_channel_recv: Option<mpsc::Receiver<SearchCheckIn>>,
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
                tt: HashMap::new(),
                driver: SearchDriver::default(),
                mtm: Motion::default()
            }
        };
    }
}
impl Player for ManagerPlayer {
    fn get_cache_saves(&self) -> usize {
        return self.searcher.driver.cache_saves;
    }
    fn get_analyzed(&self) -> usize {
        return self.searcher.driver.positions_looked_at;
    }
    fn get_parity(&self) -> Parity {
        return self.parity;
    }
    fn your_turn(&mut self, state: Arc<Mutex<State>>, comms: mpsc::Sender<SearchCheckIn>) -> bool {
        self.searcher.driver.communicate_on(comms);
        self.searcher.tree.clear();
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
            working_channel_recv: None,
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
                    board: Some(locked.board),
                    turn: Some(locked.turn),
                    waiting_for_a_human_input: Some(false),
                    allowed_castles: Some(locked.info.allowed_castles),
                    selected: Some(self.game.selected),
                    working: Some(false),
                    game_over: Some(true),
                    moves: Some(locked.moves.parity_moves(locked.turn)),
                    visuals: Some(VisualInfo::weight_eval(
                        &self.game.visual_weights, 
                        self.current_eval.clone()
                    ))
                });
                drop(locked);
                self.frame.request_repaint();
                continue;
            }
            if self.worker.as_ref().is_some_and(|x| !x.is_finished()) {
                thread::sleep(Duration::from_millis(16));
                if let Some(comms) = &self.working_channel_recv {
                    match comms.try_recv() {
                        Ok(val) => {
                            let _ = self.sender.send(SharedState {
                                board: None,
                                turn: None,
                                waiting_for_a_human_input: None,
                                moves: None,
                                allowed_castles: None,
                                working: Some(true),
                                game_over: None,
                                selected: None,
                                visuals: Some(VisualInfo::all(
                                    &self.game.visual_weights.unwrap(),
                                    self.current_eval.clone(),
                                    val.tree,
                                    val.cache_saves,
                                    val.positions_looked_at
                                ))
                            });
                            self.frame.request_repaint();
                        },
                        Err(mpsc::TryRecvError::Empty) => println!("No data from comms"),
                        Err(mpsc::TryRecvError::Disconnected) => println!("Comms disconnected!")
                    }
                }
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
                    waiting_for_a_human_input: Some(true),
                    turn: Some(self.game.human_player),
                    moves: Some(locked.moves.parity_moves(self.game.human_player)),
                    selected: Some(self.game.selected),
                    board: Some(locked.board),
                    allowed_castles: Some(locked.info.allowed_castles),
                    working: Some(false),
                    game_over: Some(false),
                    visuals: Some(VisualInfo::weight_eval(&self.game.visual_weights, self.current_eval.clone()))
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
                        let (send, recv) = mpsc::channel();
                        self.working_channel_recv = Some(recv);
                        let worker = thread::spawn(move || {
                            return player.lock().unwrap().your_turn(data, send);
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
