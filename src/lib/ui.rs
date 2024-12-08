
use std::ops::Deref;
use std::{rc::Rc, sync::Mutex};
use std::thread;
use std::time::Duration;

use crate::lib::{
    cutil::draw::{usize_painter_rect, visual_weight_remap_table, MID_COLOR_VALUE},
    cutil::draw,
    cutil::pretty_print::pretty_string_evaluator,
    eval::{self, Evaluator},
    eval::material::get_visual_material_weights,
    piece::PieceByte,
    searchtree::{SearchTree, ROOT_C, ROOT_S},
    point::Point,
    chessbyte::ChessByte,
    piece::Parity,
    player::Player,
    game::ChessGame,
    manager::Manager
};

use std::sync::{mpsc, Arc};
use eframe::egui::{self, Color32, Painter, RequestRepaintInfo};

use super::manager::{SharedState, VisualInfo};
pub struct ChessApp {
    pub receiver: mpsc::Receiver<SharedState>,
    pub sender: mpsc::Sender<Input>,
    pub paused: bool,
    pub has_human: bool,
    pub game_over: bool,
    pub game_rect: egui::Rect,
    pub info_rect: egui::Rect,
    pub last_visual_state: VisualInfo,
    pub solid_board: [u8; 64]
}

impl ChessApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, playing_area: f32, info_width: f32, init_fen: String) -> ChessApp {
        let ctx = creation_context.egui_ctx.clone();
        let (send1, recv1) = mpsc::channel();
        let (send2, recv2) = mpsc::channel();
        thread::spawn(move || Manager::init(ctx, send1, recv2, init_fen, playing_area, info_width));
        return ChessApp {
            receiver: recv1,
            sender: send2,
            paused: false,
            game_over: false,
            has_human: false,
            game_rect: egui::Rect {
                min: egui::Pos2 { x: 0.0, y: 0.0 },
                max: egui::Pos2 { x: playing_area, y: playing_area }
            },
            info_rect: egui::Rect {
                min: egui::Pos2 { x: playing_area, y: 0.0 },
                max: egui::Pos2 { x: playing_area + info_width, y: playing_area }
            },
            last_visual_state: VisualInfo::none(),
            solid_board: [0u8; 64]
        };

    }
}

