use crate::lib::{
    boardarray::BoardArray, chessbyte::ChessByte, piece::{Parity, PieceByte}, state::State
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
        println!("King mismatch W: {}. B: {}", state.info.king_indices[0], state.info.king_indices[1]);
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
    evaluator.push("Mobility", mobility::bonus(&state.board, &state.moves), -mobility::bonus(&flipped.board, &fmoves));
    if DO_TIMING { println!("Mobility {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Threats", threats::threats(&state.board, &state.moves, &state.info.maskset, state.info.king_indices[0]), -threats::threats(&flipped.board, &fmoves, &flipped.maskset, flipped.king_indices[0]));
    if DO_TIMING { println!("Threats {:.2?}", ev.elapsed()) };

    ev = std::time::Instant::now();
    evaluator.push("Passed", general::passed(&state.board, &state.moves, &state.info.maskset), -general::passed(&flipped.board, &fmoves, &flipped.maskset));
    if DO_TIMING { println!("Passed {:.2?}", ev.elapsed()) };
    ev = std::time::Instant::now();
    if state.turn == Parity::WHITE {
        evaluator.push("Tempo", general::tempo(state.turn), 0);
    } else {
        evaluator.push("Tempo", 0, general::tempo(state.turn));
    }
    if DO_TIMING { println!("Tempo {:.2?}", ev.elapsed()) };
    evaluator.finalize(state.info.halfmove_clock);
    let mut zbrist2 = state.zobrist.lock().unwrap();
    zbrist2.save((state.info.clone(), state.moves.clone(), Some(evaluator.clone())));
    drop(zbrist2);
    return evaluator;

}
mod threats {

    use crate::lib::{ chessbyte::ChessByte, mask::{Mask, ValueMask}, maskset::MaskSet, motion::MotionSet, piece::PieceByte };

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
    fn restricted_threat(moves: &MotionSet) -> u32 {
        return (moves.white_flat & (moves.black_defensive_flat | moves.black_flat)).bit_count();
    }
    pub fn threat_values(moves: &MotionSet, maskset: &MaskSet) -> ValueMask {
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
    fn king_threat(moves: &MotionSet, maskset: &MaskSet, king: usize) -> u32 {
        return (moves.white_piecewise_flat[king] & maskset.black & moves.black_defensive_flat.get_not()).bit_count();
    }
    fn queen_knight_threat(board: &[u8; 64], moves: &MotionSet, queen: usize) -> u32 {
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
    fn queen_slide_threat(moves: &MotionSet, queen: usize) -> u32 {
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
            queen_slider += WEIGHT_QUEEN_SLIDER * queen_slide_threat(moves, bq) as i32;
            queen_knight += WEIGHT_QUEEN_KNIGHT * queen_knight_threat(board, moves, bq) as i32;
            queen_weak_protection += WEIGHT_WEAK_QUEEN_PROTECTION * weak_queen_protection(board, moves, maskset, bq) as i32;
        }
        let minor = minor_threat(board, moves, maskset, &threat_vals);
        let rook = rook_threat(board, moves, maskset, &threat_vals);
        return restricted + hanging + pawn_push + safe_pawn + king_threat + queen_slider + queen_knight + queen_weak_protection + minor + rook;
    }

}
mod general {
    use crate::lib::{
        chessbyte::ChessByte, mask::{Mask, ValueMask}, maskset::MaskSet, motion::MotionSet, piece::Parity
    };

    use super::threats::threat_values;
    const WEIGHT_TEMPO: i32 = 28;
    pub fn tempo(parity: Parity) -> i32 { WEIGHT_TEMPO * if parity == Parity::WHITE { 1 } else { -1 } }

    const WEIGHT_PASSED_RANK: [i32; 7] = [0, 10, 17, 15, 62, 168, 276];
    const WEIGHT_COMPOUNDING_PASSED: [i32; 5] = [2, 7, 12, 17, 22];
    const WEIGHT_COMPOUNDING_MULT: i32 = 70;
    const WEIGHT_PASSED_FILE: i32 = -11;
    fn passed_candidate(board: &[u8; 64], maskset: &MaskSet, tvals: &ValueMask, index: usize) -> bool {
        let yi = index / 8;
        let xi = index % 8;
        let im = Mask::from_index(index);
        let col = Mask::of_column(xi);
        for iso in &(maskset.white & Mask::all_rows_above(yi) & col).isolated_bits() {
            let isoi = iso.as_index();
            if board[isoi].is_pawn() {
                return false;
            }
        }
        let (sidel, sider) = im.get_sides();
        let mut bpc = 0i32;
        for iso in &(maskset.black & Mask::all_rows_above(yi) & (col.get_sides().0 | col | col.get_sides().1)).isolated_bits() {
            let isoi = iso.as_index();
            if board[isoi].is_b_pawn() {
                bpc += 1;
            }
        }
        if bpc > 2 {
            return false;
        }
        if sidel.any() && board[sidel].is_w_pawn() {
            bpc -= 1;
        }
        if sider.any() && board[sider].is_w_pawn() {
            bpc -= 1;
        }
        if bpc > 0 {
            return false;
        }
        if (tvals.to_mask(2) & col).any() {
            if sidel.any() && board[sidel].is_w_pawn() {
                return true;
            }
            if sider.any() && board[sider].is_w_pawn() {
                return true;
            }
        }
        return true;
    }
    fn compounding_weight(tvals: &ValueMask, index: usize) -> i32 {
        if index < 8 || index > 39 { return 0 };
        let mut im = Mask::from_index(index).get_above();
        let tv = tvals.to_mask_ge0();
        let mut value = 0;
        while im.any() {
            if (tv & im).any() {
                break;
            }
            value += WEIGHT_COMPOUNDING_PASSED[8 - (im.as_index() / 8) - 4];
            im = im.get_above();
        }
        return value;
    }
    pub fn passed(board: &[u8; 64], moves: &MotionSet, maskset: &MaskSet) -> i32 {
        let mut rank_val = 0;
        let mut compound_val = 0;
        let mut file_val = 0;
        let tvals = threat_values(moves, maskset);
        for i in 0..64 {
            if board[i].is_w_pawn() && passed_candidate(board, maskset, &tvals, i){
                let rank = 8 - (i / 8);
                let file = 1 + (i % 8);
                println!("{}", (file - 1).min(8 - file) as i32);
                rank_val += WEIGHT_PASSED_RANK[rank - 1];
                compound_val += WEIGHT_COMPOUNDING_MULT * compounding_weight(&tvals, i);
                file_val += WEIGHT_PASSED_FILE * (file - 1).min(8 - file) as i32;
            }
        }
        return rank_val + compound_val + file_val;
    }
}
mod imbalance {
    use crate::lib::{
        chessbyte::ChessByte,
        piece::PieceByte
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
        for p in board.iter() {
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
fn count_pieces(board: &[u8; 64], piece: PieceByte) -> i32 {
    let mut count = 0;
    for p in board.iter() {
        if p.get_piece() == piece && p.is_parity(Parity::WHITE) {
            count += 1;
        }
    }
    return count;
}
pub mod pieces {
    use crate::lib::{
        boardarray::BoardArray, chessbyte::ChessByte, motion::MotionSet, piece::PieceByte
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
            if x > 0 && x + 1 <= 7 {
                if king_ring(board, index - 7, true) {
                    value += if board[index + 2].get_piece() == PieceByte::PAWN && board[index - 2].is_white() { 0.5 } else { 1.0 };
                }
            }
            return value;
        }
        for i in 0..64 {
            if king_ring(board, i, false) {
                return 1.0;
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
    fn outpost(board: &[u8; 64], index: usize) -> i32 {
        if !is_outpost(board, index) {
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
                    value += outpost(board, index);
                    value += mbehind_pawn(board, index);
                    value += rook_queen_file(board, index);
                    value += rook_king_ring(board, index);
                    let open_file = rook_open_file(board, index);
                    value += open_file;
                    if open_file > 0 { value += trapped_rook(board, index, castling, moves) };
                    value += king_protector(board, index, true);
                } else if piece.is_bishop() {
                    value += outpost(board, index);
                    value += mbehind_pawn(board, index);
                    value += bishop_pawns(board, index);
                    value += bishop_xray(board, index);
                    value += bishop_king_ring(board, index);
                    value += king_protector(board, index, false);
                    value += long_diagonal_bishop(board, index);

                } else if piece.is_queen() {
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
    pub fn price_piece(piece: u8) -> i32 {
        match piece.get_piece() {
            PieceByte::PAWN => MIDGAME_PRICE[0],
            PieceByte::KNIGHT => MIDGAME_PRICE[1],
            PieceByte::BISHOP => MIDGAME_PRICE[2],
            PieceByte::ROOK => MIDGAME_PRICE[3],
            PieceByte::QUEEN => MIDGAME_PRICE[4],
            _ => 0
        }
    }
    pub fn price_parity(board: &[u8; 64], parity: Parity) -> i32 {
        let mut sum = 0;
        for i in 0..64 {
            if board[i].is_parity(parity) {
                sum += price_piece(board[i]);
            }
        }
        return sum;
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
