use stanza::renderer::{console::Console, Renderer};

use crate::lib::{
    boardarray::BoardArray, chessbyte::ChessByte, cutil::pretty_print::{pretty_print_board, pretty_print_mask}, mask::{DepthMask, Mask}, piece::{Parity, PieceByte}, state::State
};

#[derive(Clone)]
pub struct EvaluationTerm {
    pub white_score: i32,
    pub black_score: i32,
    pub name: String
}
impl std::fmt::Display for EvaluationTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}\tW{}\tB{}", self.name, self.white_score, self.black_score) }
}
impl std::fmt::Debug for EvaluationTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}\tW{}\tB{}", self.name, self.white_score, self.black_score) }
}
#[derive(Clone)]
pub struct Evaluator {
    pub eval: i32,
    pub scores: Vec<EvaluationTerm>,
}

impl Evaluator {
    pub fn push(&mut self, name: &str, scorepos: i32, scoreneg: i32) -> () {
        self.scores.push(EvaluationTerm {
            white_score: scorepos,
            black_score: scoreneg,
            name: name.to_string()
        });
    }
    pub fn finalize(&mut self, halfmove: u64) -> () {
        for i in self.scores.iter() {
            self.eval += i.white_score + i.black_score;
        }
        self.eval *= (100 - halfmove as i32) / 100;
    }
}
fn flip_castles(castles: u8) -> u8 {
    let low = castles & 0b0000_0011;
    let high = castles & 0b0000_1100;
    return (low << 2) | (high >> 2);
}

    /*
    v += mobility_mg(pos) - mobility_mg(colorflip(pos));
    v += threats_mg(pos) - threats_mg(colorflip(pos));
    v += passed_mg(pos) - passed_mg(colorflip(pos));
    v += space(pos) - space(colorflip(pos));
    v += king_mg(pos) - king_mg(colorflip(pos));
    if (!nowinnable) v += winnable_total_mg(pos, v);
    */
