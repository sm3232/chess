use std::cell::RefCell;
use std::rc::Rc;

use chess::lib::chessbyte::ChessByte;
use chess::lib::eval;
use chess::lib::heap::{EvaluatedMotion, Heap};
use chess::lib::piece::Parity;
use chess::lib::player::Player;
use chess::lib::searchtree::SearchTree;
use chess::lib::state::State;
use chess::lib::ui::ChessApp;
use chess::lib::mask::Mask;
use chess::lib::piece::PieceByte;
use eframe::egui;

const FENS: [&str; 7] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // Default
    "rnbqkbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1", // No Pawns
    "r/8/8/8/8/8/8/R w KQkq - 0 1", // Just rooks,
    "r4k/8/8/8/8/8/8/R4K w KQkq - 0 1", // Check,
    "r4k/8/8/8/8/8/8/RR4K w KQkq - 0 1", // Check,
    "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", //Castling 
    "rnbqkbnr/pppppppp/8/8/8/8/P7/K7 w KQkq - 0 1", // White only pawns
    
];

const WINDOW_SIZE: [f32; 2] = [1800.0, 800.0];
const PLAYING_AREA: f32 = 600.0;



fn main() -> () {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(WINDOW_SIZE).with_position([0.0, 0.0]),
        ..Default::default()
    };
    let finish = eframe::run_native(
        "Chess",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ChessApp::new(
                        cc,
                        PLAYING_AREA,
                        WINDOW_SIZE[0] - PLAYING_AREA,
                        FENS[0].to_string()
            )))
        }),
    );

    if let Err(e) = finish {
        dbg!("App exited with error: {:?}", e);
    } else {
        dbg!("Shut down gracefully");
    }
}
