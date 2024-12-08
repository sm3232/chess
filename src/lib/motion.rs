use super::{mask::Mask, piece::Parity};

#[derive(Copy)]
pub struct Motion {
    pub from: usize,
    pub to: usize
}
impl Clone for Motion {
    fn clone(&self) -> Self { Self { to: self.to, from: self.from } }
}
impl Default for Motion { fn default() -> Self { Self { from: 65, to: 65 } } }

impl std::fmt::Debug for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "Motion from {}, `to {}", self.from, self.to);
    }
}

#[derive(Clone)]
pub struct MotionSet {
    pub white_moves: [Vec<Motion>; 64],
    pub white_flat: Mask,
    pub white_piecewise_flat: [Mask; 64],
    pub white_defensive_moves: [Vec<Motion>; 64],
    pub white_defensive_flat: Mask,
    pub white_defensive_piecewise_flat: [Mask; 64],
    pub white_vect: Vec<Motion>,

    pub black_moves: [Vec<Motion>; 64],
    pub black_flat: Mask,
    pub black_piecewise_flat: [Mask; 64],
    pub black_defensive_moves: [Vec<Motion>; 64],
    pub black_defensive_flat: Mask,
    pub black_defensive_piecewise_flat: [Mask; 64],
    pub black_vect: Vec<Motion>,
}

impl MotionSet {
    pub fn parity_moves(&self, parity: Parity) -> [Vec<Motion>; 64] {
        if parity == Parity::WHITE {
            return self.white_moves.clone();
        } else {
            return self.black_moves.clone();
        }
    }
    pub fn parity_flat(&self, parity: Parity) -> Mask {
        if parity == Parity::WHITE {
            return self.white_flat;
        } else {
            return self.black_flat;
        }
    }
    pub fn parity_piecewise_flat(&self, parity: Parity) -> [Mask; 64] {
        if parity == Parity::WHITE {
            return self.white_piecewise_flat;
        } else {
            return self.black_piecewise_flat
        }
    }
    pub fn parity_vect(&self, parity: Parity) -> Vec<Motion> {
        if parity == Parity::WHITE {
            return self.white_vect.clone();
        } else {
            return self.black_vect.clone();
        }
    }
    pub fn parity_defense_moves(&self, parity: Parity) -> [Vec<Motion>; 64]{
        if parity == Parity::WHITE {
            return self.white_defensive_moves.clone();
        } else {
            return self.black_defensive_moves.clone();
        }
    }
    pub fn parity_defense_flat(&self, parity: Parity) -> Mask {
        if parity == Parity::WHITE {
            return self.white_defensive_flat;
        } else {
            return self.black_defensive_flat;
        }
    }
    pub fn parity_defense_piecewise_flat(&self, parity: Parity) -> [Mask; 64]{
        if parity == Parity::WHITE {
            return self.white_defensive_piecewise_flat
        } else {
            return self.black_defensive_piecewise_flat;
        }
    }
}

impl Default for MotionSet {
    fn default() -> Self {
        Self {
            white_moves: [const { Vec::new() }; 64],
            white_flat: Mask::default(),
            white_piecewise_flat: [Mask::default(); 64],
            white_defensive_moves: [const {Vec::new()}; 64],
            white_defensive_flat: Mask::default(),
            white_defensive_piecewise_flat: [Mask::default(); 64],
            white_vect: Vec::new(),

            black_moves: [const { Vec::new() }; 64],
            black_flat: Mask::default(),
            black_piecewise_flat: [Mask::default(); 64],
            black_defensive_moves: [const {Vec::new()}; 64],
            black_defensive_flat: Mask::default(),
            black_defensive_piecewise_flat: [Mask::default(); 64],
            black_vect: Vec::new(),
        }
    }
}