const DO_TIMING: bool = false;
pub fn start_eval(state: &State) -> Evaluator {
    let mut evaluator = Evaluator {
        eval: 0,
        scores: Vec::new(),
    };
    let zbrist = state.zobrist.lock().unwrap();
    if let Some(pulled) = zbrist.pull(state.info.zkey) {
        if pulled.2.is_some() {
            return pulled.2.unwrap();
        }
    }
    drop(zbrist);
    if !(state.board[state.info.king_indices[0]].is_w_king() && state.board[state.info.king_indices[1]].is_b_king()) {
        evaluator.eval = i32::MIN;
        return evaluator;
    }
    
    let flipped = state.partial_flipped();
    let fmoves = flipped.board.get_motions(&flipped.maskset, &flipped.enpassant_mask, Some(flipped.allowed_castles));

    let mut ev = std::time::Instant::now();
    evaluator.push("Material", material::midgame_material(&state.board), -material::midgame_material(&flipped.board));
    if DO_TIMING { println!("Material {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Weights", material::midgame_weighted_position(&state.board), -material::midgame_weighted_position(&flipped.board));
    if DO_TIMING { println!("Weights {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("PImbalance", imbalance::piece_imbalance(&state.board) / 16, -imbalance::piece_imbalance(&flipped.board) / 16);
    if DO_TIMING { println!("PImbalance {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("BImbalance", imbalance::bishop_pair(&state.board) / 16, -imbalance::bishop_pair(&flipped.board) / 16);
    if DO_TIMING { println!("BImbalance {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Pawn", pawn::midgame_pawn(&state.board), -pawn::midgame_pawn(&flipped.board));
    if DO_TIMING { println!("Pawn {:.2?}", ev.elapsed()) };
    
    ev = std::time::Instant::now();
    evaluator.push("Pieces", pieces::midgame_pieces(&state.board, state.info.allowed_castles, &state.moves), -pieces::midgame_pieces(&flipped.board, flipped.allowed_castles, &fmoves)); 
    if DO_TIMING { println!("Pieces {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    if state.turn == Parity::WHITE {
        evaluator.push("Tempo", general::tempo(state.turn), 0);
    } else {
        evaluator.push("Tempo", 0, general::tempo(state.turn));
    }
    if DO_TIMING { println!("Tempo {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Mobility", mobility::bonus(&state.board, &state.moves), -mobility::bonus(&flipped.board, &fmoves));
    if DO_TIMING { println!("Mobility {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Threats", threats::threats(&state.board, &state.moves, &state.info.maskset, state.info.king_indices[0]), -threats::threats(&flipped.board, &fmoves, &flipped.maskset, flipped.king_indices[0]));
    if DO_TIMING { println!("Threats {:.2?}", ev.elapsed()) };
    // mobility 
    // threats 
    // evaluator.push("Passed", general::passed(&state.board, &flipped.board, &state.moves, &flipped_moves), -general::passed(&flipped.board, &state.board, &flipped_moves, &state.moves));
    // space 
    // king 
    // winnable
    
    /*
    let mut endgame = endgame_eval(state);
    
    let phase = phase(&state.board);
    endgame *- scale_factor()

    eg = eg * scale_factor(pos, eg) / 64;
    var v = (((mg * p + ((eg * (128 - p)) << 0)) / 128) << 0);
    if (arguments.length == 1) v = ((v / 16) << 0) * 16;
    
    */
    // midgame += tempo(pos);
    // v = (v * (100 - rule50) / 100) << 0;
    evaluator.finalize(state.info.halfmove_clock);
    let mut zbrist2 = state.zobrist.lock().unwrap();
    zbrist2.save((state.info.clone(), state.moves.clone(), Some(evaluator.clone())));
    drop(zbrist2);
    return evaluator;

}
mod threats {
    use stanza::renderer::{console::Console, Renderer};

    use crate::lib::{
        chessbyte::ChessByte, cutil::pretty_print::{pretty_print_mask, pretty_print_masks, pretty_print_value_mask, value_mask_to_table}, mask::{Mask, ValueMask}, maskset::MaskSet, motion::{Motion, MotionSet}, piece::{
            Parity, 
            PieceByte
        }
    };

    use super::{baccess, general::count_attacks, mobility::mobility_area};
    pub const WEIGHT_HANGING_THREAT: i32 = 69;
    pub const WEIGHT_KING_THREAT: i32 = 24;
    pub const WEIGHT_PAWN_PUSH: i32 = 48;
    pub const WEIGHT_SAFE_PAWN: i32 = 173;
    pub const WEIGHT_QUEEN_SLIDER: i32 = 60;
    pub const WEIGHT_QUEEN_KNIGHT: i32 = 16;
    pub const WEIGHT_RESTRICTED_THREAT: i32 = 7;
    pub const WEIGHT_WEAK_QUEEN_PROTECTION: i32 = 14;
    pub const WEIGHT_MINOR_THREAT: [i32; 5] = [5, 57, 77, 88, 79];
    pub const WEIGHT_ROOK_THREAT: [i32; 5] = [3, 37, 42, 0, 58];
    fn weak_enemy(board: &[u8; 64], flipped: &[u8; 64], index: usize, moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64]) -> i32 {
        if board[index].is_black() || !board[index].is_piece() { return 0 };
        let xi = index % 8;
        let yi = index / 8;
        if baccess(board, xi.checked_sub(1), yi.checked_sub(1)).is_b_pawn() { return 0 };
        if baccess(board, xi.checked_add(1), yi.checked_add(1)).is_b_pawn() { return 0 };
        if count_attacks(board, moves, index) <= 1 && count_attacks(flipped, flipped_moves, (7 - yi) * 8 + xi) > 1 {
            return 0;
        }
        return 1;
    }
    fn hanging(board: &[u8; 64], flipped: &[u8; 64], index: usize, moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64]) -> i32 {
        if weak_enemy(board, flipped, index, moves, flipped_moves) == 0 { return 0 };
        if !board[index].is_b_pawn() && count_attacks(board, moves, index) > 1 { return WEIGHT_HANGING_THREAT };    
        let xi = index % 8;
        let yi = index / 8 ;
        if count_attacks(flipped, flipped_moves, (7 - yi) * 8 + xi) > 1 {
            return WEIGHT_HANGING_THREAT;
        }
        return 0;
    }
    fn sum_of_calls(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], func: fn(&[u8; 64], &[u8; 64], &[Vec<Motion>; 64],&[Vec<Motion>; 64], Option<usize>) -> i32) -> i32 {
        let mut sum = 0;
        for i in 0..64 {
            sum += func(board, flipped, moves, flipped_moves, Some(i));
        }
        return sum;
    }
    fn pawn_push(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], index: Option<usize>) -> i32 {
        if let Some(i) = index {
            let xi = i % 8;
            let yi = i / 8;
            if !board[i].is_black() || !board[i].is_piece() { return 0 };
            if baccess(board, xi.checked_sub(1), yi.checked_add(2)).is_w_pawn() 
                && !baccess(board, xi.checked_sub(1), yi.checked_add(1)).is_piece() 
                    && !baccess(board, xi.checked_sub(2), Some(yi)).is_b_pawn() 
                    && !baccess(board, Some(xi), Some(yi)).is_b_pawn() {
                        if let Some(addx) = xi.checked_sub(1) {
                            if let Some(addy) = yi.checked_add(1) {
                                if count_attacks(board, moves, addy * 8 + addx) > 0 {
                                    return 1;
                                }
                            }
                            if let Some(addy) = 6usize.checked_sub(yi) {
                                if count_attacks(flipped, flipped_moves, addy * 8 + addx) == 0 {
                                    return 1;
                                }
                            }
                        }
                    }
            if yi == 3
                && baccess(board, xi.checked_sub(1), yi.checked_add(3)).is_w_pawn() 
                    && !baccess(board, xi.checked_sub(1), yi.checked_add(2)).is_piece()
                    && !baccess(board, xi.checked_sub(1), yi.checked_add(1)).is_piece()
                    && !baccess(board, xi.checked_sub(2), Some(yi)).is_b_pawn()
                    && !baccess(board, Some(xi), Some(yi)).is_b_pawn() {
                        if let Some(addx) = xi.checked_sub(1) {
                            if let Some(addy) = yi.checked_add(1) {
                                if count_attacks(board, moves, addy * 8 + addx) > 0 {
                                    return 1;
                                }
                            }
                            if let Some(addy) = 6usize.checked_sub(yi) {
                                if count_attacks(flipped, flipped_moves, addy * 8 + addx) == 0 {
                                    return 1;
                                }
                            }
                        }

                    }
            if baccess(board, xi.checked_add(1), yi.checked_add(2)).is_w_pawn() 
                && !baccess(board, xi.checked_add(1), yi.checked_add(1)).is_piece() 
                    && !baccess(board, Some(xi), Some(yi)).is_b_pawn() 
                    && !baccess(board, xi.checked_add(2), Some(yi)).is_b_pawn() {
                        if let Some(addx) = xi.checked_add(1) {
                            if let Some(addy) = yi.checked_add(1) {
                                if count_attacks(board, moves, addy * 8 + addx) > 0 {
                                    return 1;
                                }
                            }
                            if let Some(addy) = 6usize.checked_sub(yi) {
                                if count_attacks(flipped, flipped_moves, addy * 8 + addx) == 0 {
                                    return 1;
                                }
                            }
                        }
                    }
            if yi == 3
                && baccess(board, xi.checked_add(1), yi.checked_add(3)).is_w_pawn() 
                    && !baccess(board, xi.checked_add(1), yi.checked_add(2)).is_piece()
                    && !baccess(board, xi.checked_add(1), yi.checked_add(1)).is_piece()
                    && !baccess(board, Some(xi), Some(yi)).is_b_pawn()
                    && !baccess(board, xi.checked_add(2), Some(yi)).is_b_pawn() {
                        if let Some(addx) = xi.checked_add(1) {
                            if let Some(addy) = yi.checked_add(1) {
                                if count_attacks(board, moves, addy * 8 + addx) > 0 {
                                    return 1;
                                }
                            }
                            if let Some(addy) = 6usize.checked_sub(yi) {
                                if count_attacks(flipped, flipped_moves, addy * 8 + addx) == 0 {
                                    return 1;
                                }
                            }
                        }

                    }
            return 0;
        } else {
            return WEIGHT_PAWN_PUSH * sum_of_calls(board, flipped, moves, flipped_moves, pawn_push);
        }
    }
    fn defended_by_any_count(board: &[u8; 64], moves: &[Vec<Motion>; 64], index: usize) -> i32 {
        let mut count = 0;
        for i in 0..64 {
            if board[i].is_piece() && board[i].is_parity(board[index].get_parity()) {
                for m in &moves[i] {
                    if m.to == index {
                        count += 1;
                        break;
                    }
                }
            }
        }
        return count;
    }
    fn safe_pawn(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], index: usize) -> i32 {
        if board[index].is_w_pawn() {
            if defended_by_any_count(board, moves, index) > 0 { return 1 };
            if count_attacks(flipped, flipped_moves, index) == 0 { return 1 };
        }
        return 0;
    }
    fn pawn_attack(board: &[u8; 64], index: usize) -> i32 {
        let mut value = 0;
        let xi = index % 8;
        let yi = index / 8;
        if yi < 7 {
            if xi > 0 && board[index + 7].is_w_pawn() { value += 1 };
            if xi < 7 && board[index + 9].is_w_pawn() { value += 1 };
        }
        return value;
    }
    fn safe_pawn_threat(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], index: Option<usize>) -> i32 {
        if let Some(i) = index {
            if !board[i].is_piece() || !board[i].is_black() || board[i].is_pawn() || board[i].is_king() { return 0 };
            if pawn_attack(board, i) == 0 { return 0 };
            let xi = i % 8;
            let yi = i / 8;
            if yi < 7 {
                if xi > 0 && safe_pawn(board, flipped, moves, flipped_moves, i + 7) > 0 { return 1 };
                if xi < 7 && safe_pawn(board, flipped, moves, flipped_moves, i + 9) > 0 { return 1 };
            }
            return 0;
        } else {
            return WEIGHT_SAFE_PAWN * sum_of_calls(board, flipped, moves, flipped_moves, safe_pawn_threat)
        }
    }
    fn would_threaten_index(board: &[u8; 64], index_from: usize, index_to: usize, focus: usize) -> bool {
        let xto = index_to % 8;
        let yto = index_to / 8;
        let xfocus = focus % 8;
        let yfocus = focus / 8;
        if board[index_from].is_rook() {
            if xto == xfocus || yto == yfocus {
                return true;
            }
        } else if board[index_from].is_bishop() {
            if xto.abs_diff(xfocus) == yto.abs_diff(yfocus) {
                return true;
            }
        } else if board[index_from].is_knight() {
            if xto.abs_diff(xfocus) == 2 && yto.abs_diff(yfocus) == 1 {
                return true;
            }
            if xto.abs_diff(xfocus) == 1 && yto.abs_diff(yfocus) == 2 {
                return true;
            }
        }
        return false;
    }
    fn queen_knight(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], queen: usize, index: Option<usize>) -> i32 {
        if let Some(i) = index {
            if board[i].is_w_knight() {
                for m in &moves[i] {
                    if would_threaten_index(board, m.from, m.to, queen) {
                        return 1;
                    }
                }
            }
            return 0;
        } else {
            let mut sum = 0;
            for i in 0..64 {
                sum += queen_knight(board, flipped, moves, flipped_moves, queen, Some(i));
            }
            return WEIGHT_QUEEN_KNIGHT * sum;
        }

    }
    fn restricted_threat(moves: &MotionSet) -> u32 {
        return (moves.white_flat & (moves.black_defensive_flat | moves.black_flat)).bit_count();
    }
    fn threat_values(moves: &MotionSet, maskset: &MaskSet) -> ValueMask {
        let mut vals = ValueMask::default();
        let mut mask = Mask::zz();
        loop {
            let white_threats = moves.white_flat & maskset.black;
            let black_defense = moves.black_defensive_flat & maskset.black;
            vals.add_assign(&(white_threats & mask));
            vals.sub_assign(&(black_defense & mask));

            if mask.raw.leading_zeros() == 0 { break };
            mask.raw <<= 1;
        }
        return vals;
    }
    fn weak_queen_protection(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, queen: usize) -> u32 {
        let mut vm = ValueMask::default();
        for i in 0..64 {
            if board[i].is_white() || !board[i].is_piece() { continue };
            vm.add_assign(&moves.black_defensive_piecewise_flat[i]);
        }
        return (vm.to_mask(1) & moves.black_defensive_piecewise_flat[queen] & (moves.white_flat & maskset.black)).bit_count();

    }
    /*
    fn weak_queen_protection(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], queen: usize, index: Option<usize>) -> i32 {
        if let Some(i) = index {
            if weak_enemy(board, flipped, i, moves, flipped_moves) == 0 { return 0 };
            let qx = queen % 8;
            let qy = queen / 8;
            let qf = (7 - qy) * 8 + qx;
            for m in &flipped_moves[qf] {
                if m.to == i {
                    return 1;
                }
            }
            return 0;
        } else {
            let mut sum = 0;
            for i in 0..64 {
                sum += weak_queen_protection(board, flipped, moves, flipped_moves, queen, Some(i));
            }
            return WEIGHT_WEAK_QUEEN_PROTECTION * sum;
        }
    }
    */
    fn pawn_push_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet) -> u32 {
        let mut c = 0;
        for i in 16..56 {
            if board[i].is_w_pawn() {
                let pwf = (moves.white_piecewise_flat[i] & Mask::of_column(i % 8)) | (moves.white_piecewise_flat[i] & maskset.black);
                for bit in &pwf.isolated_bits() {
                    let dgsb = bit.get_diags_below();
                    if ((dgsb.0 | dgsb.1) & maskset.white).any() {
                        let dgsa = bit.get_diags_above();
                        c += ((dgsa.0 | dgsa.1) & maskset.black).bit_count();
                    }
                }
            }
        }
        return c;
    }
    fn pawn_safe_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet) -> u32 {
        let mut c = 0;
        for i in 8..56 {
            if board[i].is_w_pawn() {
                let m = Mask::from_index(i);
                if (moves.white_defensive_flat & m).any() {
                    c += (moves.white_piecewise_flat[i] & maskset.black & Mask::of_column(i % 8).get_not()).bit_count();
                }
            }
        }
        return c;
    }
    /*
    fn king_threat(board: &[u8; 64], flipped: &[u8; 64], index: usize, moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], king: usize) -> i32 {

        if !board[index].is_piece() || !board[index].is_black() { return 0 };
        if weak_enemy(board, flipped, index, moves, flipped_moves) == 0 { return 0 };
        for m in &moves[king] {
            if m.to == index {
                return WEIGHT_KING_THREAT;
            }
        }
        return 0;
    }
    */
    fn king_threat(moves: &MotionSet, maskset: &MaskSet, king: usize) -> u32 {
        return (moves.white_piecewise_flat[king] & maskset.black & moves.black_defensive_flat.get_not()).bit_count();
    }
    fn queen_slider(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], queen: usize, index: Option<usize>) -> i32 {
        if let Some(i) = index {
            if board[i].is_white() {
                if board[i].is_rook() || board[i].is_bishop() {
                    for m in &moves[i] {
                        if would_threaten_index(board, m.from, m.to, queen) {
                            return 1;
                        }
                    }
                }
            }
            return 0;
        } else {
            let mut sum = 0;
            for i in 0..64 {
                sum += queen_slider(board, flipped, moves, flipped_moves, queen, Some(i));
            }
            return WEIGHT_QUEEN_SLIDER * sum;
        }
    }
    fn queen_knight_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, queen: usize) -> u32 {
        let qm = Mask::from_index(queen);
        let knightish = qm.get_knightish();
        let anded = knightish & moves.white_flat & moves.black_defensive_flat.get_not();
        if anded.none() { return 0 };
        let mut c = 0;
        for iso in anded.isolated_bits() {
            for isok in iso.get_knightish().isolated_bits() {
                if board[isok.as_index()].is_w_knight() {
                    c += 1;
                }
            }
        }
        return c;
    }
    fn queen_slide_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, queen: usize) -> u32 {
        let qm = Mask::from_index(queen);
        let qx = queen % 8;
        let qy = queen / 8;
        let mut c = 0;
        for m in moves.white_piecewise_flat {
            if (m & qm).any() {
                let defended = m & moves.white_defensive_flat;
                let mi = m.as_index();
                let xi = mi % 8;
                let yi = mi / 8;
                let mut filter = Mask::default().get_not();
                if qy < yi {
                    filter &= Mask::all_rows_above(yi);
                } else if qy > yi {
                    filter &= Mask::all_rows_below(yi);
                }
                if qx < xi {
                    filter &= Mask::all_cols_left(xi);
                } else if qx > xi {
                    filter &= Mask::all_cols_right(xi);
                }
                c += (defended & filter).bit_count();
            }
        }
        return c;
    }
    fn minor_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, threats: &ValueMask) -> i32 {
        let ge0 = threats.to_mask_ge0() & maskset.black;
        let mut c = 0;
        for i in 0..64 {
            if board[i].is_piece() && (board[i].is_w_bishop() || board[i].is_w_knight()) {
                for iso in (ge0 & moves.white_piecewise_flat[i]).isolated_bits() {
                    c += match board[iso].get_piece() {
                        PieceByte::PAWN => WEIGHT_MINOR_THREAT[0],
                        PieceByte::KNIGHT => WEIGHT_MINOR_THREAT[1],
                        PieceByte::BISHOP => WEIGHT_MINOR_THREAT[2],
                        PieceByte::ROOK => WEIGHT_MINOR_THREAT[3],
                        PieceByte::QUEEN => WEIGHT_MINOR_THREAT[4],
                        _ => 0
                    };
                }
            }
        }
        return c;
    }
    fn rook_threat(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, threats: &ValueMask) -> i32 {
        let ge0 = threats.to_mask_ge0() & maskset.black;
        let mut c = 0;
        for i in 0..64 {
            if board[i].is_piece() && board[i].is_rook() {
                for iso in (ge0 & moves.white_piecewise_flat[i]).isolated_bits() {
                    c += match board[iso].get_piece() {
                        PieceByte::PAWN => WEIGHT_ROOK_THREAT[0],
                        PieceByte::KNIGHT => WEIGHT_ROOK_THREAT[1],
                        PieceByte::BISHOP => WEIGHT_ROOK_THREAT[2],
                        PieceByte::ROOK => WEIGHT_ROOK_THREAT[3],
                        PieceByte::QUEEN => WEIGHT_ROOK_THREAT[4],
                        _ => 0
                    };
                }
            }
        }
        return c;
    }
    pub fn threats(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet, king_index: usize) -> i32 {

    // pub fn threats(board: &[u8; 64], moves: &[Vec<Motion>; 64], flipped: &[u8; 64], flipped_moves: &[Vec<Motion>; 64], flat: &MaskSet, maskset: &MaskSet, simple_moves: &[Mask; 64]) -> i32 {
        let mut wqueen: Vec<usize> = Vec::new();
        let mut bqueen: Vec<usize> = Vec::new();
        for i in 0..64 {
            if board[i].is_w_queen() { wqueen.push(i) };
            if board[i].is_b_queen() { bqueen.push(i) };
        }

        let threat_vals = threat_values(moves, maskset);
        let restricted = WEIGHT_RESTRICTED_THREAT * restricted_threat(&moves) as i32;
        let hanging = WEIGHT_HANGING_THREAT * threat_vals.count_positive();

        let pawn_push = WEIGHT_PAWN_PUSH * pawn_push_threat(board, moves, maskset) as i32;
        let safe_pawn = WEIGHT_SAFE_PAWN * pawn_safe_threat(board, moves, maskset) as i32;
        let king_threat = WEIGHT_KING_THREAT * king_threat(moves, maskset, king_index) as i32;

        let mut queen_slider = 0;
        let mut queen_knight = 0;
        let mut queen_weak_protection = 0;
        for bq in bqueen {
            queen_slider += WEIGHT_QUEEN_SLIDER * queen_slide_threat(board, moves, maskset, bq) as i32;
            queen_knight += WEIGHT_QUEEN_KNIGHT * queen_knight_threat(board, moves, maskset, bq) as i32;
            queen_weak_protection += WEIGHT_WEAK_QUEEN_PROTECTION * weak_queen_protection(board, moves, maskset, bq) as i32;
        }
        let minor = minor_threat(board, moves, maskset, &threat_vals);
        let rook = rook_threat(board, moves, maskset, &threat_vals);

        /*
        println!("Restricted: {restricted}");
        println!("Hanging: {hanging}");
        println!("Pawn Push: {pawn_push}");
        println!("Safe Pawn: {safe_pawn}");
        println!("King: {king_threat}");
        println!("Queen Slider: {queen_slider}");
        println!("Queen Knight: {queen_knight}");
        println!("Queen Weak Protection: {queen_weak_protection}");
        println!("Minor: {minor}");
        println!("Rook: {rook}");
        */

        return restricted + hanging + pawn_push + safe_pawn + king_threat + queen_slider + queen_knight + queen_weak_protection + minor + rook;
    }

}


