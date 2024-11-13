use eframe::egui::{self, Painter};
use crate::cutil::game;
use crate::cutil::point::point;



pub const BOARD_SIZE: i32 = 8;
const WVAL: u8 = 128;
const BVAL: u8 = 88;
pub const BOARD_W_COLOR: egui::Color32 = egui::Color32::from_rgb(WVAL, WVAL, WVAL);
pub const BOARD_B_COLOR: egui::Color32 = egui::Color32::from_rgb(BVAL, BVAL, BVAL);

pub fn draw_board (painter: &Painter) -> f32 {
    let sqmin = painter.clip_rect().width().min(painter.clip_rect().height());
    let sqsize: f32 = sqmin / (BOARD_SIZE as f32);

    for i in 0..BOARD_SIZE {
        for k in 0..BOARD_SIZE {
            painter.rect(
                egui::Rect { min: egui::Pos2 { 
                    x: ((k as f32) * sqsize), 
                    y: ((i as f32)) * sqsize
                }, max: egui::Pos2 { 
                    x: ((1 + k) as f32 * sqsize), 
                    y: ((1 + i) as f32 * sqsize)
                }}, 
                egui::Rounding::default(), 
                if (k % 2) - (i % 2) == 0 { BOARD_W_COLOR } else { BOARD_B_COLOR }, 
                egui::Stroke::NONE
            );
        }
    }
    return sqsize;
}
pub fn draw_pieces (game: &game::ChessGame, ui: &mut egui::Ui, sqsize: f32) -> () {
    for piece in game.pvec.iter() {
        piece.borrow().draw(ui, egui::Rect { 
            min: (piece.borrow().get_props().pos * sqsize).into(), 
            max: (point(piece.borrow().get_props().pos.x + 1, piece.borrow().get_props().pos.y + 1) * sqsize).into()
        });
    }
}
pub fn draw_debug_info(game: &mut game::ChessGame, painter: &Painter, sqsize: f32) -> () {
    match &game.selected {
        Some(piece) => {
            painter.debug_rect(egui::Rect {
                min: (piece.borrow().get_props().pos * sqsize).into(), 
                max: ((piece.borrow().get_props().pos + point(1, 1)) * sqsize).into(),

            }, egui::Color32::RED, "SELECTED");
        },
        None => ()
    }
    match &game.selected_moves {
        Some(moves) => {
            for &m in moves.to_point_vector().iter() {
                painter.debug_rect(egui::Rect {
                    min: (m * sqsize).into(),
                    max: ((m + point(1, 1)) * sqsize).into(),
                }, egui::Color32::GREEN, "MOVE");
            }
        },
        None => ()
    }
    match game.enpassant {
        Some(enpassant) => {
            painter.debug_rect(egui::Rect {
                min: (enpassant * sqsize).into(),
                max: ((enpassant + point(1, 1)) * sqsize).into(),
            }, egui::Color32::YELLOW, "ENPASSANT\nSQUARE");
        },
        None => ()
    }
    match &game.enpassant_piece {
        Some(enpassant_piece) => {
            painter.debug_rect(egui::Rect {
                min: (enpassant_piece.borrow().get_props().pos * sqsize).into(),
                max: ((enpassant_piece.borrow().get_props().pos + point(1, 1)) * sqsize).into(),
            }, egui::Color32::BLUE, "ENPASSANT\nTARGET");
        },
        None => ()
    }
    for &ts in game.threatened_squares.to_point_vector().iter() {
        painter.debug_rect(egui::Rect {
            min: (ts * sqsize).into(),
            max: ((ts + point(1, 1)) * sqsize).into(),
        }, egui::Color32::LIGHT_RED, "THREAT");
    }
}

pub fn draw_all(game: &mut game::ChessGame, ui: &mut egui::Ui, bg_painter: &Painter, dbg_painter: &Painter) -> () {
    let sqsize = draw_board(bg_painter);
    draw_pieces(game, ui, sqsize);
    draw_debug_info(game, dbg_painter, sqsize);

}
