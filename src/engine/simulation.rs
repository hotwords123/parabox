use super::game::*;
use super::utility::*;

pub struct Simulator<'a> {
    game: &'a mut Game,

    // index of the current player
    player_index: usize,

    // cells that has a tendency to move
    // direction is the original position of the cell
    // gpos is the target position of the cell (after the move is scheduled)
    // fliph is the target fliph state of the cell (after the move is scheduled)
    move_stack: Vec<MoveState>,

    // cells in the stack starting from the index can actually be moved
    move_index: usize,

    // cache for transfer actions, used for inf exit/enter detection
    transfer_cache: TransferCache,

    // stack for transfer cache
    transfer_stack: Vec<TransferCache>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct MoveState {
    cell_id: usize,
    direction: Direction,
    gpos: GlobalPos,
    fliph: bool,
}

#[derive(Default)]
struct TransferCache {
    exit: Vec<TransferState>,
    enter: Vec<TransferState>,
}

type TransferPoint = num_rational::Rational32;
// (context_no, direction)
type ExitKey = (BlockNo, Direction);
// (block_no, direction, enter_point)
type EnterKey = (BlockNo, Direction, TransferPoint);

const MIDDLE_POINT: TransferPoint = TransferPoint::new_raw(1, 2);
const ONE_POINT: TransferPoint = TransferPoint::new_raw(1, 1);

struct TransferState {
    block_no: BlockNo,
    direction: Direction,
    point: TransferPoint,
    degree: u32,
    fliph: bool,
}

impl TransferState {
    fn exit_key(&self) -> ExitKey {
        (self.block_no, self.direction)
    }

    fn enter_key(&self) -> EnterKey {
        (self.block_no, self.direction, self.point)
    }
}

impl MoveState {
    fn new(cell: &Cell, direction: Direction) -> MoveState {
        MoveState {
            cell_id: cell.id(),
            direction,
            gpos: cell.gpos(),
            fliph: cell.fliph(),
        }
    }

    fn update(&mut self, other: MoveState) {
        self.gpos = other.gpos;
        self.fliph = other.fliph;
    }

    fn apply(self, game: &mut Game) {
        match &mut game.cells[self.cell_id] {
            Cell::Wall(wall) => {
                wall.gpos = self.gpos;
                wall.fliph = self.fliph;
            }
            Cell::Block(block) => {
                block.gpos = self.gpos;
                block.fliph = self.fliph;
            }
            Cell::Reference(reference) => {
                reference.gpos = self.gpos;
                reference.fliph = self.fliph;
            }
        }
    }
}

impl TransferCache {
    fn clear(&mut self) {
        self.exit.clear();
        self.enter.clear();
    }

    fn try_push_state<F, K>(
        stack: &mut Vec<TransferState>,
        state: TransferState,
        key: F,
    ) -> Option<&mut TransferState>
    where
        F: Fn(&TransferState) -> K,
        K: PartialEq,
    {
        let state_key = key(&state);
        if let Some(i) = stack.iter().position(|s| key(s) == state_key) {
            stack.truncate(i + 1);
            Some(&mut stack[i])
        } else {
            stack.push(state);
            None
        }
    }
}

