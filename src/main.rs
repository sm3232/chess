use eframe::egui;
use final_1::shared::piece::Parity;
use final_1::ChessApp;
use final_1::player::Player;
use final_1::shared::mask::Mask;

const FENS: [&str; 5] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // Default
    "rnbqkbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1", // No Pawns
    "r/8/8/8/8/8/8/R w KQkq - 0 1", // Just rooks,
    "r4k/8/8/8/8/8/8/R4K w KQkq - 0 1", // Check,
    "r4k/8/8/8/8/8/8/RR4K w KQkq - 0 1", // Check,
    
];

const WINDOW_SIZE: [f32; 2] = [900.0, 600.0];
const PLAYING_AREA: f32 = 600.0;


struct PlayerWhite {
    parity: Parity
}

impl Player for PlayerWhite {
    fn your_turn(&self, state: &final_1::shared::state::State) -> (Mask, Mask) {
        return (Mask::default(), Mask::default());
    }

        /*
        let iso = ally_mask.isolated_bits();
        for bit in iso.iter() {
            let bit_point = bit.to_point().unwrap();
            for piece in pieces.iter() {
                if piece.borrow().get_props().pos == bit_point {
                    let cm = cached_moves.get(&ByAddress(piece.clone()));
                    if cm.is_some() && cm.unwrap().is_some() && cm.unwrap().unwrap().moves.any() {
                        return (*bit, cm.unwrap().unwrap().moves.isolated_bits()[0])
                    }
                }
            }
        }
        return ( Mask::default(), Mask::default() );
    }
        */
    fn get_parity(&self) -> Parity {
        return self.parity;
    }
}

fn main() -> () {
    let pb = PlayerWhite {
        parity: Parity::BLACK
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(WINDOW_SIZE),
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
                        FENS[4],
                        None,
                        None
            )))
        }),
    );

    if let Err(e) = finish {
        dbg!("App exited with error: {:?}", e);
    } else {
        dbg!("Shut down gracefully");
    }
}
