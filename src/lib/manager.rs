
use eframe::egui;
use std::{cell::RefCell, rc::Rc, sync::{mpsc, Arc, Mutex}, thread::{JoinHandle, Thread}, time::Duration};
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

use super::{motion::Motion, state::State, ui::Input};
pub struct SharedState {
    pub board: [u8; 64],
    pub allowed_castles: u8,
    pub moves: [Vec<Motion>; 64],
    pub tree: Arc<Mutex<SearchTree>>,
    pub visual_weights: Option<[i32; 64]>,
    pub waiting_for_a_human_input: bool,
    pub turn: Parity,
    pub selected: usize,
    pub evaluation: Evaluator
}

pub struct Manager {
    pub game: ChessGame,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub current_eval: Evaluator,
    pub sender: mpsc::Sender<SharedState>,
    pub receiver: mpsc::Receiver<Input>,
    frame: egui::Context,
    worker: Option<JoinHandle<()>>,
}

pub struct ManagerPlayer { parity: Parity, searcher: Searcher }

impl ManagerPlayer {
    fn new(parity: Parity) -> Self {
        return Self {
            parity,
            searcher: Searcher {
                tree: Vec::new(),
                ply: 3,
                primary: parity,
                heap: Heap::default()
            }
        };
    }
}
impl Player for ManagerPlayer {
    fn get_parity(&self) -> Parity {
        return self.parity;
    }
    fn your_turn(&self, state: Arc<Mutex<State>>) -> () {
        let mut s = Searcher {
            tree: Vec::new(),
            ply: 3,
            primary: self.parity,
            heap: Heap::default()
        };
        s.pv(state.clone(), i32::MIN + 100, i32::MAX - 100, s.ply);
        println!("Heap peek eval: {}, {} -> {}", s.heap.peek().evaluation, s.heap.peek().motion.from, s.heap.peek().motion.to);
        
        if s.heap.empty() {
            panic!("No move");
        } else {
            let mut locked = state.lock().unwrap();
            locked.make_motion(&s.heap.pop().motion, true);
            drop(locked);
        }
        // return Some(s.tree[0].clone());
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
        mgr.game.register_players(None, Some(Arc::new(ManagerPlayer::new(Parity::BLACK))));
        let tmplock = mgr.game.state.lock().unwrap();
        mgr.current_eval = eval::start_eval(&tmplock);
        drop(tmplock);
        mgr.begin();
    }
    pub fn begin(&mut self) -> () {
        loop {
            if self.worker.as_ref().is_some_and(|x| !x.is_finished()) {


                thread::sleep(Duration::from_millis(16));
                let locked = self.game.state.lock().unwrap();

                let (p1, p2) = &self.game.players;
                
                let p = match (p1.is_some(), p2.is_some()) {
                    (false, false) => None,
                    (true, false) => if locked.turn == Parity::WHITE { Some(p1.clone().unwrap()) } else { None },
                    (false, true) => if locked.turn == Parity::BLACK { Some(p2.clone().unwrap()) } else { None },
                    (true, true) => if locked.turn == Parity::WHITE { Some(p1.clone().unwrap()) } else { Some(p2.clone().unwrap()) }
                };
                
                if let Some(player) = p {
                    let _ = self.sender.send(SharedState {
                        board: locked.board,
                        turn: locked.turn,
                        waiting_for_a_human_input: false,
                        moves: locked.moves.clone(),
                        visual_weights: self.game.visual_weights,
                        evaluation: self.current_eval.clone(),
                        allowed_castles: locked.info.allowed_castles,
                        selected: self.game.selected,
                        tree: Arc::clone(locked.tree_root.as_ref().unwrap())
                    });

                }
                self.frame.request_repaint();
                continue;
            } else {
                self.worker = None;
            }
            let locked = self.game.state.lock().unwrap();
            if locked.turn == self.game.human_player || self.game.human_player == Parity::BOTH {
                let _ = self.sender.send(SharedState {
                    tree: SearchTree::root(locked.turn),
                    waiting_for_a_human_input: true,
                    turn: self.game.human_player,
                    moves: locked.moves.clone(),
                    selected: self.game.selected,
                    board: locked.board,
                    allowed_castles: locked.info.allowed_castles,
                    evaluation: self.current_eval.clone(),
                    visual_weights: self.game.visual_weights
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
                            player.your_turn(data);
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
