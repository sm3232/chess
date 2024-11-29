
use std::{cell::RefCell, rc::Rc};

use crate::shared::{searchtree::SearchTree, mask::Mask, piece::Parity, state::State};

pub trait Player {
    fn get_parity(&self) -> Parity;
    fn your_turn(&self, state: Rc<RefCell<State>>) -> Option<Rc<RefCell<SearchTree>>>;
}
