pub mod draw {
    use eframe::egui::{self, Painter};
    use crate::game;
    use crate::shared::{
        piece::Parity,
        point::Point,
        chessbyte::ChessByte
    };


    pub const BOARD_SIZE: i32 = 8;
    pub const WHITE_COLOR_VALUE: u8 = 128;
    pub const BLACK_COLOR_VALUE: u8 = 88;
    pub const MID_COLOR_VALUE: u8 = (WHITE_COLOR_VALUE + BLACK_COLOR_VALUE) / 2;
    pub const BOARD_W_COLOR: egui::Color32 = egui::Color32::from_rgb(WHITE_COLOR_VALUE, WHITE_COLOR_VALUE, WHITE_COLOR_VALUE);
    pub const BOARD_B_COLOR: egui::Color32 = egui::Color32::from_rgb(BLACK_COLOR_VALUE, BLACK_COLOR_VALUE, BLACK_COLOR_VALUE);

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
            let cached_moves = game.state.moves[game.selected];

            for &m in cached_moves.moves.to_point_vector().iter() {
                painter.debug_rect(egui::Rect {
                    min: (m * sqsize).into(),
                    max: ((m + Point { x: 1, y: 1 }) * sqsize).into(),
                }, egui::Color32::LIGHT_GREEN, "MOVE");
            }
        } else {
            for cm in game.state.threats.iter() {
                painter.debug_rect(cm.moves.to_painter_rect(sqsize), egui::Color32::YELLOW, "THREAT");
            }

        }
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
    use crate::shared::chessbyte::ChessByte;
    use crate::shared::mask::Mask;
    use crate::shared::piece::PieceCachedMoves;

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
    fn board_to_table(b: &[u8; 64]) -> stanza::table::Table {
        return Table::with_styles(
            Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
        ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
            let mut cells = Vec::<char>::new();
            for col in 0..8 {
                cells.push(b[i * 8 + col].to_letter());
            }
            return Row::from(cells);
        })).into()
    }
    fn moveset_to_table(m: &[PieceCachedMoves; 64]) -> stanza::table::Table {
        let mut mask = Mask::default();
        for pcm in m.iter() {
            mask |= pcm.moves;
        }
        return mask_to_table(&mask);
    }
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
    pub fn pretty_print_moveset(moveset: &[PieceCachedMoves; 64]) -> () {
        println!("{}", Console::default().render(&moveset_to_table(moveset)));
    }
    pub fn pretty_print_board(board: &[u8; 64]) -> () {
        println!("{}", Console::default().render(&board_to_table(board)));
    }
}
