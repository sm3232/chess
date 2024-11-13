use std::{cell::RefCell, rc::Rc};

use mask::Mask;

pub mod draw;
pub mod piece;
pub mod game;
pub mod point;
pub mod mask;

use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{BorderFg, HAlign, Header, MaxWidth, MinWidth, Styles};
use stanza::table::{Col, Row, Table};


fn piece_to_table(p: Rc<RefCell<Box<dyn piece::TPiece>>>) -> Table {
    return Table::with_styles(
        Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
    ).with_cols
}
pub fn pretty_print_future(using: Rc<RefCell<Box<dyn piece::TPiece>>>, usings_move: &Mask, other_piece: Rc<RefCell<Box<dyn piece::TPiece>>>, others_moves: &Mask){
    let mut frame = Table::with_styles(
        Styles::default()
    ).with_cols(vec![
        Col::default(),
        Col::default()
    ]).with_row(Row::from(["With this piece", "Col 2", "Col 3"])).with_row(
        Row::new(Styles::default(), vec![
            piece_to_table(using).into()
        ])

    );
    let mut table = Table::with_styles(
        Styles::default().with(MinWidth(3)).with(MaxWidth(3)).with(HAlign::Centred)
    ).with_cols((0..8).map(|_col| {
        Col::new(Styles::default())
    }).collect());

    let rows: Vec<Row> = (0..8).map(|_row| {
        let mut cells: Vec<stanza::table::Cell> = Vec::new();
        for col in 0..8 {
            cells.push(col.into());
        }
        Row::new(Styles::default(), cells)
    }).collect();

    table.push_rows(rows);
    println!("{}", Console::default().render(&table));


}
