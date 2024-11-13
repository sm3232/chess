use std::cell::RefCell;

use eframe::egui;


use crate::cutil::{game::ChessGame, point::{point, Point}, mask::Mask};



#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum PieceType {
    ROOK, PAWN, BISHOP, QUEEN, KING, KNIGHT
}

pub trait TPiece {
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> ();
    fn get_props(&self) -> PieceProperties;
    fn get_moves(&self, ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, cg: &ChessGame) -> Mask;
    fn set_props(&mut self, props: PieceProperties) -> ();
}
pub fn parity_to_string(parity: bool) -> &'static str {
    if parity { return "BLACK" } else { return "WHITE" };
}

impl std::fmt::Debug for dyn TPiece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{:?}\n{:?}\n{:?}", self.get_props().pos, self.get_props().ptype, parity_to_string(self.get_props().parity));
    }

}

pub fn create_piece(props: PieceProperties) -> RefCell<Box<dyn TPiece>> {
    match props.ptype {
        PieceType::ROOK => return RefCell::new(Box::new(Rook { props })),
        PieceType::PAWN => return RefCell::new(Box::new(Pawn { props })),
        PieceType::KING => return RefCell::new(Box::new(King { props })),
        PieceType::QUEEN => return RefCell::new(Box::new(Queen { props })),
        PieceType::BISHOP => return RefCell::new(Box::new(Bishop { props })),
        PieceType::KNIGHT => return RefCell::new(Box::new(Knight { props }))
    }

}
#[derive(Clone, Copy)]
pub struct PieceProperties {
    pub parity: bool,
    pub pos: Point,
    pub ptype: PieceType,
    pub has_moved: bool
}

impl std::cmp::PartialEq<PieceProperties> for PieceProperties {
    fn eq(&self, other: &PieceProperties) -> bool {
        return self.has_moved == other.has_moved && self.pos == other.pos && self.ptype == other.ptype && self.parity == other.parity;
    }
}

impl std::cmp::PartialEq<dyn TPiece> for dyn TPiece {
    fn eq(&self, other: &dyn TPiece) -> bool {
        return self.get_props() == other.get_props();
    }
}



pub struct Rook { pub props: PieceProperties }
pub struct Pawn { 
    pub props: PieceProperties,
}
pub struct Bishop { pub props: PieceProperties }
pub struct Queen { pub props: PieceProperties }
pub struct King { pub props: PieceProperties }
pub struct Knight { pub props: PieceProperties }

fn sliding_move(position: Point, directions: &Vec<Point>, enemy_mask: &Mask, piece_mask: &Mask) -> Mask {
    let mut move_mask: Mask = Mask::default();
    let mut hits: Vec<bool> = vec![false; directions.len()];
    for i in 1..=8 {
        for k in 0..directions.len() {
            let pos = position + (directions[k] * i);
            if pos.valid() {
                if hits[k] { continue };
                let pos_mask = Mask::from_point(pos);
                if (*piece_mask & pos_mask).any() {
                    if (*enemy_mask & pos_mask).any() {
                        move_mask |= pos_mask;
                    }
                    hits[k] = true;
                } else {
                    move_mask |= pos_mask;
                }
            } else {
                hits[k] = true;
            }
        }
    }
    return move_mask;
}


impl TPiece for Rook {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) -> () {
        self.props = props;
    }
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/rook.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/rook.png")).paint_at(ui, rect);
        }
    }
    fn get_moves(&self, _ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, _cg: &ChessGame) -> Mask {
        let dirs = vec![
            point(1, 0),
            point(-1, 0),
            point(0, 1),
            point(0, -1)
        ];
        return sliding_move(self.props.pos, &dirs, enemy_mask, piece_mask);
    }
}

