#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Pos(pub i32, pub i32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GlobalPos {
    pub block_id: usize,
    pub pos: Pos,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Pos {
    pub fn go(&mut self, direction: Direction) {
        match direction {
            Direction::Up => self.1 += 1,
            Direction::Down => self.1 -= 1,
            Direction::Left => self.0 -= 1,
            Direction::Right => self.0 += 1,
        }
    }

    pub fn towards(&self, direction: Direction) -> Pos {
        let mut pos = self.clone();
        pos.go(direction);
        pos
    }
}

impl Direction {
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}