fn baccess(board: &[u8; 64], xi: Option<usize>, yi: Option<usize>) -> u8 {
    if let (Some(x), Some(y)) = (xi, yi) {
        if x < 8 && y < 8 && y * 8 + x < 64 {
            return board[y * 8 + x];
        }
    }
    return 0u8;
}
mod captures {
    use crate::lib::{
        chessbyte::ChessByte, motion::Motion, piece::{Parity, PieceByte}
    };
    use super::material;

    const WEIGHT_CAPTURE: i32 = -1;
    pub fn captures(board: &[u8; 64], moves: &[Vec<Motion>; 64]) -> i32 {
        let mut value = 0;
        for i in 0..64 {
            if board[i].is_white() {
                for m in &moves[i] {
                    if board[m.to].is_piece() {
                        value += material::material_value_of_index(board, m.to);
                    }
                }
            }
        }
        return value * WEIGHT_CAPTURE;
    }
}
mod general {
    use crate::lib::{
        chessbyte::ChessByte, motion::Motion, piece::{Parity, PieceByte}
    };

    use super::baccess;
    const WEIGHT_TEMPO: i32 = 28;
    const WEIGHT_PASSED_FILE: i32 = -11;
    pub fn tempo(parity: Parity) -> i32 { WEIGHT_TEMPO * if parity == Parity::WHITE { 1 } else { -1 } }
    fn supported(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        return if baccess(board, xi.checked_sub(1), yi.checked_add(1)).is_w_pawn() { 1 } else { 0 }
            + if baccess(board, xi.checked_add(1), yi.checked_add(1)).is_w_pawn() { 1 } else { 0 };
    }