impl TPiece for Pawn {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) { 
        self.props = props 
    }
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/pawn.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/pawn.png")).paint_at(ui, rect);
        }
    }

    fn get_moves(&self, _ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, cg: &ChessGame) -> Mask {
        let mut move_mask: Mask = Mask::default();
        let par = if self.props.parity { 1 } else { -1 };
        let basic = point(self.props.pos.x, self.props.pos.y + par);
        if !basic.valid() {
            return move_mask;
        }
        let basic_mask = Mask::from_point(basic);
        if (*piece_mask & basic_mask).none() {
            move_mask |= basic_mask;
            if !self.props.has_moved {
                let dbl_point = point(basic.x, basic.y + par);
                if dbl_point.valid() {
                    let dbl = Mask::from_point(point(basic.x, basic.y + par));
                    if (*piece_mask & dbl).none() {
                        move_mask |= dbl;
                    }
                }
            }
        }
        let diags: [Point; 2] = [
            point(basic.x - 1, basic.y),
            point(basic.x + 1, basic.y)
        ];
        for d in diags {
            if d.valid() {
                let pos_mask = Mask::from_point(d);
                if (*enemy_mask & pos_mask).any() {
                    move_mask |= pos_mask;
                } else if (*piece_mask & pos_mask).none() {
                    match cg.enpassant {
                        Some(n) => {
                            if n == d {
                                move_mask |= pos_mask;
                            }
                        },
                        None => ()
                    }
                }

            }
        }
        return move_mask;
    }

}
impl TPiece for Bishop {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) { 
        self.props = props 
    }
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/bishop.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/bishop.png")).paint_at(ui, rect);
        }
    }
    fn get_moves(&self, _ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, _cg: &ChessGame) -> Mask {
        let dirs = vec![
            point(1, 1),
            point(-1, 1),
            point(-1, -1),
            point(1, -1)
        ];
        return sliding_move(self.props.pos, &dirs, enemy_mask, piece_mask)
    }

}
impl TPiece for Queen {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) { 
        self.props = props 
    }

    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/queen.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/queen.png")).paint_at(ui, rect);
        }
    }
    fn get_moves(&self, _ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, _cg: &ChessGame) -> Mask {
        let dirs = vec![
            point(1, 1),
            point(-1, 1),
            point(-1, -1),
            point(1, -1),
            point(1, 0),
            point(-1, 0),
            point(0, 1),
            point(0, -1)
        ];
        return sliding_move(self.props.pos, &dirs, enemy_mask, piece_mask)
        
    }
}
impl TPiece for King {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) { 
        self.props = props 
    }
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/king.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/king.png")).paint_at(ui, rect);
        }
    }
    fn get_moves(&self, ally_mask: &Mask, _enemy_mask: &Mask, _piece_mask: &Mask, _cg: &ChessGame) -> Mask {
        let mut move_mask: Mask = Mask::default();
        for y in -1..2 {
            for x in -1..2 {
                if x == 0 && y == 0 { continue };
                let pos = self.props.pos + point(x, y);
                if pos.valid() {
                    let pos_mask = Mask::from_point(pos);
                    if (*ally_mask & pos_mask).none() {
                        move_mask |= pos_mask;
                    }
                }

            }
        }
        return move_mask;
    }

}
impl TPiece for Knight {
    fn get_props(&self) -> PieceProperties {
        return self.props;
    }
    fn set_props(&mut self, props: PieceProperties) { 
        self.props = props 
    }
    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect) -> () {
        if self.props.parity {
            egui::Image::new(egui::include_image!("../../assets/dark/knight.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../../assets/light/knight.png")).paint_at(ui, rect);
        }
    }
    fn get_moves(&self, _ally_mask: &Mask, enemy_mask: &Mask, piece_mask: &Mask, _cg: &ChessGame) -> Mask {
        let mut move_mask: Mask = Mask::default();
        
        let offsets = [
            point(-2, -1), point(-2, 1), point(-1, -2), point(-1, 2), point(1, -2), point(1, 2), point(2, -1), point(2, 1)
        ];
        for offset in offsets {
            let pos = self.props.pos + offset;
            if pos.valid() {
                let pos_mask = Mask::from_point(pos);
                if(*piece_mask & pos_mask).any() {
                    if(*enemy_mask & pos_mask).any() {
                        move_mask |= pos_mask;
                    }
                } else {
                    move_mask |= pos_mask;
                }
            }
        }
        return move_mask;
    }
}