#[inline(always)]
pub fn white_kingside_can_castle(cs: u8) -> bool { (cs & 0b00000100) != 0 }
#[inline(always)]
pub fn white_queenside_can_castle(cs: u8) -> bool { (cs & 0b00001000) != 0 }
#[inline(always)]
pub fn black_kingside_can_castle(cs: u8) -> bool { (cs & 0b00000001) != 0 }
#[inline(always)]
pub fn black_queenside_can_castle(cs: u8) -> bool { (cs & 0b00000010) != 0 }
pub struct Input {
    pub wants_escape: bool,
    pub left: bool,
#[allow(dead_code)]
    pub right: bool,
    pub pos: Option<Point>,
    pub wants_unpause: bool
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
impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "Escape? {}\t\tLeft? {}\t\tRight? {}\t\t, Pos? {:#?}\t\tUnpause? {}", self.wants_escape, self.left, self.right, self.pos, self.wants_unpause);
    }
}
impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "Escape? {}\t\tLeft? {}\t\tRight? {}\t\t, Pos? {:#?}\t\tUnpause? {}", self.wants_escape, self.left, self.right, self.pos, self.wants_unpause);
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
        let state = match self.receiver.recv() {
            Err(..) => panic!("No recv in ui thread"),
            Ok(x) => x
        };

        let input = collect_input(ctx);
        if input.wants_escape {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.paused && self.has_human {
            self.paused = true;
        }
        if input.wants_unpause {
            self.paused = false;
        }

        if state.visuals.cache_saves.is_some_and(|x| x != 0) {
            self.last_visual_state.cache_saves = state.visuals.cache_saves;
        }
        if state.visuals.analyzed.is_some_and(|x| x != 0) {
            self.last_visual_state.analyzed = state.visuals.analyzed;
        }
        if state.visuals.visual_weights.is_some() {
            self.last_visual_state.visual_weights = state.visuals.visual_weights;
        }
        if state.visuals.evaluation.is_some() {
            self.last_visual_state.evaluation = state.visuals.evaluation.clone();
        }
        if state.visuals.tree.as_ref().is_some_and(|x| !x.lock().unwrap().children.is_empty()) {
            self.last_visual_state.tree = Some(Arc::clone(state.visuals.tree.as_ref().unwrap()));
        }

        if !state.working {
            self.solid_board = state.board;
        }

        
        egui::CentralPanel::default().frame(egui::Frame::none().inner_margin(egui::Margin{ top: 0.0, left: 0.0, bottom: 0.0, right: 30.0 })).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2 { x: 15.0, y: 0.0 };
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.horizontal_top(|ui| {
                ui.horizontal(|ui| {
                    let game_response = ui.allocate_rect(self.game_rect, egui::Sense { click: true, drag: false, focusable: false });
                    ui.with_layer_id(game_response.layer_id, |game_ui| {
                        game_ui.with_layer_id(egui::LayerId::new(egui::Order::Middle, egui::Id::new("fg")), |uui| {
                            let dbg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("dbg_painter")), self.game_rect);
                            let bg_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Background, egui::Id::new("bg_painter")), self.game_rect);
                            if state.working {
                                draw::draw_pieces(&state.board, uui, bg_painter.clip_rect().width().min(bg_painter.clip_rect().height()) / 8.0, true);
                            }
                            draw::draw_all(&self.solid_board, state.selected, &state.moves, uui, &bg_painter, &dbg_painter, input.pos, false);
                        });

                        if state.game_over {
                            game_ui.with_layer_id(egui::LayerId::new(egui::Order::TOP, egui::Id::new("top")), |uui| {
                                if state.turn == Parity::WHITE {
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
                        if white_kingside_can_castle(state.allowed_castles) { castles.push("White can castle kingside.") };
                        if white_queenside_can_castle(state.allowed_castles) { castles.push("White can castle queenside.") };
                        if black_kingside_can_castle(state.allowed_castles) { castles.push("Black can castle kingside.") };
                        if black_queenside_can_castle(state.allowed_castles) { castles.push("Black can castle queenside.") };
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
                    
                        ui.label(egui::RichText::new(pretty_string_evaluator(state.visuals.evaluation.as_ref().unwrap())));

                        let (_, weight_rect) = ui.allocate_space(egui::Vec2 { x: WEIGHT_VIS_SIZE, y: WEIGHT_VIS_SIZE });
                        let weight_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("weight_painter")), weight_rect);
                        if let Some(hp) = input.pos {
                            if hp.valid() {
                                let p = state.board[hp];
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
                        ui.label(egui::RichText::new(format!("Searched {} positions", state.visuals.analyzed.unwrap_or(self.last_visual_state.analyzed.unwrap_or(0)))));
                        ui.label(egui::RichText::new(format!("Saved {} searches with caching", state.visuals.cache_saves.unwrap_or(self.last_visual_state.cache_saves.unwrap_or(0)))));
                    });
                    let (_, tree_rect) = ui.allocate_space(egui::Vec2 { x: ui.available_width(), y: ui.available_height() * 2.0 });
                    let tree_painter = Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Debug, egui::Id::new("tree_painter")), tree_rect);
                    if state.working {
                        SearchTree::display(state.visuals.tree.as_ref().unwrap(), ui, &tree_painter);
                    } else if self.last_visual_state.tree.is_some() {
                        SearchTree::display(self.last_visual_state.tree.as_ref().unwrap(), ui, &tree_painter);
                    }
                });

            });
            if state.waiting_for_a_human_input {
                let _ = self.sender.send(input).unwrap();
            }
        });
    }
}