    fn index_passed(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        let mut yc1 = 8;
        let mut yc2 = 8;
        if board[index].is_white() && board[index].is_pawn() {
            if yi > 0 {
                for y in (yi - 1)..=0 {
                    if board[y * 8 + xi].is_piece() && board[y * 8 + xi].is_pawn() {
                        if board[y * 8 + xi].is_white() { return 0 };
                        yc1 = y;
                    }
                    if y * 8 + xi > 0 && board[y * 8 + xi - 1].is_pawn() && board[y * 8 + xi - 1].is_black() {
                        yc2 = y;
                    }
                    if y * 8 + xi < 63 && board[y * 8 + xi + 1].is_pawn() && board[y * 8 + xi + 1].is_black() {
                        yc2 = y;
                    }
                }
                if yc1 == 8 && yc2 >= yi - 1 {
                    return 1;
                }
                if yi > 1 && (yc2 < yi - 2 || yc1 < yi - 1) { return 0 };
                if yc2 >= yi && yc1 == yi - 1 && yi < 4 {
                    if baccess(board, xi.checked_sub(1), yi.checked_add(1)).is_w_pawn() 
                        && !baccess(board, xi.checked_sub(1), Some(yi)).is_b_pawn()
                        && !baccess(board, xi.checked_sub(2), yi.checked_sub(1)).is_b_pawn() { return 1 };
                    if baccess(board, xi.checked_add(1), yi.checked_add(1)).is_w_pawn() 
                        && !baccess(board, xi.checked_add(1), Some(yi)).is_b_pawn()
                        && !baccess(board, xi.checked_add(2), yi.checked_sub(1)).is_b_pawn() { return 1 };
                }
                if baccess(board, Some(xi), yi.checked_sub(1)).is_b_pawn() { return 0 };
                let lev = if baccess(board, xi.checked_sub(1), yi.checked_sub(1)).is_b_pawn() { 1 } else { 0 } + if baccess(board, xi.checked_add(1), yi.checked_sub(1)).is_b_pawn() { 1 } else { 0 };

                let levp = if baccess(board, xi.checked_sub(1), yi.checked_sub(2)).is_b_pawn() { 1 } else { 0 } + if baccess(board, xi.checked_add(1), yi.checked_sub(2)).is_b_pawn() { 1 } else { 0 };

                let phal = if baccess(board, xi.checked_sub(1), Some(yi)).is_b_pawn() { 1 } else { 0 } + if baccess(board, xi.checked_add(1), Some(yi)).is_b_pawn() { 1 } else { 0 };

                if lev - supported(board, index) > 1 { return 0 };
                if levp - phal > 0 { return 0 };
                if lev > 0 && levp > 0 { return 0 };
                return 1;
            }
        }
        return 0;

    }
    pub fn count_attacks(board: &[u8; 64], moves: &[Vec<Motion>; 64], index: usize) -> i32 {
        let mut count = 0;
        for i in 0..64 {
            if board[i].is_parity(board[index].get_parity()) {
                continue;
            }
            for m in &moves[i] {
                if m.to == index {
                    count += 1;
                }
            }
        }
        return count;
    }
    fn passed_lev(board: &[u8; 64], flipped: &[u8; 64], index: usize, moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64]) -> bool {
        if index_passed(board, index) == 0 { return false };
        let xi = index % 8;
        let yi = index / 8;
        if !baccess(board, Some(xi), yi.checked_sub(1)).is_b_pawn() { return true };
        if baccess(board, xi.checked_sub(1), yi.checked_add(1)).is_w_pawn() {
            if baccess(board, xi.checked_sub(1), Some(yi)).is_white() {
                if count_attacks(board, moves, index - 1) > 0 || count_attacks(flipped, flipped_moves, (7 - yi) * 8 + xi - 1) < 2 {
                    return true;
                }
            }
        }
        if baccess(board, xi.checked_add(1), yi.checked_add(1)).is_w_pawn() {
            if baccess(board, xi.checked_add(1), Some(yi)).is_white() {
                if count_attacks(board, moves, index + 1) > 0 || count_attacks(flipped, flipped_moves, (7 - yi) * 8 + xi + 1) < 2 {
                    return true;
                }
            }
        }
        return false;
    }
    fn passed_block(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        if 8 - yi < 4 {
            return 0;
        }
        if baccess(board, Some(xi), yi.checked_sub(1)).is_piece() {
            return 0;
        }
        let rank = 8 - yi - 1;
        let weight = if rank > 2 { 5 * rank - 13 } else { 0 };
        let mut defend = 0;
        let mut threatened = 0;
        let mut whitethreatened = 0;
        let mut defend1 = 0;
        let mut threatened1 = 0;
        for y in (yi - 1)..=0 {
            if count_attacks(board, moves, y * 8 + xi) > 0 {
                defend += 1;
            }
            if count_attacks(flipped, flipped_moves, (7 - y) * 8 + xi) > 0 {
                threatened += 1;
            }
            if count_attacks(flipped, flipped_moves, (7 - y) * 8 + xi - 1) > 0 {
                whitethreatened += 1;
            }
            if count_attacks(flipped, flipped_moves, (7 - y) * 8 + xi + 1) > 0 {
                whitethreatened += 1;
            }
            if y == yi - 1 {
                defend1 = defend;
                threatened1 = threatened;
            }
        }
        for y in (yi + 1)..8 {
            if board[y * 8 + xi].is_w_rook() || board[y * 8 + xi].is_w_queen() {
                defend1 = y;
            }
            if board[y * 8 + xi].is_b_rook() || board[y * 8 + xi].is_b_queen() {
                threatened = y;
                threatened1 = y;
            }
        }
        let k = (if threatened == 0 && whitethreatened == 0 { 35 } else if threatened == 0 { 20 } else if threatened1 == 0 { 9 } else { 0 })
            + if defend1 != 0 { 5 } else { 0 };
        return weight as i32 * k;
    }

    const WEIGHT_PASSED_RANK: [i32; 7] = [0, 10, 17, 15, 62, 168, 276];
    pub fn passed(board: &[u8; 64], flipped: &[u8; 64], moves: &[Vec<Motion>; 64], flipped_moves: &[Vec<Motion>; 64]) -> i32 {
        let mut value = 0;
        for i in 0..64 {
            if board[i].is_white() && passed_lev(board, flipped, i, moves, flipped_moves) {
                value += WEIGHT_PASSED_RANK[8 - (i / 8) - 1];
                value += passed_block(board, flipped, moves, flipped_moves, i);
                value += WEIGHT_PASSED_FILE * (i % 8).min(7 - (i % 8)) as i32;
            }
        }
        return value;
    }
}
mod imbalance {
    use crate::lib::{
        chessbyte::ChessByte,
        piece::{Parity, PieceByte}
    };

    use super::count_pieces;
    fn get_piece_value_for_ordering(piece: PieceByte) -> i32 {
        match piece {
            PieceByte::NONE => -1,
            PieceByte::PAWN => 0,
            PieceByte::KNIGHT => 1,
            PieceByte::BISHOP => 2,
            PieceByte::ROOK => 3,
            PieceByte::QUEEN => 4,
            PieceByte::KING => 5
        }
    }
    const PRIMARY_IMBALANCE_TABLE: [[i32; 6]; 5] = [ // qo
        [40, 38, 0, 0, 0, 0],
        [32, 255, -62, 0, 0, 0],
        [0, 104, 4, 0, 0, 0],
        [-26, -2, 47, 105, -208, 0],
        [-189, 24, 117, 133, -134, -6]
    ];
    const SECONDARY_IMBALANCE_TABLE: [[i32; 6]; 5] = [ // qt
        [36, 0, 0, 0, 0, 0],
        [9, 63, 0, 0, 0, 0],
        [59, 65, 42, 0, 0, 0],
        [46, 39, 24, -24, 0, 0],
        [97, 100, -42, 137, 268, 0]
    ];
    fn piece_imbalance_table(piece1: PieceByte, piece2: PieceByte, table: &[[i32; 6]; 5]) -> i32 {
        let arrayi = get_piece_value_for_ordering(piece1);
        let array = &table[arrayi as usize];
        let array2_index = get_piece_value_for_ordering(piece2) + 1;
        return array[array2_index as usize];
    }
    pub fn piece_imbalance(board: &[u8; 64]) -> i32 {
        let mut sum = 0;
        for (index, p) in board.iter().enumerate() {
            if p.is_piece() && p.is_white() && !p.is_king() {
                let mut bishops = (0i32, 0i32);
                for x in 0..8 {
                    for y in 0..8 {
                        let i = board[y * 8 + x];
                        if !i.is_piece() || i.is_king() || p.get_piece() == i.get_piece() { continue };
                        if i.is_bishop() {
                            if i.is_black() { bishops.0 += 1 };
                            if i.is_white() { bishops.1 += 1 };
                        }
                        if (get_piece_value_for_ordering(i.get_piece()) + 1) % 6 > (get_piece_value_for_ordering(p.get_piece()) + 1) {
                            continue;
                        }
                        
                        if get_piece_value_for_ordering(i.get_piece()) > 4 {
                            sum += piece_imbalance_table(p.get_piece(), i.get_piece(), &SECONDARY_IMBALANCE_TABLE);
                        } else {
                            sum += piece_imbalance_table(p.get_piece(), i.get_piece(), &PRIMARY_IMBALANCE_TABLE);
                        }
                    }
                }
                if bishops.0 > 1 {
                    sum += SECONDARY_IMBALANCE_TABLE[get_piece_value_for_ordering(p.get_piece()) as usize][0];
                }
                if bishops.1 > 1 {
                    sum += PRIMARY_IMBALANCE_TABLE[get_piece_value_for_ordering(p.get_piece()) as usize][0];
                }
            }
        }
        return sum;
    }

    pub fn bishop_pair(board: &[u8; 64]) -> i32 {
        return if count_pieces(board, PieceByte::BISHOP) < 2 { 0 } else { 1438 };
    }

}
fn phase(board: &[u8; 64]) -> i32 {
  let midlimit = 15258;
  let endlimit = 3915;
  let mut material = material::midgame_material(&board) + material::midgame_material(&board.flipped());
  material = endlimit.max(material.min(midlimit));
  return ((material - endlimit) * 128) / (midlimit - endlimit);
}
fn count_pieces(board: &[u8; 64], piece: PieceByte) -> i32 {
    let mut count = 0;
    for p in board.iter() {
        if p.get_piece() == piece && p.is_parity(Parity::WHITE) {
            count += 1;
        }
    }
    return count;
}
fn opposite_bishops(board: &[u8; 64], flipped: &[u8; 64]) -> bool {
    if count_pieces(board, PieceByte::BISHOP) != 1 || count_pieces(flipped, PieceByte::BISHOP) != 1 {
        return false;
    }
    let mut c1 = 0;
    let mut c2 = 0;
    for x in 0..8 {
        for y in 0..8 {
            if board[y * 8 + x].get_piece() == PieceByte::BISHOP {
                if board[y * 8 + x].is_parity(Parity::WHITE) {
                    c1 = (x + y) % 2;
                } else {
                    c2 = (x + y) % 2;
                }
            }
        }
    }
    return !(c1 == c2)
}
fn piece_count(board: &[u8; 64]) -> i32 {
    let mut count = 0;
    for i in board.iter() {
        if i.is_parity(Parity::WHITE) {
            count += 1;
        }
    }
    return count;
}
fn scale_factor(board: &[u8; 64], endgame: i32) -> i32 {
    let flipped = board.flipped();
    let board_w = if endgame > 0 { &board } else { &flipped };
    let board_b = if endgame > 0 { &flipped } else { &board };
    let mut factor = 64;
    let pawns_w = count_pieces(board_w, PieceByte::PAWN);
    let queens_b = count_pieces(board_b, PieceByte::QUEEN);
    let bishops_w = count_pieces(board_w, PieceByte::BISHOP);
    let knights_b = count_pieces(board_b, PieceByte::KNIGHT);
    let material_w = material::midgame_material(board_w);
    let material_b = material::midgame_material(board_b);
    if pawns_w == 0 && material_w - material_b <= 825 {
        factor = if material_w < 1276 { 0 } else { if material_b <= 825 { 4 } else { 14 }};
    }
    if factor == 64 {
        let ob = opposite_bishops(board_w, board_b);
        if ob && material_w == 825 && material_b == 825 {
            factor = 22;
        } else if ob {
            factor = 22 + 3 * piece_count(board_w);
        }

    }
    return factor;
}
pub mod pieces {
    use crate::lib::{
        boardarray::BoardArray, chessbyte::ChessByte, motion::{Motion, MotionSet}, piece::{Parity, PieceByte}
    };

