pub mod player;
pub mod cutil;
pub mod game;
pub mod shared;


use std::rc::Rc;
use std::thread;
use std::time::Duration;

use cutil::draw::{usize_painter_rect, visual_weight_remap_table, MID_COLOR_VALUE};
use cutil::pretty_print::pretty_string_evaluator;
use eframe::egui::{self, Color32, Painter};
use shared::chessbyte::ChessByte;
use shared::eval::{self, Evaluator};
use shared::eval::material::get_visual_material_weights;
use shared::piece::PieceByte;
use shared::searchtree::{SearchTree, ROOT_C, ROOT_S};
use shared::point::Point;
use std::sync::mpsc;
use player::Player;
use crate::cutil::draw;
use crate::shared::piece::Parity;
pub struct ChessApp { 
    pub game: game::ChessGame,
    tx: mpsc::Sender<()>,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub current_eval: Evaluator
}
impl ChessApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, playing_area: f32, info_width: f32, init_fen: &str, player_white: Option<Rc<dyn Player>>, player_black: Option<Rc<dyn Player>>) -> ChessApp {
        let ctx = creation_context.egui_ctx.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(|| update_loop(ctx, rx));
        let mut cap = ChessApp {
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
            },
            current_eval: Evaluator { eval: 0, scores: Vec::new() }
        };
        cap.current_eval = eval::start_eval(&cap.game.state.borrow());
        return cap;
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
    pos: Option<Point>,
    wants_unpause: bool
}
impl Input {
    pub fn from_tuple(tuple: (bool, bool, bool, Option<egui::Pos2>, bool)) -> Input {
        let mut po: Option<Point> = None;
        if let Some(p) = tuple.3 {
            po = Some((p / (600.0 / 8.0)).into());
        }
        return Input {
            wants_escape: tuple.0,
            left: tuple.1,
            right: tuple.2,
            pos: po,
            wants_unpause: tuple.4
        };
    }
}

fn collect_input(ctx: &egui::Context) -> Input {
    return Input::from_tuple(ctx.input(|i| (i.key_down(egui::Key::Escape), i.pointer.primary_pressed(), i.pointer.secondary_clicked(), i.pointer.latest_pos(), i.key_pressed(egui::Key::Space))));

}

const INFO_LINE_HEIGHT: f32 = 16.0;
const WEIGHT_VIS_SIZE: f32 = 100.0;

