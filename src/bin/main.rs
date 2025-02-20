use std::{panic, process::{self}};

use chess::lib::ui::ChessApp;
use eframe::egui;

const FENS: [&str; 12] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // Default
    "rnbqkbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1", // No Pawns
    "r/8/8/8/8/8/8/R w KQkq - 0 1", // Just rooks,
    "r4k/8/8/8/8/8/8/R4K w KQkq - 0 1", // Check,
    "r4k/8/8/8/8/8/8/RR4K w KQkq - 0 1", // Check,
    "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", //Castling 
    "rnbqkbnr/pppppppp/8/8/8/8/P7/K7 w KQkq - 0 1", // White only pawns
    "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e3 0 1",
    "rnbqkbnr/pppppppp/8/4B3/8/8/PPPPPPPP/RN1QKBNR w KQkq - 0 1",
    "rnbqkbnr/pppppppp/2Q5/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 1 1",
    "rnbqk2Q/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1",
    "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq b6 0 2"
    
];

const WINDOW_SIZE: [f32; 2] = [1800.0, 600.0];
const PLAYING_AREA: f32 = 600.0;

fn main() -> () {
    let oh = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        oh(panic_info);
        process::exit(1);
    }));
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
    process::exit(0);
}
