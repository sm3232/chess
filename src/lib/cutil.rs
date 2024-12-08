pub mod draw {
    use eframe::egui::{self, Painter};
    use crate::lib::{
        boardarray::BoardArray, chessbyte::ChessByte, game, mask::Mask, motion::Motion, piece::{Parity, PieceByte}, point::Point
    };

    pub fn visual_weight_remap_table(piece: PieceByte) -> (i32, i32) {
        return match piece {
            PieceByte::PAWN => (-23, 40),
            PieceByte::BISHOP => (-53, 39),
            PieceByte::KNIGHT => (-201, 58),
            PieceByte::ROOK => (-31, 18),
            PieceByte::QUEEN => (-5, 14),
            PieceByte::KING => (-1, 327),
            PieceByte::NONE => (0, 0)
        };
    }
    



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
    pub fn draw_pieces (board: &[u8; 64], ui: &mut egui::Ui, sqsize: f32, lightly: bool) -> () {
        for (index, &byte) in board.iter().enumerate() {
            if byte == 0 {
                continue;
            }
            let mut path = "file:///home/sm/assm/final/rust/chess/assets/".to_string();
            if byte.get_parity() == Parity::WHITE { path.push_str("light/") } else { path.push_str("dark/") };
            path.push_str(&byte.get_piece().to_string().to_lowercase());
            path.push_str(".png");
            let mut img = egui::Image::from_uri(path);
            if lightly {
                img = img.tint(egui::Color32::WHITE.lerp_to_gamma(egui::Color32::TRANSPARENT, 0.9));
            }
            img.paint_at(ui, egui::Rect {
                min: (Point::from_index(index) * sqsize).into(),
                max: ((Point::from_index(index) + Point { x: 1, y: 1 }) * sqsize).into()
            });
        }
    }
    pub fn usize_painter_rect(u: usize, sqsize: f32) -> egui::Rect {
        return eframe::egui::Rect {
            min: (Point::from_index(u) * sqsize).into(),
            max: ((Point::from_index(u) + Point { x: 1, y: 1 }) * sqsize).into()
        };
    }

    pub fn draw_debug_info(board: &[u8; 64], selected: usize, moves: &[Vec<Motion>; 64], painter: &Painter, sqsize: f32, hover: Option<Point>) -> () {
        if selected != 65 {
            painter.debug_rect(Mask::from_index(selected).to_painter_rect(sqsize), egui::Color32::GREEN, "SELECTED");
            let cached_moves = &moves[selected];
            for m in cached_moves.iter() { painter.debug_rect(usize_painter_rect(m.to, sqsize), egui::Color32::LIGHT_GREEN, "MOVE") };
        }
        if let Some(hp) = hover {
            if hp.valid() {
                let cached_moves = &moves[hp.to_index()];
                for m in cached_moves.iter() {
                    painter.debug_rect(usize_painter_rect(m.to, sqsize), egui::Color32::LIGHT_GREEN, "MOVE") ;
                }
            }
        }

    }

    pub fn draw_all(board: &[u8; 64], selected: usize, moves: &[Vec<Motion>; 64], ui: &mut egui::Ui, bg_painter: &Painter, dbg_painter: &Painter, hover: Option<Point>, lightly: bool) -> () {
        let sqsize = draw_board(bg_painter);
        draw_pieces(board, ui, sqsize, lightly);
        if !lightly {
            draw_debug_info(board, selected, moves, dbg_painter, sqsize, hover);
        }
    }

}



pub mod pretty_print {