fn remap(v: i32, inpair: (i32, i32), outpair: (i32, i32)) -> i32 {
    return outpair.0 + (v - inpair.0) * (outpair.1 - outpair.0) / (inpair.1 - inpair.0);
}
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
        if !self.game.paused && self.game.human_player == Parity::NONE {
            self.game.paused = true;
        }
        if input.wants_unpause {
            self.game.paused = false;
        }
        egui::CentralPanel::default().frame(egui::Frame::none().inner_margin(egui::Margin{ top: 0.0, left: 0.0, bottom: 0.0, right: 30.0 })).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2 { x: 15.0, y: 0.0 };
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.horizontal_top(|ui| {
                if !self.game.game_over && !self.game.paused {
                    if !self.game.poll_players() {
                        if self.game.human_player != Parity::NONE {
                            if self.game.human_player == self.game.state.borrow().turn || self.game.human_player == Parity::BOTH {
                                if input.left {
                                    self.game.human_input(input.pos.unwrap(), self.game.human_player);
                                    self.current_eval = eval::start_eval(&self.game.state.borrow());
                                }
                            }
                        }
                    } else {
                        self.current_eval = eval::start_eval(&self.game.state.borrow());
                    }
                }
                // Draw everything
                ui.horizontal(|ui| {
                    let game_response = ui.allocate_rect(self.game_rect, egui::Sense { click: true, drag: false, focusable: false });
                    ui.with_layer_id(game_response.layer_id, |game_ui| {
                        game_ui.with_layer_id(egui::LayerId::new(egui::Order::Middle, egui::Id::new("fg")), |uui| {
                            let dbg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("dbg_painter")), self.game_rect);
                            let bg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Background, egui::Id::new("bg_painter")), self.game_rect);
                            draw::draw_all(&mut self.game, uui, &bg_painter, &dbg_painter, input.pos);
                        });


                        if self.game.game_over {
                            game_ui.with_layer_id(egui::LayerId::new(egui::Order::TOP, egui::Id::new("top")), |uui| {
                                if self.game.state.borrow().turn == Parity::WHITE {
                                    let rich = egui::RichText::new("BLACK WINS").monospace().size(48.0).color(Color32::BLACK).background_color(draw::BOARD_W_COLOR);
                                    uui.put(self.game_rect, egui::Label::new(rich));
                                } else {
                                    let rich = egui::RichText::new("WHITE WINS").monospace().size(48.0).color(Color32::WHITE).background_color(draw::BOARD_B_COLOR);
                                    uui.put(self.game_rect, egui::Label::new(rich));
                                }
                            });
                        }
                    });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2 { x: 15.0, y: 15.0 };
                        let mut castles: Vec<&str> = vec![];
                        if self.game.state.borrow().white_kingside_can_castle() { castles.push("White can castle kingside.") };
                        if self.game.state.borrow().white_queenside_can_castle() { castles.push("White can castle queenside.") };
                        if self.game.state.borrow().black_kingside_can_castle() { castles.push("Black can castle kingside.") };
                        if self.game.state.borrow().black_queenside_can_castle() { castles.push("Black can castle queenside.") };
                        ui.add_space(15.0);
                        if castles.len() != 0 {
                            let rounded_height = ui.painter().round_to_pixel(INFO_LINE_HEIGHT);
                            let label = egui::Label::new(
                                egui::RichText::new(castles.join("\n"))
                                .monospace()
                                .line_height(Some(rounded_height))
                            ).halign(egui::Align::Min).selectable(false);
                            let tl = self.info_rect.left_top() + egui::Vec2 { x: 15.0, y: 0.0 };
                            ui.add_sized(egui::Rect {
                                min: tl + egui::Vec2 { x: 0.0, y: ui.painter().round_to_pixel(INFO_LINE_HEIGHT / 2.0) },
                                max: tl + egui::Vec2 { x: 27.0 * 7.0, y: rounded_height * (castles.len() as f32) }
                            }.size(), label);
                        } else {
                            ui.add_space(ui.painter().round_to_pixel(INFO_LINE_HEIGHT) / 2.0 + 7.0);
                        }
                        ui.add_space(ui.painter().round_to_pixel(INFO_LINE_HEIGHT) * (4.0 - castles.len() as f32));
                        ui.label(egui::RichText::new("Evaluation").monospace());
                        ui.label(egui::RichText::new(pretty_string_evaluator(&self.current_eval)));

                        let (_, weight_rect) = ui.allocate_space(egui::Vec2 { x: WEIGHT_VIS_SIZE, y: WEIGHT_VIS_SIZE });
                        let weight_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("weight_painter")), weight_rect);
                        if let Some(hp) = input.pos {
                            if hp.valid() {
                                let p = self.game.state.borrow().board[hp];
                                if p.get_piece() != PieceByte::NONE {
                                    let vw = get_visual_material_weights(p);
                                    for (index, &weight) in vw.iter().enumerate() {
                                        let color = egui::Color32::from_rgb(255, 0, 0).lerp_to_gamma(egui::Color32::from_rgb(0, 0, 255), (remap(weight, visual_weight_remap_table(p.get_piece()), (0, 100)) as f32) / 100.0 );
                                        weight_painter.rect_filled(egui::Rect {
                                            min: (weight_rect.min + Into::<egui::Pos2>::into(Point::from_index(index)).to_vec2() * WEIGHT_VIS_SIZE / 8.0),
                                            max: (weight_rect.min + Into::<egui::Pos2>::into(Point::from_index(index) + Point { x: 1, y: 1 }).to_vec2() * WEIGHT_VIS_SIZE / 8.0)
                                        }, egui::Rounding::ZERO, color);
                                    }

                                }
                            }
                        }
                    });
                    let (tree_id, tree_rect) = ui.allocate_space(egui::Vec2 { x: ui.available_width(), y: ui.available_height() });
                    let tree_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("tree_painter")), tree_rect);
                    for tree in self.game.trees.iter() {
                        SearchTree::display(
                            tree, 
                            ui, 
                            &tree_painter
                        );
                    }



                });

            });
        });
    }
}

