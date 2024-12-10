
use eframe::egui;
use voxell_rng::{prelude::RngCore, slice_methods::SelectRandom};
use std::{collections::{HashMap, HashSet}, fs::{File, OpenOptions}, io::{Read, Write}, sync::{Arc, Mutex}, thread::JoinHandle, time::{self, Duration}};
use std::thread;
use crate::lib::{
    eval::{self, Evaluator}, 
    game::ChessGame, 
    piece::Parity, 
    player::Player, 
    searcher::Searcher, 
    searchtree::SearchTree
};

use super::{cutil::{draw::remap_cha, pretty_print::{pretty_print_mask, pretty_print_masks}}, heap::EvaluatedMotion, mask::Mask, motion::Motion, searcher::{SearchCheckIn, SearchDriver}, state::State, ui::Input};
pub struct VisualInfo {
    pub visual_weights: Option<[i32; 64]>,
    pub cache_saves: Option<usize>,
    pub analyzed: Option<usize>,
    pub evaluation: Option<Evaluator>,
    pub tree: Option<SearchTree>,
    pub mtm: Option<Motion>,
    pub considerations: Option<Vec<EvaluatedMotion>>
}
impl VisualInfo {
    pub fn none() -> Self { Self { analyzed: None, cache_saves: None, tree: None, evaluation: None, visual_weights: None, mtm: None, considerations: None } }
    pub fn weight_eval(weights: &Option<[i32; 64]>, evaluator: Evaluator) -> Self {
        Self {
            visual_weights: *weights,
            evaluation: Some(evaluator),
            tree: None,
            cache_saves: None,
            analyzed: None,
            mtm: None,
            considerations: None
        }
    }
    pub fn all(weights: &[i32; 64], evaluator: Evaluator, tree: SearchTree, cache: usize, analyze: usize, mtm: Motion, considerations: &Vec<EvaluatedMotion>) -> Self {
        Self {
            visual_weights: Some(*weights),
            evaluation: Some(evaluator),
            tree: Some(tree),
            analyzed: Some(analyze),
            cache_saves: Some(cache),
            mtm: Some(mtm),
            considerations: Some(considerations.to_vec())
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
    pub visuals: VisualInfo
}

pub struct Manager {
    pub game: ChessGame,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub current_eval: Evaluator,
    pub sender: crossbeam_channel::Sender<SharedState>,
    pub receiver: crossbeam_channel::Receiver<Input>,
    working_channel_recv: Option<crossbeam_channel::Receiver<SearchCheckIn>>,
    frame: egui::Context,
    worker: Option<JoinHandle<bool>>,
    last_worker_notice: Option<SearchCheckIn>,
    stable_board: [u8; 64],
    benchmode: bool,
    asm: bool
}

pub struct ManagerPlayer { parity: Parity, searcher: Searcher }

impl ManagerPlayer {
    fn new(parity: Parity) -> Self {
        return Self {
            parity,
            searcher: Searcher {
                tree: Vec::new(),
                time_limit: time::Duration::from_secs_f32(3.0),
                tt: HashMap::new(),
                driver: SearchDriver::default(),
                mtm: Motion::default(),
                echo: HashSet::default()
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
    fn your_turn(&mut self, state: Arc<Mutex<State>>, comms: crossbeam_channel::Sender<SearchCheckIn>) -> bool {
        let lock = state.lock().unwrap();
        self.searcher.echo.insert(lock.info.zkey);
        drop(lock);
        self.searcher.driver.communicate_on(comms);
        self.searcher.tree.clear();
        let m = self.searcher.run(state.clone());
        let mut locked = state.lock().unwrap();
        locked.make_motion(&m, true);
        self.searcher.echo.insert(locked.info.zkey);
        drop(locked);
        return true;
    }
}

impl Manager {
    pub fn init(frame: egui::Context, sender: crossbeam_channel::Sender<SharedState>, receiver: crossbeam_channel::Receiver<Input>, init_fen: String, playing_area: f32, info_width: f32) {
        let asm = if cfg!(feature = "use_asm") {
            println!("using asm");
            true
        } else {
            println!("NOT using asm");
            false
        };
        // let mut benchmode = true;
        let mut benchmode = false;
        for arg in std::env::args() {
            if arg == "bench" {
                benchmode = true;
            }
        }

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
            last_worker_notice: None,
            stable_board: [0u8; 64],
            benchmode,
            asm
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
            if self.worker.is_none() && tmplock.turn == Parity::WHITE && tmplock.moves.white_vect.is_empty() {
                self.game.game_over = true;
            }
            if self.worker.is_none() && tmplock.turn == Parity::BLACK && tmplock.moves.black_vect.is_empty() {
                self.game.game_over = true;
            }
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
                    visuals: VisualInfo::weight_eval(
                        &self.game.visual_weights, 
                        self.current_eval.clone()
                    )
                });
                drop(locked);
                thread::sleep(Duration::from_millis(16));
                self.frame.request_repaint();
                continue;
            }
            if self.worker.as_ref().is_some_and(|x| !x.is_finished()) {
                if let Some(comms) = &self.working_channel_recv {
                    let recvv = comms.try_iter().last();
                    if let Some(last) = recvv {
                        let _ = self.sender.send(SharedState {
                            board: Some(self.stable_board),
                            turn: None,
                            waiting_for_a_human_input: Some(false),
                            moves: None,
                            allowed_castles: None,
                            working: Some(true),
                            game_over: None,
                            selected: None,
                            visuals: VisualInfo::all(
                                &self.game.visual_weights.unwrap(),
                                self.current_eval.clone(),
                                last.tree.clone(),
                                last.cache_saves,
                                last.positions_looked_at,
                                last.mtm,
                                &last.considerations
                            )
                        });
                        self.last_worker_notice = Some(last);
                    }
                }
                self.frame.request_repaint();
                continue;
            } else {
                if let Some(w) = self.worker.take() {
                    if !w.join().unwrap_or(false) {
                        self.game.game_over = true;
                    }
                    if let Some(last) = &self.last_worker_notice {
                        let mut file = OpenOptions::new()
                            .write(true)
                            .append(true)
                            .create(true)
                            .open(format!("bench.{}.txt", if self.asm {"asm"} else { "no_asm" } ))
                            .unwrap();
                        writeln!(file, "{}", last.positions_looked_at).unwrap();
                    }
                }
                self.worker = None;
            }
            let mut locked = self.game.state.lock().unwrap();
            if locked.turn == self.game.human_player || self.game.human_player == Parity::BOTH {
                if self.benchmode {
                    let _ = self.sender.send(SharedState {
                        waiting_for_a_human_input: Some(false),
                        turn: Some(self.game.human_player),
                        moves: Some(locked.moves.parity_moves(self.game.human_player)),
                        selected: Some(self.game.selected),
                        board: Some(locked.board),
                        allowed_castles: Some(locked.info.allowed_castles),
                        working: Some(false),
                        game_over: Some(false),
                        visuals: VisualInfo::weight_eval(&self.game.visual_weights, self.current_eval.clone())
                    }).unwrap();
                    self.frame.request_repaint();
                    let motions = locked.moves.parity_vect(locked.turn);
                    if motions.is_empty() {
                        panic!();
                    }
                    locked.make_motion(motions.select_random().as_ref().unwrap().unwrap(), true);
                    let _ = self.sender.send(SharedState {
                        waiting_for_a_human_input: Some(false),
                        turn: Some(self.game.human_player),
                        moves: Some(locked.moves.parity_moves(self.game.human_player)),
                        selected: Some(self.game.selected),
                        board: Some(locked.board),
                        allowed_castles: Some(locked.info.allowed_castles),
                        working: Some(false),
                        game_over: Some(false),
                        visuals: VisualInfo::weight_eval(&self.game.visual_weights, self.current_eval.clone())
                    }).unwrap();
                    drop(locked);
                    continue;
                } else {
                    let _ = self.sender.send(SharedState {
                        waiting_for_a_human_input: Some(true),
                        turn: Some(self.game.human_player),
                        moves: Some(locked.moves.parity_moves(self.game.human_player)),
                        selected: Some(self.game.selected),
                        board: Some(locked.board),
                        allowed_castles: Some(locked.info.allowed_castles),
                        working: Some(false),
                        game_over: Some(false),
                        visuals: VisualInfo::weight_eval(&self.game.visual_weights, self.current_eval.clone())
                    }).unwrap();
                    match self.receiver.try_recv() {
                        Ok(x) => {
                            if x.left {
                                drop(locked);
                                self.game.human_input(x.pos.unwrap_or_default(), self.game.human_player);
                                let locked = self.game.state.lock().unwrap();
                                let _ = self.sender.send(SharedState {
                                    waiting_for_a_human_input: None,
                                    turn: Some(locked.turn),
                                    moves: Some(locked.moves.parity_moves(locked.turn)),
                                    selected: Some(self.game.selected),
                                    board: Some(locked.board),
                                    allowed_castles: Some(locked.info.allowed_castles),
                                    working: Some(false),
                                    game_over: Some(false),
                                    visuals: VisualInfo::weight_eval(&self.game.visual_weights, self.current_eval.clone())
                                }).unwrap();
                                drop(locked);
                            }
                        },
                        n @ Err(crossbeam_channel::TryRecvError::Disconnected) => panic!("{:?}", n),
                        Err(crossbeam_channel::TryRecvError::Empty) => ()
                    };
                }
                self.frame.request_repaint();
                thread::sleep(Duration::from_millis(16));
                continue;
            } else {
                if self.worker.is_none() {
                    let option_player = if locked.turn == Parity::WHITE { &self.game.players.0 } else { &self.game.players.1 };
                    if let Some(p) = option_player {
                        self.stable_board = locked.board;

                        let data = Arc::clone(&self.game.state);
                        
                        let player = Arc::clone(p);
                        let (send, recv) = crossbeam_channel::unbounded();
                        self.working_channel_recv = Some(recv);
                        let worker = thread::spawn(move || {
                            return player.lock().unwrap().your_turn(data, send);
                        });
                        drop(locked);
                        self.worker = Some(worker);
                    }
                }
            }
        }
    }
}