    use super::{mobility, pawn::is_backwards};

    fn king_ring(board: &[u8; 64], index: usize, full: bool) -> bool {
        if !full {
            if index >= 9 {
                if board[index - 7].is_pawn() && board[index - 7].is_black() && board[index - 9].is_pawn() && board[index - 9].is_black() {
                    return false;
                }
            }
        }
        let xi: i32 = (index % 8) as i32;
        let yi: i32 = (index / 8) as i32;
        for x in -2i32..=2 {
            for y in -2i32..=2 {
                let i = y * 8 + x;
                if i < 0 {
                    continue;
                }
                if index + (i as usize) < 64 {
                    if board[index + i as usize].is_king() && board[index + i as usize].is_black() {
                        if x >= -1 && x <= 1 || xi + x == 0 || xi + x == 7 {
                            if y >= -1 && y <= 1 || yi + y == 0 || yi + y == 7 {
                                return true;
                            }
                        }
                    }

                }
            }
        }
        return false;
    }
    fn count_king_attackers(board: &[u8; 64], index: usize) -> f32 {
        if board[index].is_king() { return 0.0 };
        let xi = index % 8;
        let yi = index / 8;
        if board[index].is_pawn() {
            let mut value = 0.0;
            if xi > 1 && xi < 6 {
                let is = board[yi * 8 + xi - 2].is_pawn() && board[yi * 8 + xi - 2].is_white();
                if king_ring(board, index, true) {
                    value += if is { 0.5 } else { 1.0 };
                }
                let is2 = board[yi * 8 + xi + 2].is_pawn() && board[yi * 8 + xi + 2].is_white();
                if king_ring(board, index, true) {
                    value += if is2 { 0.5 } else { 1.0 };
                }
            }
            return value;
        }


        if board[index].get_piece() == PieceByte::PAWN && board[index].is_white() {
            let mut value = 0f32;
            let x = index % 8;
            if x > 0 && x - 1 <= 7 && index > 8 {
                if king_ring(board, index - 9, true) {
                    value += if board[index - 2].get_piece() == PieceByte::PAWN && board[index - 2].is_white() { 0.5 } else { 1.0 };
                }
            }
            if x > 0 && x + 1 <= 7 && index + 7 >= 0 {
                if king_ring(board, index - 7, true) {
                    value += if board[index + 2].get_piece() == PieceByte::PAWN && board[index - 2].is_white() { 0.5 } else { 1.0 };
                }
            }
            return value;
        }
        for (i, piece) in board.iter().enumerate() {
            if king_ring(board, i, false) {

                return 1.0;
                // if knight_attack(board, i) || bishop_xray_attack(board, i) || rook_xray_attack(board, i) || queen_attack(board, i) {
                    // return 1.0;
                // }
            }
        }
        return 0.0;
    }
    const WEIGHT_MINOR_BEHIND_PAWN: i32 = 18;
    const WEIGHT_BISHOP_PAWNS: i32 = -3;
    const WEIGHT_BISHOP_XRAY_PAWNS: i32 = -4;
    const WEIGHT_ROOK_QUEEN_FILE: i32 = 6;
    const WEIGHT_ROOK_OPEN_FILE: [i32; 3] = [0, 19, 48];
    const WEIGHT_OUTPOST: [i32; 5] = [0, 31, -7, 30, 56];
    const WEIGHT_ROOK_KING_RING: i32 = 16;
    const WEIGHT_BISHOP_KING_RING: i32 = 24;
    const WEIGHT_TRAPPED_ROOK: i32 = -55;
    const WEIGHT_LONG_DIAG_BISHOP: i32 = 45;
    const WEIGHT_WEAK_QUEEN: i32 = -56;
    const WEIGHT_QUEEN_INFILTRATION: i32 = -2;
    const WEIGHT_KING_PROTECTOR_HORSE: i32 = -8;
    const WEIGHT_KING_PROTECTOR_BISHOP: i32 = -6;
    fn mbehind_pawn(board: &[u8; 64], index: usize) -> i32 {
        return if index > 7 && board[index - 8].is_white() && board[index - 8].is_pawn() { WEIGHT_MINOR_BEHIND_PAWN } else { 0 };
    }
    fn pawn_attack(board: &[u8; 64], index: usize) -> i32 {
        let mut value = 0;
        if index + 9 < 64 && index % 8 < 7 && index % 8 > 0 && board[index + 9].get_piece() == PieceByte::PAWN && board[index + 9].is_white() {
            value += 1;
        }
        if index + 7 < 64 && index % 8 > 0 && board[index + 7].get_piece() == PieceByte::PAWN && board[index + 7].is_white() {
            value += 1;
        }
        return value;
    }
    fn bishop_pawns(board: &[u8; 64], index: usize) -> i32 {
        let c = (index % 8 + index / 8) % 2;
        let mut value = 0;
        let mut blocked = 0;
        for x in 0..8 {
            for y in 0..8 {
                if board[y * 8 + x].is_white() && board[y * 8 + x].is_pawn() {
                    if c == (x + y) % 2 {
                        value += 1;
                    }
                    if x > 1 && x < 6 && y != 0 && board[(y - 1) * 8 + x].is_piece() {
                        blocked += 1;
                    }
                }
            }
        }
        let pawnatt = if pawn_attack(board, index) > 0 { 0 } else { 1 };
        return WEIGHT_BISHOP_PAWNS * (value * (blocked + pawnatt));
    }
    fn bishop_xray(board: &[u8; 64], index: usize) -> i32 {
        let mut count = 0;
        let xi = index % 8;
        let yi = index / 8;
        for (i, piece) in board.iter().enumerate() {
            if piece.is_black() && piece.is_pawn() && xi.abs_diff(i % 8) == yi.abs_diff(i / 8) {
                count += 1;
            }
        }
        return WEIGHT_BISHOP_XRAY_PAWNS * count;
    }
    fn rook_queen_file(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        for y in 0..8 {
            if board[y * 8 + xi].is_queen() {
                return WEIGHT_ROOK_QUEEN_FILE;
            }
        }
        return 0;
    }
    fn rook_open_file(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let mut open = 1;
        for y in 0..8 {
            if board[y * 8 + xi].is_pawn() {
                if board[y * 8 + xi].is_white() {
                    return 0;
                }
                open = 0;
            }
        }
        return WEIGHT_ROOK_OPEN_FILE[open + 1];
    }
    fn pawn_attacks_span(board: &[u8; 64], index: usize) -> bool {
        let flipped = board.flipped();
        let yi = index / 8;
        let xi = index % 8;
        for y in 0..yi {
            if xi > 0 && board[y * 8 + xi - 1].is_pawn() && board[y * 8 + xi - 1].is_black() {
                if yi > 0 {
                    let yin = (y + 1) * 8;
                    if y == yi - 1 || (yin < 64 && board[yin + xi - 1].is_pawn() && board[yin + xi - 1].is_white()) {
                        if !is_backwards(&flipped, (7 - y) * 8 + xi - 1) {
                            return true;
                        }
                    }
                }
            }
            if xi < 7 && board[y * 8 + xi + 1].is_pawn() && board[y * 8 + xi + 1].is_black() {
                if yi > 0 {
                    let yin = (y + 1) * 8;
                    if y == yi - 1 || (yin < 64 && board[yin + xi + 1].is_pawn() && board[yin + xi + 1].is_white()) {
                        if !is_backwards(&flipped, (7 - y) * 8 + xi + 1) {
                            return true;
                        }
                    }
                }
            }

        }
        return false;
    }
    fn is_outpost(board: &[u8; 64], index: usize) -> bool {
        let rank = 8 - (index / 8);
        if rank < 4 || rank > 6 {
            return false;
        }
        if index % 8 != 7 {
            if board[index + 9].is_pawn() && board[index + 9].is_white() && !pawn_attacks_span(board, index) {
                return true;
            }
        }
        if index % 8 != 0 {
            if board[index + 7].is_pawn() && board[index + 7].is_white() && !pawn_attacks_span(board, index) {
                return true;
            }
        }
        return false;
    }
    /*
    fn reachable_outpost(board: &[u8; 64], index: usize) -> i32 {
        let mut value = 0;
        for x in 0..8 {
            for y in 2..5 {
                let i = y * 8 + x;
                if is_outpost(board, i) {
                    if board[i].is_knight() && knight_reaches(board, index, i) {

                    }
                }
                if board[i].is_knight() && board[i].is_white() {
                    if 
                }
            }
        }

    }
    */
    fn outpost(board: &[u8; 64], index: usize) -> i32 {
        if !is_outpost(board, index) {
            // if board[index].is_knight() && reachable_outpost(board, index) != 0 {
            //     return WEIGHT_OUTPOST[1]; 
            // }
            return 0;
        }
        let xi = index % 8;
        let yi = index / 8;
        if board[index].is_knight() && (xi < 2 || xi > 5) {
            let mut ea = 0;
            let mut count = 0;
            for x in 0..8 {
                for y in 0..8 {
                    if board[y * 8 + x].is_piece() && !board[y * 8 + x].is_pawn() && board[y * 8 + x].is_black() {
                        if (xi.abs_diff(x) == 2 && yi.abs_diff(y) == 1) || (xi.abs_diff(x) == 1 && yi.abs_diff(y) == 2) {
                            ea = 1;
                        }
                        if (x < 4 && xi < 4) || (x >= 4 && xi >= 4) {
                            count += 1;
                        }
                    }
                }
            }
            if ea == 0 && count <= 1 {
                return WEIGHT_OUTPOST[2];
            }
        }
        return WEIGHT_OUTPOST[if board[index].is_knight() { 4 } else { 3 }];
    }
    fn rook_king_ring(board: &[u8; 64], index: usize) -> i32 {
        if count_king_attackers(board, index) > 0.0 { return 0 };
        let xi = index % 8;
        for y in 0..8 {
            if king_ring(board, y * 8 + xi, false) { return WEIGHT_ROOK_KING_RING };
        }
        return 0;
    }
    fn bishop_king_ring(board: &[u8; 64], index: usize) -> i32 {
        if count_king_attackers(board, index) > 0.0 { return 0 };
        let xi = (index % 8) as i32;
        let yi = (index / 8) as i32;
        for i in 0..4 {
            let x: i32 = if i > 1 { 1 } else { -1 };
            let y: i32 = if i % 2 == 0 { 1 } else { -1 }; 
            for d in 1..8 {
                let sqx = xi + d * x;
                let sqy = yi + d * y;
                let sqi = (sqy * 8 + sqx) as usize;
                if sqi < 64 {
                    if !board[sqi].is_piece() {
                        break;
                    }
                    if king_ring(board, sqi, false) {
                        return WEIGHT_BISHOP_KING_RING;
                    }
                    if board[sqi].is_pawn() {
                        break;
                    }

                }
            }
        }
        return 0;
    }
    fn trapped_rook(board: &[u8; 64], index: usize, castling: u8, moves: &MotionSet) -> i32 {
        if mobility::mobility(board, index, moves) > 3 { return 0 };
        let mut kingx = 0;
        for x in 0..8 {
            for y in 0..8 {
                if board[y * 8 + x].is_king() && board[y * 8 + x].is_white() {
                    kingx = x;
                }
            }
        }
        if (kingx < 4 && index % 8 < 4) || kingx >= 4 && index % 8 >= 4 {
            return WEIGHT_TRAPPED_ROOK * if (castling & 0b0000_1100) != 0 { 1 } else { 2 };
        }
        return 0;
    }
    fn long_diagonal_bishop(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        if xi.abs_diff(yi) != 0 && xi.abs_diff(7 - yi) != 0 { return 0 };
        let mut x = xi;
        let mut y = yi;
        if x.min(7 - x) > 2 { return 0 };
        for _ in x.min(7 - x)..4 {
            if board[y * 8 + x].is_pawn() { return 0 };
            if x < 4 {
                x += 1;
            } else {
                x -= 1;
            }
            if y < 4 {
                y += 1;
            } else {
                y -= 1;
            }
        }
        return WEIGHT_LONG_DIAG_BISHOP;
    }
    fn weak_queen(board: &[u8; 64], index: usize) -> i32 {
        let xi = (index % 8) as i32;
        let yi = (index / 8) as i32;
        for i in 0..8 {
            let add = if i > 3 { 1 } else { 0 };
            let x = ((i + add) % 3) as i32 - 1;
            let y = ((i + add) / 3) as i32 - 1;
            let mut count = 0;
            for d in 1..8 {
                let sqx = xi + d * x;
                let sqy = yi + d * y;
                let sqi = (sqy * 8 + sqx) as usize;
                if sqi < 64 {
                    if board[sqi].is_rook() && board[sqi].is_black() {
                        if (x == 0 || y == 0) && count == 1 { return WEIGHT_WEAK_QUEEN };
                    }
                    if board[sqi].is_bishop() && board[sqi].is_black() {
                        if (x != 0 && y != 0) && count == 1 { return WEIGHT_WEAK_QUEEN };
                    }
                    if !board[sqi].is_piece() {
                        count += 1;
                    }
                }
            }
        }
        return 0;
    }
    fn queen_infiltration(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        if yi > 3 { return 0 };
        if yi == 0 { return WEIGHT_QUEEN_INFILTRATION };
        if xi == 7 && index.checked_sub(7).is_some() {
            if (board[index - 7].is_pawn() && board[index - 7].is_black()) || pawn_attacks_span(board, index) {
                return 0;
            } else {
                return WEIGHT_QUEEN_INFILTRATION;
            }
        }
        if xi == 0 && index.checked_sub(9).is_some() {
            if (board[index - 9].is_pawn() && board[index - 9].is_black()) || pawn_attacks_span(board, index) {
                return 0;
            } else {
                return WEIGHT_QUEEN_INFILTRATION
            }
        }
        if let Some(subbed) = index.checked_sub(7) {
            if (board[subbed].is_pawn() && board[subbed].is_black()) || pawn_attacks_span(board, index) {
                return 0;
            }
        }
        if let Some(subbed) = index.checked_sub(9) {
            if (board[subbed].is_pawn() && board[subbed].is_black()) || pawn_attacks_span(board, index) {
                return 0;
            }
        }
        return WEIGHT_QUEEN_INFILTRATION;
    }
    fn king_protector(board: &[u8; 64], index: usize, is_horse: bool) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        for x in 0..8 {
            for y in 0..8 {
                if board[y * 8 + x].is_king() && board[y * 8 + x].is_white() {
                    return x.abs_diff(xi).max(y.abs_diff(yi)) as i32 * if is_horse { WEIGHT_KING_PROTECTOR_HORSE } else { WEIGHT_KING_PROTECTOR_BISHOP };
                }
            }
        }
        return 0;
    }
    pub fn midgame_pieces(board: &[u8; 64], castling: u8, moves: &MotionSet) -> i32 {
        let mut value = 0;
        for (index, piece) in board.iter().enumerate() {
            if piece.is_white() {
                if piece.is_knight() {
                    /*
                    println!("Knight, in order: {}, {}, {}, {}, {}, {}, {}", 
                        outpost(board, index), 
                        mbehind_pawn(board, index), 
                        rook_queen_file(board, index), 
                        rook_king_ring(board, index),
                        rook_open_file(board, index),
                        trapped_rook(board, index, castling, moves),
                        king_protector(board, index, true)
                    );
                    */
                    value += outpost(board, index);
                    value += mbehind_pawn(board, index);
                    value += rook_queen_file(board, index);
                    value += rook_king_ring(board, index);
                    let open_file = rook_open_file(board, index);
                    value += open_file;
                    if open_file > 0 { value += trapped_rook(board, index, castling, moves) };
                    value += king_protector(board, index, true);

                } else if piece.is_bishop() {
                    /*
                    println!("Bishop, in order: {}, {}, {}, {}, {}, {}, {}", 
                        outpost(board, index), 
                        mbehind_pawn(board, index), 
                        bishop_pawns(board, index),
                        bishop_xray(board, index),
                        bishop_king_ring(board, index),
                        king_protector(board, index, false),
                        long_diagonal_bishop(board, index)
                    );
                    */
                    value += outpost(board, index);
                    value += mbehind_pawn(board, index);
                    value += bishop_pawns(board, index);
                    value += bishop_xray(board, index);
                    value += bishop_king_ring(board, index);
                    value += king_protector(board, index, false);
                    value += long_diagonal_bishop(board, index);

                } else if piece.is_queen() {
                    /*
                    println!("Queen, in order: {}, {}", 
                        weak_queen(board, index),
                        queen_infiltration(board, index)
                    );
                    */
                    value += weak_queen(board, index);
                    value += queen_infiltration(board, index);
                }
            }
        }
        return value;
    }
}
mod mobility {
    use crate::lib::{
        chessbyte::ChessByte, mask::Mask, motion::MotionSet, piece::PieceByte
    };