impl Simulator<'_> {
    pub fn new(game: &mut Game) -> Simulator {
        Simulator {
            game,
            player_index: 0,
            move_stack: Vec::new(),
            move_index: 0,
            transfer_cache: Default::default(),
            transfer_stack: Vec::new(),
        }
    }

    pub fn play(&mut self, direction: Direction) {
        for i in 0..self.game.player_ids.len() {
            self.player_index = i;
            if self.try_move(self.game.player_ids[i], direction) {
                for state in &self.move_stack[self.move_index..] {
                    state.apply(self.game);
                }
            }
            self.move_stack.clear();
            self.move_index = 0;
            self.transfer_stack.clear();
            self.transfer_cache.clear();
        }
    }

    /// Checks whether the given cell is already in the move stack, that is, a
    /// cycle exists.
    ///
    /// If no cycle exists, returns `None`.
    ///
    /// If a cycle exists, and the cells in it can move together, returns the
    /// index of the cell in the move stack, i.e. the new `move_index`.
    fn check_cycle(&self, cell_id: usize, direction: Direction) -> Option<Result<usize, ()>> {
        if let Some(i) = self.move_stack.iter().position(|s| s.cell_id == cell_id) {
            // The cell is already in the move stack, which means that the cell
            // is in a cycle.
            if i >= self.move_index && self.move_stack[i].direction == direction {
                // The cell is moving in the same direction as before, so the
                // cells in the cycle can move together.
                Some(Ok(i))
            } else {
                // The cell is moving in a different direction, so the cells in
                // the cycle cannot move together.
                Some(Err(()))
            }
        } else {
            None
        }
    }

    /// Starts a new move and push it to the move stack. Also pushes the old
    /// transfer cache to the transfer stack.
    ///
    /// Returns the new move state.
    fn push_move(&mut self, cell_id: usize, direction: Direction) -> MoveState {
        let current = MoveState::new(&self.game.cells[cell_id], direction);
        self.move_stack.push(current);
        self.transfer_stack
            .push(std::mem::take(&mut self.transfer_cache));
        current
    }

    /// Pops the last move from the move stack, restoring the transfer cache.
    fn pop_move(&mut self) {
        self.move_stack.pop();
        self.transfer_cache = self.transfer_stack.pop().unwrap();
    }

    /// Attempts to move the given cell towards the given direction.
    ///
    /// Returns true if the movement was successful.
    fn try_move(&mut self, cell_id: usize, direction: Direction) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_move: {:?} {:?}", cell_id, direction);

        match self.check_cycle(cell_id, direction) {
            Some(Ok(i)) => {
                // The cell is in a cycle, and the cells in the cycle can move
                // together. So we can just update the move index.
                self.move_index = i;
                return true;
            }
            Some(Err(())) => return false,
            None => (),
        }

        let current = self.push_move(cell_id, direction);
        if current.gpos.block_id != usize::MAX && self.try_exit(current, MIDDLE_POINT) {
            true
        } else {
            self.pop_move();
            false
        }
    }

    fn try_exit(&mut self, mut current: MoveState, mut exit_point: TransferPoint) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_exit: {:?} {:?}", current, exit_point);

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
            Direction::Up | Direction::Down => (exit_point + current.gpos.pos.0) / block.width,
            Direction::Left | Direction::Right => (exit_point + current.gpos.pos.1) / block.height,
        };

        let context_no = match exit {
            Cell::Block(block) => block.block_no,
            Cell::Reference(reference) => reference.target_no,
            _ => unreachable!("exit should be a block or reference"),
        };
        let state = TransferState {
            block_no: context_no,
            direction: current.direction,
            point: exit_point,
            degree: 0,
            fliph: current.fliph,
        };

        if let Some(state) = TransferCache::try_push_state(
            &mut self.transfer_cache.exit,
            state,
            TransferState::exit_key,
        ) {
            // this is an infinite exit
            let inf_exit_id = self
                .game
                .inf_exit_id_for(context_no, state.degree)
                .unwrap_or_else(|| self.game.add_inf_exit_for(context_no, state.degree));

            // redirect the exit to the inf exit
            exit = &self.game.cells[inf_exit_id];
            exit_point = state.point;
            current.fliph = state.fliph;

            // increase the degree next time
            state.degree += 1;
        }

        // this step is necessary because the exit might be redirected
        let exit_id = exit.id();

        // flip the direction if necessary
        if exit.fliph() {
            match current.direction {
                Direction::Left => current.direction = Direction::Right,
                Direction::Right => current.direction = Direction::Left,
                _ => exit_point = ONE_POINT - exit_point,
            };
            current.fliph = !current.fliph;
        }

        // try again from the new exit
        current.gpos = exit.gpos();
        if self.try_exit(current, exit_point) {
            return true;
        }

        if self.game.config.shed {
            self.move_stack.last_mut().unwrap().update(current);

            if self.try_move(exit_id, current.direction.opposite()) {
                return true;
            }
        }

        false
    }

    /// Attempts to interact with the given position.
    ///
    /// Returns true if the occupation was successful.
    fn try_interact_pos(&mut self, current: MoveState, point: TransferPoint) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_interact_pos: {:?} {:?}", current, point);

        if let Some(target) = self.game.cell_at(current.gpos) {
            // some cell exists at the target position
            // try to interact with it
            self.try_interact(current, target.id(), point)
        } else {
            // no cell exists at the target position
            // just walk up and take the position
            self.move_stack.last_mut().unwrap().update(current);
            true
        }
    }

    /// Attempts to simulate the interaction between the source cell and the target cell.
    ///
    /// The default attempt order is: push, enter, eat, possess.
    ///
    /// Returns true if the interaction was successful.
    fn try_interact(&mut self, current: MoveState, target_id: usize, point: TransferPoint) -> bool {
        self.game
            .config
            .attempt_order
            .clone()
            .iter()
            .any(|action_type| match action_type {
                ActionType::Push => self.try_push(current, target_id),
                ActionType::Enter => {
                    let moving = &self.move_stack[self.move_index..];
                    if moving.iter().any(|s| s.cell_id == target_id) {
                        // entering a moving cell is not allowed
                        return false;
                    }
                    self.try_enter(current, target_id, point)
                }
                ActionType::Eat => self.try_eat(current, target_id),
                ActionType::Possess => self.try_possess(current.cell_id, target_id),
            })
    }

    fn try_push(&mut self, current: MoveState, target_id: usize) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_push: {:?} {:?}", current, target_id);

        // move the pusher to the new position
        self.move_stack.last_mut().unwrap().update(current);

        let target = &self.game.cells[target_id];
        if target.is_wall() {
            // walls can be pushed in a cycle (typically when the wall is possessed)
            if let Some(Ok(i)) = self.check_cycle(target_id, current.direction) {
                // The wall is in a cycle, and the cells in the cycle can move
                // together. So we can just update the move index.
                self.move_index = i;
                return true;
            }

            if self.game.config.inner_push {
                // try to move the parent block of the wall
                let parent = self.game.cells[target.gpos().block_id].block().unwrap();
                if let Some(exit_id) = self.game.exit_id_for(parent) {
                    // even if the inner push succeeds, previous movements cannot be made
                    let old_move_index = self.move_index;
                    self.move_index = self.move_stack.len();

                    let exit = &self.game.cells[exit_id];
                    let mut direction = current.direction;
                    if exit.fliph() {
                        // flip the direction if necessary
                        match direction {
                            Direction::Left => direction = Direction::Right,
                            Direction::Right => direction = Direction::Left,
                            _ => (),
                        };
                    }

                    if self.try_move(exit_id, direction) {
                        return true;
                    }

                    // restore previous movements
                    self.move_index = old_move_index;
                }
            }
            return false;
        }

        // try to move the pushee cell
        self.try_move(target_id, current.direction)
    }

    fn try_enter(
        &mut self,
        mut current: MoveState,
        target_id: usize,
        mut enter_point: TransferPoint,
    ) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_enter: {:?} {:?} {:?}", current, target_id, enter_point);

        let target = &self.game.cells[target_id];
        let mut block = match &target {
            Cell::Wall(_) => return false,
            Cell::Block(block) => {
                if self.game.is_space(block.gpos.block_id) {
                    return false;
                }
                block
            }
            Cell::Reference(reference) => {
                if !reference.can_enter() || self.game.is_space(reference.gpos.block_id) {
                    return false;
                }
                self.game.block_by_no(reference.target_no).unwrap()
            }
        };

        if !block.can_enter() {
            return false;
        }

        // flip the direction if necessary
        if target.fliph() {
            match current.direction {
                Direction::Left => current.direction = Direction::Right,
                Direction::Right => current.direction = Direction::Left,
                _ => enter_point = ONE_POINT - enter_point,
            };
        }

        // check for infinite enter
        let state = TransferState {
            block_no: block.block_no,
            direction: current.direction,
            point: enter_point,
            degree: 0,
            fliph: current.fliph,
        };

        if let Some(state) = TransferCache::try_push_state(
            &mut self.transfer_cache.enter,
            state,
            TransferState::enter_key,
        ) {
            // this is an infinite enter
            let inf_enter_id = self
                .game
                .inf_enter_id_for(block, state.degree)
                .unwrap_or_else(|| self.game.add_inf_enter_for(state.block_no, state.degree));

            // redirect to the inf enter block
            block = self.game.cells[inf_enter_id].block().unwrap();
            enter_point = MIDDLE_POINT;
            state.point = MIDDLE_POINT;
            current.fliph = state.fliph;

            // increase the degree next time
            state.degree += 1;
        } else {
            if target.fliph() {
                current.fliph = !current.fliph;
            }
        }

        // convert the enter point to a coordinate, rounded down
        let mut enter_coord = |side_length: i32| -> i32 {
            enter_point *= side_length;
            let coord = enter_point.to_integer();
            enter_point -= coord;
            coord
        };

        // determine the enter pos
        current.gpos = GlobalPos {
            block_id: block.id,
            pos: match current.direction {
                Direction::Up => Pos(enter_coord(block.width), 0),
                Direction::Down => Pos(enter_coord(block.width), block.height - 1),
                Direction::Left => Pos(block.width - 1, enter_coord(block.height)),
                Direction::Right => Pos(0, enter_coord(block.height)),
            },
        };

        // try to interact with the enter pos
        self.try_interact_pos(current, enter_point)
    }

    fn try_eat(&mut self, current: MoveState, target_id: usize) -> bool {
        // print!("{}", "  ".repeat(self.move_stack.len()));
        // println!("try_eat: {:?} {:?}", current, target_id);

        let target = &self.game.cells[target_id];
        if target.is_wall() {
            return false;
        }

        // cycles are not allowed in eat
        if self.move_stack.iter().any(|s| s.cell_id == target_id) {
            return false;
        }

        // move the eater to the new position
        self.move_stack.last_mut().unwrap().update(current);

        // try to let the eaten cell enter the eater cell
        let mut eaten = self.push_move(target_id, current.direction.opposite());

        // The eat process can be divided into two parts:
        //
        // 1. the eaten cell enters the eater cell;
        // 2. the eater cell takes the original position of the eaten cell.
        //
        // Ideally, the two parts should happen simultaneously. However, in
        // order to check whether an eat process can happen, it is simpler to
        // check part 2 first. Meanwhile, when it comes to actually carrying
        // out the process, step 1 should be simulated first, since the
        // prerequisite of step 2 is that the original position of the eaten
        // cell is empty, and step 1 will make sure of that.
        //
        // When step 1 is simulated, the eaten cell might need to exit and
        // enter blocks first before it can enter the eater cell, so we need to
        // consider the fliph state during the transfer.
        //
        // If the eater cell's fliph changes during step 2, then we can infer
        // that the eaten cells' fliph will also change during step 1. In this
        // case, we need to flip the direction before letting the eaten cell
        // enter the eater cell.
        if current.fliph != self.game.cells[current.cell_id].fliph() {
            match eaten.direction {
                Direction::Left => eaten.direction = Direction::Right,
                Direction::Right => eaten.direction = Direction::Left,
                _ => (),
            }
            eaten.fliph = !eaten.fliph;
        }

        if self.try_enter(eaten, current.cell_id, MIDDLE_POINT) {
            true
        } else {
            self.pop_move();
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
            // no cells can be moved
            self.move_index = self.move_stack.len();
            return true;
        }

        false
    }
}
