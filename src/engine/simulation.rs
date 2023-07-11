use super::utility::*;
use super::game::*;

pub struct Simulator<'a> {
    game: &'a mut Game,
    player_index: usize,
}

impl Simulator<'_> {
    pub fn new(game: &mut Game) -> Simulator {
        Simulator {
            game,
            player_index: 0,
        }
    }

    pub fn play(&mut self, direction: Direction) {
        for (i, player_id) in self.game.player_ids.clone().iter().enumerate() {
            self.player_index = i;
            self.try_move(*player_id, direction);
        }
    }

    fn move_cell(&mut self, cell_id: usize, to: GlobalPos) {
        match &mut self.game.cells[cell_id] {
            Cell::Wall(wall) => wall.gpos = to,
            Cell::Block(block) => block.gpos = to,
            Cell::Reference(reference) => reference.gpos = to,
        }
    }

    /// Attempts to move the given cell towards the given direction.
    ///
    /// Returns true if the movement was successful.
    fn try_move(&mut self, cell_id: usize, direction: Direction) -> bool {
        self.try_move_from(cell_id, direction, self.game.cells[cell_id].gpos(), 0.5)
    }

    /// TODO: handle infinite exits
    /// TODO: handle horizontal flips
    /// TODO: handle cycles; maybe check the direction?
    fn try_move_from(&mut self, cell_id: usize, mut direction: Direction, mut gpos: GlobalPos, mut exit_point: f64) -> bool {
        // first, try to move the cell in the given direction
        gpos.pos.go(direction);

        let block: &Block = self.game.cells[gpos.block_id].block().unwrap();
        // if the new position is still in the same block, we're done
        if block.in_bounds(gpos.pos) {
            return self.try_interact_pos(cell_id, direction, gpos, 0.5);
        }

        // otherwise, we need to exit the block
        // first, check if the block can be exited
        let exit = self.game.exit_for(block);
        if exit.is_none() {
            return false;
        }

        // then, find the new exit point
        let exit = exit.unwrap();
        exit_point = match direction {
            Direction::Up | Direction::Down =>
                (gpos.pos.0 as f64 + exit_point) / block.width as f64,
            Direction::Left | Direction::Right =>
                (gpos.pos.1 as f64 + exit_point) / block.height as f64,
        }.clamp(0.0, 1.0);

        // flip the direction if necessary
        if exit.fliph() {
            match direction {
                Direction::Left => direction = Direction::Right,
                Direction::Right => direction = Direction::Left,
                _ => exit_point = 1.0 - exit_point,
            };
            // TODO: flip the cell horizontally
        }

        self.try_move_from(cell_id, direction, exit.gpos(), exit_point)
    }

    /// Attempts to interact with the given position.
    ///
    /// Returns true if the occupation was successful.
    fn try_interact_pos(&mut self, cell_id: usize, direction: Direction, target_gpos: GlobalPos, point: f64) -> bool {
        if let Some(target) = self.game.cell_at(target_gpos) {
            // some cell exists at the target position
            // try to interact with it
            self.try_interact(cell_id, target.id(), direction, point)
        } else {
            // no cell exists at the target position
            // just walk up and take the position
            self.move_cell(cell_id, target_gpos);
            true
        }
    }

    /// Attempts to simulate the interaction between the source cell and the target cell.
    ///
    /// The default attempt order is: push, enter, eat, possess.
    ///
    /// Returns true if the interaction was successful.
    fn try_interact(&mut self, source_id: usize, target_id: usize, direction: Direction, point: f64) -> bool {
        if self.try_push(source_id, target_id, direction) {
            return true;
        }

        if self.try_enter(source_id, target_id, direction, point) {
            return true;
        }

        if self.try_eat(source_id, target_id, direction) {
            return true;
        }

        if self.try_possess(source_id, target_id) {
            return true;
        }

        false
    }

    fn try_push(&mut self, source_id: usize, target_id: usize, direction: Direction) -> bool {
        let target = &self.game.cells[target_id];
        let target_gpos = target.gpos();

        if target.is_wall() {
            return false;
        }

        // try to move the pushee cell
        if self.try_move(target_id, direction) {
            // move the pusher to the new position
            // FIXME: the cell is not necessarily moved in some cycle cases!
            self.move_cell(source_id, target_gpos);
            return true;
        }

        false
    }

    fn try_enter(&mut self, source_id: usize, target_id: usize, mut direction: Direction, mut enter_point: f64) -> bool {
        // TODO: handle infinite enters

        let target = &self.game.cells[target_id];
        let block = match &target {
            Cell::Wall(_) => return false,

            Cell::Block(block) => {
                &block
            },

            Cell::Reference(reference) => {
                if !reference.can_enter() {
                    return false;
                }
                self.game.block_by_no(reference.target_no).unwrap()
            },
        };

        if !block.can_enter() {
            return false;
        }

        // flip the direction if necessary
        if target.fliph() {
            match direction {
                Direction::Left => direction = Direction::Right,
                Direction::Right => direction = Direction::Left,
                _ => enter_point = 1.0 - enter_point,
            };
            // TODO: flip the source horizontally
        }

        // convert the enter point to a coordinate, rounded down
        let mut enter_coord = |side_length: i32| -> i32 {
            // FIXME: deal with floating point errors
            // FIXME: what will happen if the enter point is exactly at the edge?
            enter_point = enter_point * side_length as f64;
            let coord = (enter_point + 1e-9).floor();
            enter_point = (enter_point - coord).clamp(0.0, 1.0);
            coord as i32
        };

        // determine the enter pos
        let enter_gpos = GlobalPos {
            block_id: block.id,
            pos: match direction {
                Direction::Up => Pos(enter_coord(block.width), 0),
                Direction::Down => Pos(enter_coord(block.width), block.height - 1),
                Direction::Left => Pos(block.width - 1, enter_coord(block.height)),
                Direction::Right => Pos(0, enter_coord(block.height)),
            }
        };

        // try to interact with the enter pos
        self.try_interact_pos(source_id, direction, enter_gpos, enter_point)
    }

    fn try_eat(&mut self, source_id: usize, target_id: usize, direction: Direction) -> bool {
        let target = &self.game.cells[target_id];
        let target_gpos = target.gpos();

        if target.is_wall() {
            return false;
        }

        // try to let the eaten cell enter the eater cell
        if self.try_enter(target_id, source_id, direction.opposite(), 0.5) {
            // move the eater to the new position
            self.move_cell(source_id, target_gpos);
            return true;
        }

        false
    }

    fn try_possess(&mut self, source_id: usize, target_id: usize) -> bool {
        // TODO: should we perform extra checks here?

        // only the current player can possess
        if source_id != self.game.player_ids[self.player_index] {
            return false;
        }

        let target = &self.game.cells[target_id];
        if target.possessable() && !self.game.player_ids.contains(&target_id) {
            self.game.player_ids[self.player_index] = target_id;
            return true;
        }

        false
    }
}