    const MOBILITY_KNIGHT_WEIGHTS: [i32; 9] =   [-62, -53, -12, -4, 3, 13, 22, 28, 33];
    const MOBILITY_BISHOP_WEIGHTS: [i32; 14] =  [-48, -20, 16, 26, 38, 51, 55, 63, 63, 68, 81, 81, 91, 98];
    const MOBILITY_ROOK_WEIGHTS: [i32; 15] =    [-60, -20, 2, 3, 3, 11, 22, 31, 40, 40, 41, 48, 57, 57, 62];
    const MOBILITY_QUEEN_WEIGHTS: [i32; 28] =   [-30, -12, -8, -9, 20, 23, 23, 35, 38, 53, 64, 65, 65, 66, 
                                                67, 67, 72, 72, 77, 79, 93, 108, 108, 108, 110, 114, 114, 116];

    pub fn bonus(board: &[u8; 64], moves: &MotionSet) -> i32 {
        let mut value = 0;
        for i in 0..64 {
            if board[i].is_piece() && board[i].is_white() {
                value += match board[i].get_piece() {
                    PieceByte::KNIGHT => MOBILITY_KNIGHT_WEIGHTS[mobility(board, i, moves).min(8) as usize],
                    PieceByte::BISHOP => MOBILITY_BISHOP_WEIGHTS[mobility_single(board, i, moves).min(13) as usize],
                    PieceByte::ROOK => MOBILITY_ROOK_WEIGHTS[mobility_single(board, i, moves).min(14) as usize],
                    PieceByte::QUEEN => MOBILITY_QUEEN_WEIGHTS[mobility_single(board, i, moves).min(27) as usize],
                    _ => 0
                };
            }
        }
        return value;
    }
    fn diagonal_surrounding_xrayable_count(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        let mut v: Vec<u8> = Vec::new();
        if yi > 0 {
            if xi > 0 { v.push(board[index - 9]) };
            if xi < 7 { v.push(board[index - 7]) };
        }
        if yi < 7 {
            if xi > 0 { v.push(board[index + 7]) };
            if xi < 7 { v.push(board[index + 9]) };
        }
        let mut count = 0;
        for byte in v {
            if !byte.is_piece() || byte.is_black() || !(byte.is_w_pawn() || byte.is_w_king()){
                count += 1;
            }
        }
        return count;
    }
    fn cardinal_surrounding_xrayable_count(board: &[u8; 64], index: usize) -> i32 {
        let xi = index % 8;
        let yi = index / 8;
        let mut v: Vec<u8> = Vec::new();
        if yi > 0 { v.push(board[(yi - 1) * 8 + xi]) };
        if yi < 7 { v.push(board[(yi + 1) * 8 + xi]) };
        if xi > 0 { v.push(board[index - 1]) };
        if xi < 7 { v.push(board[index + 1]) };
        let mut count = 0;
        for byte in v {
            if !byte.is_piece() || byte.is_black() {
                count += 1;
            } else if byte.is_piece() && byte.is_white() {
                if !byte.is_king() && !byte.is_pawn() {
                    count += 1;
                }
            }
        }
        return count;
    }
    pub fn mobility_single(board: &[u8; 64], index: usize, moves: &MotionSet) -> i32 {
        if board[index].is_king() || board[index].is_pawn() {
            return 0;
        }
        let mut value = mobility(board, index, moves);
        if (board[index].is_rook() || board[index].is_queen()) && cardinal_surrounding_xrayable_count(board, index) > 0 {
            value += 1;
        }
        if (board[index].is_bishop() || board[index].is_queen()) && diagonal_surrounding_xrayable_count(board, index) > 0 {
            value += 1;
        }
        return value;
    }

