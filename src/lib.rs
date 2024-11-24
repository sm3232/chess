pub mod player;
pub mod cutil;
pub mod game;
pub mod shared;


use std::rc::Rc;
use std::thread;
use std::time::Duration;

use cutil::draw::{BLACK_COLOR_VALUE, MID_COLOR_VALUE, WHITE_COLOR_VALUE};
use eframe::egui::{self, Color32, Painter};
use shared::point::Point;
use std::sync::mpsc;
use player::Player;
use crate::cutil::draw;
use crate::shared::piece::Parity;
pub struct ChessApp { 
    pub game: game::ChessGame,
    tx: mpsc::Sender<()>,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect
}
impl ChessApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, playing_area: f32, info_width: f32, init_fen: &str, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessApp {
        let ctx = creation_context.egui_ctx.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(|| update_loop(ctx, rx));
        return ChessApp {
            game: game::ChessGame::init(
                      init_fen.to_string(),
                      player_white,
                      player_black
                  ),
            tx,
            game_rect: egui::Rect {
                min: egui::Pos2 { x: 0.0, y: 0.0 },
                max: egui::Pos2 { x: playing_area, y: playing_area }
            },
            info_rect: egui::Rect {
                min: egui::Pos2 { x: playing_area, y: 0.0 },
                max: egui::Pos2 { x: playing_area + info_width, y: playing_area }
            }
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

struct Input {
    wants_escape: bool,
    left: bool,
#[allow(dead_code)]
    right: bool,
    pos: Option<Point>
}
impl Input {
    pub fn from_tuple(tuple: (bool, bool, bool, Option<egui::Pos2>)) -> Input {
        let mut po: Option<Point> = None;
        if let Some(p) = tuple.3 {
            po = Some((p / (600.0 / 8.0)).into());
        }
        return Input {
            wants_escape: tuple.0,
            left: tuple.1,
            right: tuple.2,
            pos: po
        };
    }
}

fn collect_input(ctx: &egui::Context) -> Input {
    return Input::from_tuple(ctx.input(|i| (i.key_down(egui::Key::Escape), i.pointer.primary_pressed(), i.pointer.secondary_clicked(), i.pointer.latest_pos())));

}

const INFO_LINE_HEIGHT: f32 = 16.0;

impl eframe::App for ChessApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        return egui::Rgba::from_srgba_unmultiplied(MID_COLOR_VALUE, MID_COLOR_VALUE, MID_COLOR_VALUE, 255).to_array();
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        
        // ctx.set_debug_on_hover(true);
        if self.game.game_over { let _ = self.tx.send(()); }
        let input = collect_input(ctx);
        if input.wants_escape {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2 { x: 15.0, y: 0.0 };
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.horizontal_top(|ui| {
                if !self.game.game_over {
                    if !self.game.poll_players() {
                        if self.game.human_player != Parity::NONE {
                            if self.game.human_player == self.game.state.turn || self.game.human_player == Parity::BOTH {
                                if input.left {
                                    self.game.human_input(input.pos.unwrap(), self.game.human_player);
                                }
                            }
                        }
                    }
                }
                // Draw everything
                ui.horizontal(|ui| {
                    let game_response = ui.allocate_rect(self.game_rect, egui::Sense { click: true, drag: false, focusable: false });
                    ui.with_layer_id(game_response.layer_id, |game_ui| {
                        game_ui.with_layer_id(egui::LayerId::new(egui::Order::Middle, egui::Id::new("fg")), |uui| {
                            let dbg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("dbg_painter")), self.game_rect);
                            let bg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Background, egui::Id::new("bg_painter")), self.game_rect);
                            draw::draw_all(&mut self.game, uui, &bg_painter, &dbg_painter);
                        });


                        if self.game.game_over {
                            game_ui.with_layer_id(egui::LayerId::new(egui::Order::TOP, egui::Id::new("top")), |uui| {
                                if self.game.state.turn == Parity::WHITE {
                                    let rich = egui::RichText::new("BLACK WINS").monospace().size(48.0).color(Color32::BLACK).background_color(draw::BOARD_W_COLOR);
                                    uui.put(self.game_rect, egui::Label::new(rich));
                                } else {
                                    let rich = egui::RichText::new("WHITE WINS").monospace().size(48.0).color(Color32::WHITE).background_color(draw::BOARD_B_COLOR);
                                    uui.put(self.game_rect, egui::Label::new(rich));
                                }
                            });
                        }
                    });
                    let mut castles: Vec<&str> = vec![];
                    if self.game.state.white_kingside_can_castle() { castles.push("White can castle kingside.") };
                    if self.game.state.white_queenside_can_castle() { castles.push("White can castle queenside.") };
                    if self.game.state.black_kingside_can_castle() { castles.push("Black can castle kingside.") };
                    if self.game.state.black_queenside_can_castle() { castles.push("Black can castle queenside.") };
                    if castles.len() != 0 {
                        let rounded_height = ui.painter().round_to_pixel(INFO_LINE_HEIGHT);
                        let label = egui::Label::new(
                            egui::RichText::new(castles.join("\n"))
                            .monospace()
                            .color(Color32::WHITE)
                            .line_height(Some(rounded_height))
                        ).halign(egui::Align::Min).selectable(false);
                        let tl = self.info_rect.left_top() + egui::Vec2 { x: 15.0, y: 0.0 };
                        ui.put(egui::Rect {
                            min: tl + egui::Vec2 { x: 0.0, y: ui.painter().round_to_pixel(INFO_LINE_HEIGHT / 2.0) },
                            max: tl + egui::Vec2 { x: 27.0 * 7.0, y: rounded_height * (castles.len() as f32) }
                        }, label);
                    }

                });

            });
        });
    }
}

