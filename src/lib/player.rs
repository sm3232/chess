use std::{cell::RefCell, rc::Rc, sync::{mpsc, Arc, Mutex}};

use crate::lib::{
    searchtree::SearchTree,
    piece::Parity,
    state::State
};

use super::searcher::SearchCheckIn;

pub trait Player: Send + Sync {
    fn get_analyzed(&self) -> usize;
    fn get_cache_saves(&self) -> usize;
    fn get_parity(&self) -> Parity;
    fn your_turn(&mut self, state: Arc<Mutex<State>>, comms: mpsc::Sender<SearchCheckIn>) -> bool;
}
