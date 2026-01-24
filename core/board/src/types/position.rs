use crate::types::Coord;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Pos {
    pub x: Coord,
    pub y: Coord,
}
