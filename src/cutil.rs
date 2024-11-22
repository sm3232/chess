pub mod draw {
    use eframe::egui::{self, Painter};
    use crate::shared::state::ChessByte;
    use crate::game;
    use crate::shared::piece::Parity;
    use crate::shared::point::Point;

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
        for (index, &byte) in game.state.board.iter().enumerate() {
            if byte == 0 {
                continue;
            }
            let mut path = "file:///home/sm/assm/final/rust/final-1/assets/".to_string();
            if byte.get_parity() == Parity::WHITE { path.push_str("light/") } else { path.push_str("dark/") };
            path.push_str(&byte.get_ptype().to_string().to_lowercase());
            path.push_str(".png");
            egui::Image::from_uri(path).paint_at(ui, egui::Rect {
                min: (Point::from_index(index) * sqsize).into(),
                max: ((Point::from_index(index) + Point { x: 1, y: 1 }) * sqsize).into()
            });
        }
    }
    pub fn draw_debug_info(game: &mut game::ChessGame, painter: &Painter, sqsize: f32) -> () {
        if game.selected != 65 {
            painter.debug_rect(egui::Rect {
                min: (Point::from_index(game.selected) * sqsize).into(),
                max: ((Point::from_index(game.selected) + Point { x: 1, y: 1 }) * sqsize).into()
            }, egui::Color32::GREEN, "SELECTED");
        }
        /*
        match &game.selected {
            Some(piece) => {
                painter.debug_rect(egui::Rect {
                    min: (piece.borrow().get_props().pos * sqsize).into(), 
                    max: ((piece.borrow().get_props().pos + point(1, 1)) * sqsize).into(),

                }, egui::Color32::GREEN, "SELECTED");

                let moves_option = game.cached_moves[piece.borrow().get_props().pos];
                if moves_option.moves.any() {
                    for &m in moves_option.moves.to_point_vector().iter() {
                        painter.debug_rect(egui::Rect {
                            min: (m * sqsize).into(),
                            max: ((m + point(1, 1)) * sqsize).into(),
                        }, egui::Color32::LIGHT_GREEN, "MOVE");
                    }
                    if piece.borrow().get_props().ptype == PieceByte::KING {
                        let bits = if game.turn == Parity::WHITE { game.castle_moves & 0b00000011 } else { (game.castle_moves & 0b00001100) >> 2 };
                        let pos = piece.borrow().get_props().pos;
                        if (bits & 0b00000001) != 0 {
                            painter.debug_rect(egui::Rect {
                                min: (Point {x: pos.x + 3, y: pos.y} * sqsize).into(),
                                max: ((Point {x: pos.x + 3, y: pos.y} + point(1, 1)) * sqsize).into()
                            }, egui::Color32::LIGHT_GREEN, "CASTLE");
                        }
                        if (bits & 0b00000010) != 0 {
                            painter.debug_rect(egui::Rect {
                                min: (Point {x: pos.x - 4, y: pos.y} * sqsize).into(),
                                max: ((Point {x: pos.x - 4, y: pos.y} + point(1, 1)) * sqsize).into()
                            }, egui::Color32::LIGHT_GREEN, "CASTLE");
                        }
                    }
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
        */
    }

    pub fn draw_all(game: &mut game::ChessGame, ui: &mut egui::Ui, bg_painter: &Painter, dbg_painter: &Painter) -> () {
        let sqsize = draw_board(bg_painter);
        draw_pieces(game, ui, sqsize);
        draw_debug_info(game, dbg_painter, sqsize);

    }

}



pub mod pretty_print {

    use stanza::renderer::console::Console;
    use stanza::renderer::Renderer;
    use stanza::style::{ HAlign, MaxWidth, MinWidth, Styles};
    use stanza::table::{ Col, Row, Table};
    use crate::shared::mask::Mask;

    #[allow(dead_code)]
    /*
    fn piece_to_table(p: Rc<RefCell<Box<dyn TPiece>>>) -> stanza::table::Table {
        return Table::with_styles(
            Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
        ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
            let x = p.borrow().get_props().pos.x;
            let y = p.borrow().get_props().pos.y;
            let mut v = vec!['0'; 7];
            if i == y {
                v.insert(x as usize, '1');
            } else {
                v.push('0');
            }
            return Row::from(v);
        })).into();
    }
    */
    #[allow(dead_code)]
    fn mask_to_table(m: &Mask) -> stanza::table::Table {
        let bv = &mut m.raw.to_ne_bytes();
        return Table::with_styles(
            Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
        ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
            let mut cells = Vec::<char>::new();
            for bit in 0..8 {
                if bv[i] & (1 << bit) != 0 {
                    cells.push('1');
                } else {
                    cells.push('0');
                }
            }
            return Row::from(cells);
        })).into()
    }
    /*
    #[allow(dead_code)]
    fn info_to_rows(p: Rc<RefCell<Box<dyn TPiece>>>, piece_m: &Mask, o: Rc<RefCell<Box<dyn TPiece>>>, other_m: &Mask) -> Vec<Row>{
        return vec![
            Row::new(
                Styles::default(),
                vec![
                    p.borrow().into(),
                    piece_m.to_point_or_00().into(),
                    o.borrow().into(),
                    other_m.to_point_or_00().into(),
                ]
            ),
            Row::new(
                Styles::default(),
                vec![
                    piece_to_table(p).into(),
                    mask_to_table(piece_m).into(),
                    piece_to_table(o).into(),
                    mask_to_table(other_m).into()
                ]
            )
        ];
    }
    /*
    pub fn pretty_print_future(using: Rc<RefCell<Box<dyn TPiece>>>, usings_move: &Mask, other_piece: Rc<RefCell<Box<dyn TPiece>>>, others_moves: &Mask){ 
        let frame = Table::with_styles(
            Styles::default()
        ).with_cols(vec![
            Col::default(),
            Col::default()
        ]).with_row(Row::from(["This piece", "Can move to", "Which gives this piece", "These moves"])).with_rows(
            info_to_rows(using, usings_move, other_piece, others_moves)
        );
        println!("{}", Console::default().render(&frame));
    }*/
    */

    pub fn pretty_print_mask(mask: &Mask) -> () {
        println!("{}", Console::default().render(&mask_to_table(mask)));
    }
}