    pub fn mobility(board: &[u8; 64], index: usize, moves: &MotionSet) -> i32 {
        let mut value = 0;
        if !board[index].is_piece() || board[index].is_black() || board[index].is_king() || board[index].is_pawn() {
            return 0;
        }
        for x in 0..8 {
            for y in 0..8 {
                if mobility_area(board, y * 8 + x) == 0 {
                    continue;
                }
                if (board[index].is_knight() || board[index].is_bishop()) && !board[y * 8 + x].is_w_queen() {
                    if (moves.white_piecewise_flat[index] & Mask::from_index(y * 8 + x)).any() {
                        value += 1;
                    }
                } else if board[index].is_rook() || board[index].is_queen() {
                    if (moves.white_piecewise_flat[index] & Mask::from_index(y * 8 + x)).any() {
                        value += 1;
                    }
                }

            }
        }
        return value;
    }

    pub fn pinned_direction(board: &[u8; 64], index: usize) -> i32 {
        let color = if board[index].is_white() { 1 } else { -1 };
        let xi = (index % 8) as i32;
        let yi = (index / 8) as i32;
        for i in 0..8 {
            let add = if i > 3 { 1 } else { 0 };
            let x = ((i + add) % 3) as i32 - 1;
            let y = ((i + add) / 3) as i32 - 1;
            let mut king = false;
            for d in 1..8 {
                let sqx = xi + d * x;
                let sqy = yi + d * y;
                let sqi = (sqy * 8 + sqx) as usize;
                if sqi < 64 {
                    if board[sqi].is_king() && board[sqi].is_white() {
                        king = true;
                    }
                    if board[sqi].is_piece() {
                        break;
                    }
                }
            }
            if king {
                for d in 1..8 {
                    let sqx = xi + d * x;
                    let sqy = yi + d * y;
                    let sqi = (sqy * 8 + sqx) as usize;
                    if sqi < 64 {
                        if board[sqi].is_black() {
                            if board[sqi].is_queen() || (board[sqi].is_bishop() && x * y != 0) || (board[sqi].is_rook() && x * y == 0 ) {
                                return (x + y * 3) as i32 * color;
                            }
                        }
                        if board[sqi].is_piece() {
                            break;
                        }
                    }
                }
            }

        }
        return 0;
    }
    pub fn mobility_area(board: &[u8; 64], index: usize) -> i32 {
        if (board[index].is_king() || board[index].is_queen()) && board[index].is_white() { return 0 };
        if let Some(subbed) = index.checked_sub(9) {
            if board[subbed].is_pawn() && board[subbed].is_black() { return 0 };
        }
        if let Some(subbed) = index.checked_sub(7) {
            if board[subbed].is_pawn() && board[subbed].is_black() { return 0 };
        }
        if board[index].is_pawn() && board[index].is_white() {
            let rank = 8 - (index / 8);
            if rank < 4 { return 0 };
            if let Some(subbed) = index.checked_sub(8) {
                if !board[subbed].is_piece() { return 0 };
            }
        }
        if pinned_direction(board, index) != 0 { return 0 };
        return 1;
    }

}
mod pawn {
    use crate::lib::{
        chessbyte::ChessByte,
        piece::PieceByte
    };


    fn is_isolated(board: &[u8; 64], index: usize) -> bool {
        if !board[index].is_white() || board[index].get_piece() != PieceByte::PAWN {
            return false;
        }
        for y in 0..8 {
            if y * 8 + index - 1 < 64 && board[y * 8 + index - 1].get_piece() == PieceByte::PAWN && board[y * 8 + index - 1].is_white() {
                return false;
            }
            if y * 8 + index + 1 < 64 && board[y * 8 + index + 1].get_piece() == PieceByte::PAWN && board[y * 8 + index + 1].is_white() {
                return false;
            }
        }
        return true;
    }
    pub fn is_backwards(board: &[u8; 64], index: usize) -> bool {
        if !board[index].is_white() || board[index].get_piece() != PieceByte::PAWN {
            return false;
        }
        for y in (index / 8)..8 {
            if y * 8 + index % 8 + 1 > 63 {
                continue;
            }
            if (board[y * 8 + index % 8 - 1].get_piece() == PieceByte::PAWN && board[y * 8 + index % 8 - 1].is_white()) || ((board[y * 8 - index % 8 + 1].get_piece() == PieceByte::PAWN && board[y * 8 + index % 8 + 1].is_white())){
                return false;
            }
        }
        if index > 17 {
            if (board[index - 17].is_black() && board[index - 17].get_piece() == PieceByte::PAWN)
                || (board[index - 15].is_black() && board[index - 15].get_piece() == PieceByte::PAWN)
                || (board[index - 8].is_black() && board[index - 8].get_piece() == PieceByte::PAWN) {
                    return true;
            }
        }
        return false;
    }
    fn is_doubled(board: &[u8; 64], index: usize) -> bool {
        if index < 8 {
            return false;
        } 
        if board[index - 8].is_white() && board[index - 8].get_piece() == PieceByte::PAWN {
            return true;
        }
        if index > 55 {
            return false;
        }
        if board[index + 8].is_white() && board[index + 8].get_piece() == PieceByte::PAWN {
            return true;
        }
        return false;
    }
    fn supported(board: &[u8; 64], index: usize) -> i32 {
        let mut v = 0;
        if index + 9 < 64 && index % 8 < 7 && index % 8 > 0 && board[index + 9].get_piece() == PieceByte::PAWN && board[index + 9].is_white() {
            v += 1;
        }
        if index + 7 < 64 && index % 8 > 0 && board[index + 7].get_piece() == PieceByte::PAWN && board[index + 7].is_white() {
            v += 1;
        }
        return v;
    }
    fn is_phalanx(board: &[u8; 64], index: usize) -> bool {
        let mut v = 0;
        if index + 1 < 64 && board[index + 1].get_piece() == PieceByte::PAWN && board[index + 1].is_white() {
            v += 1;
        }
        if index >= 1 && board[index - 1].get_piece() == PieceByte::PAWN && board[index - 1].is_white() {
            v += 1;
        }
        return v != 0;
    }
    fn is_opposed(board: &[u8; 64], index: usize) -> bool {
        for y in 0..(index / 8) {
            if board[y * 8 + (index % 8)].is_black() && board[y * 8 + (index % 8)].get_piece() == PieceByte::PAWN {
                return true;
            }
        }
        return false;
    }
    fn is_connected(board: &[u8; 64], index: usize) -> bool {
        return supported(board, index) > 0 || is_phalanx(board, index);
    }
    const CONNECTED_SEED: [i32; 7] = [0, 7, 8, 12, 29, 48, 86];
    fn connected(board: &[u8; 64], index: usize) -> i32 {
        let rank = 8 - (index / 8);
        if rank < 2 || rank > 7 || !is_connected(board, index) { return 0 };
        let opposed = if is_opposed(board, index) { 1 } else { 0 };
        let phalanx = if is_phalanx(board, index) { 1 } else { 0 };
        let supported = supported(board, index);
        return CONNECTED_SEED[rank - 1] * (2 + phalanx - opposed) + 21 * supported;

    }
    fn unopposed(board: &[u8; 64], index: usize) -> bool {
        if is_opposed(board, index) {
            return false;
        }
        return is_isolated(board, index) || is_backwards(board, index);
    }
    fn blocked(board: &[u8; 64], index: usize) -> i32 {
        let y = index / 8;
        if y != 2 && y != 3 { return 0 };
        if !board[index - 8].is_pawn() || board[index - 8].is_white() { return 0 };
        return 4 - y as i32;
    }
    pub fn is_double_isolated(board: &[u8; 64], index: usize) -> bool {
        if is_isolated(board, index) {
            let indexy = index / 8;
            let indexx = index % 8;
            let mut has_opposed = false;
            let mut has_doubled = false;
            for y in 0..8 {
                if y < indexy && board[y * 8 + indexx].is_pawn() && board[y * 8 + indexx].is_black() {
                    has_opposed = true;
                }
                if y > indexy && board[y * 8 + indexx].is_pawn() && board[y * 8 + indexx].is_white() {
                    has_doubled = true;
                }
                if indexx != 0 {
                    if board[y * 8 + indexx - 1].is_pawn() && board[y * 8 + indexx - 1].is_black() {
                        return false;
                    }
                }
                if indexx != 7 {
                    if board[y * 8 + indexx + 1].is_pawn() && board[y * 8 + indexx + 1].is_black() {
                        return false;
                    }
                }
            }
            return has_doubled && has_opposed;
        }
        return false;
    }