    use stanza::renderer::console::{Console, Decor};
    use stanza::renderer::Renderer;
    use stanza::style::{ HAlign, Header, MaxWidth, MinWidth, Styles};
    use stanza::table::{ Cell, Col, Content, Row, Table};
    use crate::lib::chessbyte::ChessByte;
    use crate::lib::eval::{EvaluationTerm, Evaluator};
    use crate::lib::mask::{DepthMask, Mask, ValueMask};
    use crate::lib::maskset::MaskSet;
    use crate::lib::motion::Motion;

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
    pub fn maskset_to_table(title: &str, maskset: &MaskSet) -> Table {
        return table_with_title(title, 
            Table::with_styles(Styles::default()).with_cols(vec![
                Col::default(),
                Col::default(),
                Col::default()
            ]).with_row(
                Row::new(Styles::default(), vec![
                    table_with_title("White", mask_to_table(&maskset.white)).into(),
                    table_with_title("Black", mask_to_table(&maskset.black)).into(),
                    table_with_title("All", mask_to_table(&maskset.all)).into()
                ])
            )
        )

    }
    fn table_with_title(title: &str, table: Table) -> Table {
        let mut wrapper = Table::with_styles(Styles::default().with(HAlign::Centred));
        wrapper.push_row(Row::from([String::from(title)]));
        wrapper.push_row(Row::new(Styles::default(), vec![table.into()]));
        return wrapper;
    }
    fn board_to_table(title: &str, b: &[u8; 64]) -> stanza::table::Table {
        return table_with_title(title, 
            Table::with_styles(
                Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
            ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
                let mut cells = Vec::<char>::new();
                for col in 0..8 {
                    cells.push(b[i * 8 + col].to_letter());
                }
                return Row::from(cells);
            }))
        );
    }

    fn moveset_to_table(m: &[Vec<Motion>; 64]) -> stanza::table::Table {
        let mut mask = Mask::default();
        for i in 0..64 {
            for pcm in m[i].iter() {
                mask |= Mask::from_index(pcm.to);
            }
        }
        return mask_to_table(&mask);
    }
    fn masks_to_table(title: &str, masks: &Vec<(&str, &Mask)>) -> stanza::table::Table {
        let mut cols = vec![];
        let mut tbls = vec![];
        for i in 0..masks.len() {
            cols.push(Col::default());
            tbls.push(table_with_title(masks[i].0, mask_to_table(masks[i].1)).into())
        }
        return table_with_title(title, 
            Table::with_styles(Styles::default()).with_cols(
                cols
            ).with_row(
                Row::new(Styles::default(), tbls)
            )
        );
    }
    pub fn value_mask_to_table(title: &str, m: &ValueMask) -> stanza::table::Table {
        return table_with_title(title, Table::with_styles(
            Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
        ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
            let mut cells = Vec::<i8>::new();
            for bit in 0..8 {
                cells.push(m[i * 8 + bit]);
            }
            return Row::from(cells);
        })).into()
        );
    }
    pub fn depth_mask_to_table(m: &DepthMask) -> stanza::table::Table {
        let bv = &m.raw.to_ne_bytes();
        return Table::with_styles(
            Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
        ).with_cols((0..8).map(|_| { Col::new(Styles::default()) }).collect()).with_rows((0..8).map(|i| {
            let mut cells = Vec::<char>::new();
            for bit in 0..8 {
                if bv[i] & (1 << bit) != 0 {
                    if bv[i + 8] & (1 << bit) != 0 {
                        cells.push('2');
                    } else {
                        cells.push('1');
                    }
                } else if bv[i + 8] & (1 << bit) != 0 {
                    if bv[i] & (1 << bit) != 0 {
                        cells.push('2');
                    } else {
                        cells.push('1');
                    }
                } else {
                    cells.push('0');
                }
            }
            return Row::from(cells);
        })).into()
    }
    pub fn mask_to_table(m: &Mask) -> stanza::table::Table {
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
    pub fn eval_to_table(evaluator: &Evaluator) -> stanza::table::Table {
        let mut wt = 0;
        let mut bt = 0;
        return Table::with_styles(
            Styles::default()
        ).with_cols(vec![
        Col::new(Styles::default().with(HAlign::Left).with(MinWidth(10))),
        Col::new(Styles::default().with(HAlign::Right).with(MinWidth(10))),
        Col::new(Styles::default().with(HAlign::Right).with(MinWidth(10))),
        Col::new(Styles::default().with(HAlign::Right).with(MinWidth(10))),
        ]).with_row(Row::new(Styles::default().with(Header(true)), vec!["Term".into(), "White".into(), "Black".into(), "+/-".into()])).with_rows(
        (0..evaluator.scores.len()).map(|i| {
            wt += evaluator.scores[i].white_score;
            bt += evaluator.scores[i].black_score;
            return Row::new(Styles::default(), vec![
                evaluator.scores[i].name.to_string().into(),
                evaluator.scores[i].white_score.to_string().into(),
                evaluator.scores[i].black_score.to_string().into(),
                (evaluator.scores[i].white_score + evaluator.scores[i].black_score).to_string().into(),
            ]);
        })
        ).with_row(Row::new(Styles::default().with(Header(true)), vec!["Total".into(), wt.to_string().into(), bt.to_string().into(), (wt + bt).to_string().into()]))
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

        pub fn pretty_string_evaluator(evaluator: &Evaluator) -> String {
            let renderer = Console({
                let mut decor = Decor::default();
                decor.draw_outer_border = false;
                decor
            });
            return format!("{}", renderer.render(&eval_to_table(evaluator)));
        }
    pub fn pretty_print_value_mask(title: &str, mask: &ValueMask) -> () {
        println!("{}", Console::default().render(&value_mask_to_table(title, &mask)));
    }
    pub fn pretty_print_maskset(title: &str, maskset: &MaskSet) -> () {
        println!("{}", Console::default().render(&maskset_to_table(title, maskset)));
    }
    pub fn pretty_print_masks(title: &str, masks: &Vec<(&str, &Mask)>) -> () {
        println!("{}", Console::default().render(&masks_to_table(title, masks)));
    }
    pub fn pretty_print_mask(mask: &Mask) -> () {
        println!("{}", Console::default().render(&mask_to_table(mask)));
    }
    pub fn pretty_print_moveset(moveset: &[Vec<Motion>; 64]) -> () {
        println!("{}", Console::default().render(&moveset_to_table(moveset)));
    }
    pub fn pretty_print_board(title: &str, board: &[u8; 64]) -> () {
        println!("{}", Console::default().render(&board_to_table(title, board)));
    }
}
