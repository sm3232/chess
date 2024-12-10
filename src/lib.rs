#![feature(unbounded_shifts)]
#![feature(array_chunks)]
#![feature(slice_from_ptr_range)]
#![feature(slice_as_array)]
pub mod lib {
    pub mod cutil;
    pub mod ui;
    pub mod game;
    pub mod heap;
    pub mod eval;
    pub mod mask;
    pub mod point;
    pub mod state;
    pub mod piece;
    pub mod motion;
    pub mod zobrist;
    pub mod maskset;
    pub mod chessbyte;
    pub mod boardarray;
    pub mod searchtree;
    pub mod player;
    pub mod manager;
    pub mod searcher;
}