    pub fn midgame_pawn(board: &[u8; 64]) -> i32 {
        let mut value = 0;
        for (index, piece) in board.iter().enumerate() {
            if piece.is_white() && piece.is_pawn(){
                if is_double_isolated(board, index) {
                    value -= 11;
                } else if is_isolated(board, index) {
                    value -= 5;
                } else if is_backwards(board, index) {
                    value -= 9;
                }
                if is_doubled(board, index) {
                    value -= 11;
                }
                value += connected(board, index);
                value -= 13 * if unopposed(board, index) { 1 } else { 0 };
                value += match blocked(board, index) {
                    1 => -11,
                    2 => -3,
                    _ => 0
                };
            }
        }
        return value;
    }
}
pub mod material {
    use crate::lib::{
        chessbyte::ChessByte,
        piece::{Parity, PieceByte}
    };



    pub const EG_PAWN_WEIGHTS: [[i32; 8]; 8] = [[0, 0, 0, 0, 0, 0, 0, 0],[-10, -6, 10, 0, 14, 7, -5, -19],[-10, -10, -10, 4, 4, 3, -6, -4],[6, -2, -8, -4, -13, -12, -10, -9],[10, 5, 4, -5, -5, -5, 14, 9], [28, 20, 21, 28, 30, 7, 6, 13],[0, -11, 12, 21, 25, 19, 4, 7],[0, 0, 0, 0, 0, 0, 0, 0]];
    pub const EG_KNIGHT_WEIGHTS: [[i32; 4]; 8] = [[-96, -65, -49, -21],[-67, -54, -18, 8],[-40, -27, -8, 29],[-35, -2, 13, 28],[-45, -16, 9, 39],[-51, -44, -16, 17],[-69, -50, -51, 12],[-100, -88, -56, -17]];
    pub const EG_BISHOP_WEIGHTS: [[i32; 4]; 8]  = [[-57, -30, -37, -12],[-37, -13, -17, 1],[-16, -1, -2, 10],[-20, -6, 0, 17],[-17, -1, -14, 15],[-30, 6, 4, 6],[-31, -20, -1, 1],[-46, -42, -37, -24]];
    pub const EG_ROOK_WEIGHTS: [[i32; 4]; 8]  = [[-9, -13, -10, -9],[-12, -9, -1, -2],[6, -8, -2, -6],[-6, 1, -9, 7],[-5, 8, 7, -6],[6, 1, -7, 10],[4, 5, 20, -5],[18, 0, 19, 13]];
    pub const EG_QUEEN_WEIGHTS: [[i32; 4]; 8]  = [[-69, -57, -47, -26],[-55, -31, -22, -4],[-39, -18, -9, 3],[-23, -3, 13, 24],[-29, -6, 9, 21],[-38, -18, -12, 1],[-50, -27, -24, -8],[-75, -52, -43, -36]];
    pub const EG_KING_WEIGHTS: [[i32; 4]; 8]  = [[1, 45, 85, 76],[53, 100, 133, 135],[88, 130, 169, 175],[103, 156, 172, 172],[96, 166, 199, 199],[92, 172, 184, 191],[47, 121, 116, 131],[11, 59, 73, 78]];

    pub const MG_PAWN_WEIGHTS: [[i32; 8]; 8] = [[0, 0, 0, 0, 0, 0, 0, 0],[3, 3, 10, 19, 16, 19, 7, -5],[-9, -15, 11, 15, 32, 22, 5, -22],[-4, -23, 6, 20, 40, 17, 4, -8],[13, 0, -13, 1, 11, -2, -13, 5], [5, -12, -7, 22, -8, -5, -15, -8],[-7, 7, -3, -13, 5, -16, 10, -8],[0, 0, 0, 0, 0, 0, 0, 0]];
    pub const MG_KNIGHT_WEIGHTS: [[i32; 4]; 8] = [ [-175, -92, -74, -73],[-77, -41, -27, -15],[-61, -17, 6, 12],[-35, 8, 40, 49],[-34, 13, 44, 51],[-9, 22, 58, 53],[-67, -27, 4, 37],[-201, -83, -56, -26], ];
    pub const MG_BISHOP_WEIGHTS: [[i32; 4]; 8] = [ [-53, -5, -8, -23],[-15, 8, 19, 4],[-7, 21, -5, 17],[-5, 11, 25, 39],[-12, 29, 22, 31],[-16, 6, 1, 11],[-17, -14, 5, 0],[-48, 1, -14, -23], ];
    pub const MG_ROOK_WEIGHTS: [[i32; 4]; 8] = [ [-31, -20, -14, -5],[-21, -13, -8, 6],[-25, -11, -1, 3],[-13, -5, -4, -6],[-27, -15, -4, 3],[-22, -2, 6, 12],[-2, 12, 16, 18],[-17, -19, -1, 9], ];
    pub const MG_QUEEN_WEIGHTS: [[i32; 4]; 8] = [ [3, -5, -5, 4],[-3, 5, 8, 12],[-3, 6, 13, 7],[4, 5, 9, 8],[0, 14, 12, 5],[-4, 10, 6, 8],[-5, 6, 10, 8],[-2, -2, 1, -2] ];
    pub const MG_KING_WEIGHTS: [[i32; 4]; 8] = [ [271, 327, 271, 198],[278, 303, 234, 179],[195, 258, 169, 120],[164, 190, 138, 98],[154, 179, 105, 70],[123, 145, 81, 31],[88, 120, 65, 33],[59, 89, 45, -1] ];

    pub const MIDGAME_PRICE: [i32; 5] = [124, 781, 825, 1276, 2538];
    pub const ENDGAME_PRICE: [i32; 5] = [206, 854, 915, 1380, 2682];
    fn material(board: &[u8; 64], price: &[i32; 5], include_pawns: bool) -> i32 {
        let mut total = 0;
        for p in board.iter() {
            if p.is_white() {
                total += match p.get_piece() {
                    PieceByte::PAWN => if include_pawns { price[0] } else { 0 },
                    PieceByte::KNIGHT => price[1],
                    PieceByte::BISHOP => price[2],
                    PieceByte::ROOK => price[3],
                    PieceByte::QUEEN => price[4],
                    _ => 0
                };
            }
        }
        return total;
    }

    pub fn midgame_material(board: &[u8; 64]) -> i32 {
        return material(board, &MIDGAME_PRICE, true);
    }

    pub fn endgame_material(board: &[u8; 64]) -> i32 {
        return material(board, &ENDGAME_PRICE, true);
    }

    pub fn material_value_of_index(board: &[u8; 64], index: usize) -> i32 {
        return match board[index].get_piece() {
            PieceByte::PAWN => MIDGAME_PRICE[0],
            PieceByte::KNIGHT => MIDGAME_PRICE[1],
            PieceByte::BISHOP => MIDGAME_PRICE[2],
            PieceByte::ROOK => MIDGAME_PRICE[3],
            PieceByte::QUEEN => MIDGAME_PRICE[4],
            PieceByte::KING => 30000,
            PieceByte::NONE => 0
        };
    }

    pub fn midgame_weighted_position(board: &[u8; 64]) -> i32 {
        let mut total = 0;
        for y in 0..8 {
            for x in 0..8 {
                if board[y * 8 + x].is_white() {
                    total += match board[y * 8 + x].get_piece() {
                        PieceByte::KING => MG_KING_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::QUEEN => MG_QUEEN_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::KNIGHT => MG_KNIGHT_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::BISHOP => MG_BISHOP_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::ROOK => MG_ROOK_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::PAWN => MG_PAWN_WEIGHTS[7 - y][x],
                        PieceByte::NONE => 0
                    };
                }
            }
        }
        return total;
    }
    pub fn get_visual_material_weights(piece: u8) -> [i32; 64] {
        let mut array = [0i32; 64];
        for y in 0..8 {
            for x in 0..8 {
                array[y * 8 + x] = match piece.get_piece() {
                    PieceByte::KING => MG_KING_WEIGHTS[7 - y][x.min(7 - x)],
                    PieceByte::QUEEN => MG_QUEEN_WEIGHTS[7 - y][x.min(7 - x)],
                    PieceByte::KNIGHT => MG_KNIGHT_WEIGHTS[7 - y][x.min(7 - x)],
                    PieceByte::BISHOP => MG_BISHOP_WEIGHTS[7 - y][x.min(7 - x)],
                    PieceByte::ROOK => MG_ROOK_WEIGHTS[7 - y][x.min(7 - x)],
                    PieceByte::PAWN => MG_PAWN_WEIGHTS[7 - y][x],
                    PieceByte::NONE => 0
                };
            }
        }


        if piece.is_parity(Parity::BLACK) {
            array.reverse();
            for i in 0..8 {
                for k in 0..4 {
                    array.swap(i * 8 + k, i * 8 + 7 - k);
                }
            }
            return array;
        }
        return array;
    }
    pub fn endgame_weighted_position(board: &[u8; 64]) -> i32 {
        let mut total = 0;
        for y in 0..8 {
            for x in 0..8 {
                if board[y * 8 + x].is_white() {
                    total += match board[y * 8 + x].get_piece() {
                        PieceByte::KING => EG_KING_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::QUEEN => EG_QUEEN_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::KNIGHT => EG_KNIGHT_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::BISHOP => EG_BISHOP_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::ROOK => EG_ROOK_WEIGHTS[7 - y][x.min(7 - x)],
                        PieceByte::PAWN => EG_PAWN_WEIGHTS[7 - y][x],
                        PieceByte::NONE => 0
                    };
                }
            }
        }
        return total;

    }

}
