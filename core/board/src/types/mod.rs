pub mod board;
pub mod error;
pub mod position;
pub mod unit_map;

pub use board::Board;
pub use error::*;
pub use position::Pos;
pub use unit_map::UnitMap;

pub type Coord = usize;
pub type UnitId = usize;
