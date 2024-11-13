
use eframe::egui;
mod cutil;
use cutil::{draw, game};

const FENS: [&str; 3] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r/8/8/8/8/8/8/R w KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq a4 0 1",
];

struct ChessApp { 
    pub game: game::ChessGame
}
impl Default for ChessApp {
    fn default() -> Self {
        return Self { game: game::ChessGame::init(String::from(FENS[0])) };
    }
}


fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 600.0]),
        ..Default::default()
    };
    return eframe::run_native(
        "Chess",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<ChessApp>::default())
        }),
    );
}


impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let dbg_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::new("dbg")));
        let bg_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("bg")));
        let click = ctx.input(|i| i.pointer.any_pressed());
        if click {
            let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap();
            let factor = ctx.screen_rect().width() / 8.0;
            self.game.select((pos / factor).into());
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layer_id(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("fg")), |uui| {
                draw::draw_all(&mut self.game, uui, &bg_painter, &dbg_painter);
            });
        });
    }
}

