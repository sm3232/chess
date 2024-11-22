
use crate::shared::{mask::Mask, piece::Parity, state::State};

pub trait Player {
    fn get_parity(&self) -> Parity;
    fn your_turn(&self, state: &State) -> (Mask, Mask);
}
