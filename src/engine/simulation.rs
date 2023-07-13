use super::utility::*;
use super::game::*;
use std::collections::HashMap;

pub struct Simulator<'a> {
    game: &'a mut Game,
    player_index: usize,
    // (cell_id, direction, to)
    move_stack: Vec<MoveState>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct MoveState {
    cell_id: usize,
    gpos: GlobalPos,
    fliph: bool,
    direction: Direction,
}

type ExitKey = (i32, Direction);

struct ExitState {
    exit_point: f64,
    degree: u32,
    fliph: bool,
}

impl MoveState {
    fn new(cell: &Cell, direction: Direction) -> MoveState {
        MoveState {
            cell_id: cell.id(),
            gpos: cell.gpos(),
            fliph: cell.fliph(),
            direction,
        }
    }

    fn apply(self, game: &mut Game) {
        match &mut game.cells[self.cell_id] {
            Cell::Wall(wall) => {
                wall.gpos = self.gpos;
            },
            Cell::Block(block) => {
                block.gpos = self.gpos;
                block.fliph = self.fliph;
            },
            Cell::Reference(reference) => {
                reference.gpos = self.gpos;
                reference.fliph = self.fliph;
            },
        }
    }
}

impl Simulator<'_> {
    pub fn new(game: &mut Game) -> Simulator {
        Simulator {
            game,
            player_index: 0,
            move_stack: Vec::new(),
        }
    }

    pub fn play(&mut self, direction: Direction) {
        for (i, player_id) in self.game.player_ids.clone().iter().enumerate() {
            self.player_index = i;
            self.move_stack.clear();
            self.try_move(*player_id, direction);
        }
    }

    /// Attempts to move the given cell towards the given direction.
    ///
    /// Returns true if the movement was successful.
    fn try_move(&mut self, cell_id: usize, direction: Direction) -> bool {
        if let Some(i) = self.move_stack.iter().position(|s| s.cell_id == cell_id) {
            // the cell is already in the move stack
            // this means that the cell is in a cycle
            let previous = self.move_stack[i];
            if previous.direction == direction {
                // the cell is moving in the same direction as before
                // so the cells in the cycle can be moved together
                for new_state in &self.move_stack[i..] {
                    new_state.apply(self.game);
                }
                return true;
            } else {
                // otherwise, the cell cannot be moved
                return false;
            }
        }

        let mut current = MoveState::new(&self.game.cells[cell_id], direction);
        let mut exit_point = 0.5;

        // (context_no, direction) -> (exit_point, degree)
        let mut exit_state: HashMap<ExitKey, ExitState> = HashMap::new();

        loop {
            // first, try to move the cell in the given direction
            current.gpos.pos.go(current.direction);

            let block: &Block = self.game.cells[current.gpos.block_id].block().unwrap();
            // if the new position is still in the same block, we're done
            if block.in_bounds(current.gpos.pos) {
                return self.try_interact_pos(current, exit_point);
            }

            // otherwise, we need to exit the block
            // first, check if the block can be exited
            let exit_id = self.game.exit_id_for(block);
            if exit_id.is_none() {
                return false;
            }
            let mut exit = &self.game.cells[exit_id.unwrap()];

            // find the new exit point
            exit_point = match current.direction {
                Direction::Up | Direction::Down =>
                    (current.gpos.pos.0 as f64 + exit_point) / block.width as f64,
                Direction::Left | Direction::Right =>
                    (current.gpos.pos.1 as f64 + exit_point) / block.height as f64,
            }.clamp(0.0, 1.0);

            let context_no = match exit {
                Cell::Block(block) => block.block_no,
                Cell::Reference(reference) => reference.target_no,
                _ => unreachable!("exit should be a block or reference"),
            };
            let exit_key = (context_no, current.direction);

            if let Some(state) = exit_state.get_mut(&exit_key) {
                // this is an infinite exit
                let inf_exit_id = self.game.inf_exit_id_for(context_no, state.degree)
                    .unwrap_or_else(|| self.game.add_inf_exit_for(context_no, state.degree));

                // redirect the exit to the inf exit
                exit = &self.game.cells[inf_exit_id];
                exit_point = state.exit_point;
                current.fliph = state.fliph;

                // increase the degree next time
                state.degree += 1;
            } else {
                // this is a normal exit, record it in the map
                exit_state.insert(exit_key, ExitState { exit_point, degree: 0, fliph: current.fliph });
            }

            // flip the direction if necessary
            if exit.fliph() {
                match current.direction {
                    Direction::Left => current.direction = Direction::Right,
                    Direction::Right => current.direction = Direction::Left,
                    _ => exit_point = 1.0 - exit_point,
                };
                current.fliph = !current.fliph;
            }

            // try again from the new exit
            current.gpos = exit.gpos();
        }
    }

    /// Attempts to interact with the given position.
    ///
    /// Returns true if the occupation was successful.
    fn try_interact_pos(&mut self, current: MoveState, point: f64) -> bool {
        if let Some(target) = self.game.cell_at(current.gpos) {
            // some cell exists at the target position
            // try to interact with it
            self.try_interact(current, target.id(), point)
        } else {
            // no cell exists at the target position
            // just walk up and take the position
            current.apply(self.game);
            for new_state in &self.move_stack {
                new_state.apply(self.game);
            }
            true
        }
    }

    /// Attempts to simulate the interaction between the source cell and the target cell.
    ///
    /// The default attempt order is: push, enter, eat, possess.
    ///
    /// Returns true if the interaction was successful.
    fn try_interact(&mut self, current: MoveState, target_id: usize, point: f64) -> bool {
        if self.try_push(current, target_id) {
            return true;
        }

        if self.try_enter(current, target_id, point) {
            return true;
        }

        if self.try_eat(current, target_id) {
            return true;
        }

        if self.try_possess(current.cell_id, target_id) {
            return true;
        }

        false
    }

    fn try_push(&mut self, current: MoveState, target_id: usize) -> bool {
        let target = &self.game.cells[target_id];
        if target.is_wall() {
            return false;
        }

        // move the pusher to the new position
        self.move_stack.push(current);

        // try to move the pushee cell
        if self.try_move(target_id, current.direction) {
            true
        } else {
            self.move_stack.pop();
            false
        }
    }

    fn try_enter(&mut self, mut current: MoveState, target_id: usize, mut enter_point: f64) -> bool {
        // TODO: handle infinite enters

        let target = &self.game.cells[target_id];
        let block = match &target {
            Cell::Wall(_) => return false,

            Cell::Block(block) => {
                if block.locked {
                    return false;
                }
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

        let fliph = match self.move_stack.iter().find(|s| s.cell_id == target_id) {
            Some(state) => state.fliph,
            None => target.fliph(),
        };

        // flip the direction if necessary
        if fliph {
            match current.direction {
                Direction::Left => current.direction = Direction::Right,
                Direction::Right => current.direction = Direction::Left,
                _ => enter_point = 1.0 - enter_point,
            };
            current.fliph = !current.fliph;
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
        current.gpos = GlobalPos {
            block_id: block.id,
            pos: match current.direction {
                Direction::Up => Pos(enter_coord(block.width), 0),
                Direction::Down => Pos(enter_coord(block.width), block.height - 1),
                Direction::Left => Pos(block.width - 1, enter_coord(block.height)),
                Direction::Right => Pos(0, enter_coord(block.height)),
            }
        };

        // try to interact with the enter pos
        self.try_interact_pos(current, enter_point)
    }

    fn try_eat(&mut self, current: MoveState, target_id: usize) -> bool {
        let target = &self.game.cells[target_id];
        if target.is_wall() {
            return false;
        }

        // move the eater to the new position
        self.move_stack.push(current);

        // try to let the eaten cell enter the eater cell
        let eaten = MoveState::new(target, current.direction.opposite());
        if self.try_enter(eaten, current.cell_id, 0.5) {
            true
        } else {
            self.move_stack.pop();
            false
        }
    }

    fn try_possess(&mut self, source_id: usize, target_id: usize) -> bool {
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
