pub mod player;
pub mod cutil;
pub mod game;
pub mod shared;


use std::rc::Rc;
use std::thread;
use std::time::Duration;

use eframe::egui::{self, Color32, Painter};
use std::sync::mpsc;
use player::Player;
use crate::cutil::draw;
use crate::shared::piece::Parity;
pub struct ChessApp { 
    pub game: game::ChessGame,
    pub playing_area: f32,
    pub skip_update: bool,
    tx: mpsc::Sender<()>
}
impl ChessApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, playing_area: f32, init_fen: &str, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessApp {
        let ctx = creation_context.egui_ctx.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(|| update_loop(ctx, rx));
        return ChessApp {
            game: game::ChessGame::init(
                      init_fen.to_string(),
                      player_white,
                      player_black
                  ),
            playing_area,
            skip_update: true,
            tx
        };
    }
}
fn update_loop(frame: egui::Context, rx: mpsc::Receiver<()>) {
    loop {
        thread::sleep(Duration::from_millis(16));
        frame.request_repaint();
        match rx.try_recv() {
            Ok(_) | Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => ()
        }
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.skip_update = !self.skip_update;
        if self.game.game_over { let _ = self.tx.send(()); }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let game_rect = egui::Rect {
            min: egui::Pos2 { x: 0.0, y: 0.0 },
            max: egui::Pos2 { x: self.playing_area, y: self.playing_area }
        };
        let info_rect = egui::Rect {
            min: egui::Pos2 { x: self.playing_area, y: 0.0 },
            max: egui::Pos2 { x: ctx.screen_rect().width(), y: self.playing_area }
        };
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2 { x: 15.0, y: 0.0 };
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.horizontal_top(|ui| {
                let game_response = ui.allocate_rect(game_rect, egui::Sense { click: true, drag: false, focusable: false });
                if !self.game.game_over && self.skip_update {
                    if !self.game.poll_players() {
                        if self.game.human_player != Parity::NONE {
                            if self.game.human_player == self.game.state.turn || self.game.human_player == Parity::BOTH {
                                if game_response.is_pointer_button_down_on() {
                                    let pos = game_response.interact_pointer_pos().unwrap();
                                    let factor = game_rect.width() / 8.0;
                                    self.game.human_input((pos / factor).into(), self.game.human_player);
                                }
                            }
                        }
                    }
                }

                // Draw everything
                ui.horizontal(|ui| {
                    ui.with_layer_id(game_response.layer_id, |game_ui| {
                        game_ui.with_layer_id(egui::LayerId::new(egui::Order::Middle, egui::Id::new("fg")), |uui| {
                            let dbg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("dbg_painter")), game_rect);
                            let bg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Background, egui::Id::new("bg_painter")), game_rect);
                            draw::draw_all(&mut self.game, uui, &bg_painter, &dbg_painter);
                        });


                        if self.game.game_over {
                            game_ui.with_layer_id(egui::LayerId::new(egui::Order::TOP, egui::Id::new("top")), |uui| {
                                if self.game.state.turn == Parity::WHITE {
                                    let rich = egui::RichText::new("BLACK WINS").monospace().size(48.0).color(Color32::BLACK).background_color(draw::BOARD_W_COLOR);
                                    uui.put(game_rect, egui::Label::new(rich));
                                } else {
                                    let rich = egui::RichText::new("WHITE WINS").monospace().size(48.0).color(Color32::WHITE).background_color(draw::BOARD_B_COLOR);
                                    uui.put(game_rect, egui::Label::new(rich));
                                }
                            });
                        }
                    });

                    let mut castles: Vec<&str> = vec!["\n"];
                    if self.game.state.white_kingside_can_castle() { castles.push("White can castle kingside.") };
                    if self.game.state.white_queenside_can_castle() { castles.push("White can castle queenside.") };
                    if self.game.state.black_kingside_can_castle() { castles.push("Black can castle kingside.") };
                    if self.game.state.black_queenside_can_castle() { castles.push("Black can castle queenside.") };
                    if castles.len() != 1 {
                        let rich = egui::RichText::new(castles.join("\n")).monospace().color(Color32::WHITE);
                        let label = egui::Label::new(rich);
                        ui.put(info_rect, label);
                    }
                });

            });
        });
    }
}

