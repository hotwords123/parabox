use std::collections::HashMap;
use color_space::Hsv;

use super::utility::*;

#[derive(Clone, Debug)]
pub struct Game {
    pub(super) cells: Vec<Cell>,
    pub(super) goals: Vec<Goal>,
    pub(super) block_map: HashMap<i32, usize>,
    pub(super) player_ids: Vec<usize>,
}

#[derive(Clone, Debug)]
pub enum Cell {
    Wall(Wall),
    Block(Block),
    Reference(Reference),
}

#[derive(Clone, Debug)]
pub struct Wall {
    pub id: usize,
    pub gpos: GlobalPos,
    pub possessable: bool,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: usize,
    pub gpos: GlobalPos,
    pub block_no: i32,
    pub width: i32,
    pub height: i32,
    pub hsv: Hsv,
    pub filled: bool,
    pub space: bool,
    pub locked: bool,
    pub possessable: bool,
    pub fliph: bool,
}

#[derive(Clone, Debug)]
pub struct Reference {
    pub id: usize,
    pub gpos: GlobalPos,
    pub target_no: i32,
    pub link: ReferenceLink,
    pub exit: bool,
    pub possessable: bool,
    pub fliph: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReferenceLink {
    None,
    InfExit { degree: u32 },
    InfEnter { degree: u32, block_no: i32 },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    pub gpos: GlobalPos,
    pub player: bool,
}

impl Cell {
    pub fn id(&self) -> usize {
        match &self {
            Cell::Wall(wall) => wall.id,
            Cell::Block(block) => block.id,
            Cell::Reference(reference) => reference.id,
        }
    }

    pub fn gpos(&self) -> GlobalPos {
        match &self {
            Cell::Wall(wall) => wall.gpos,
            Cell::Block(block) => block.gpos,
            Cell::Reference(reference) => reference.gpos,
        }
    }

    pub fn possessable(&self) -> bool {
        match &self {
            Cell::Wall(wall) => wall.possessable,
            Cell::Block(block) => block.possessable,
            Cell::Reference(reference) => reference.possessable,
        }
    }

    pub fn fliph(&self) -> bool {
        match &self {
            Cell::Wall(_) => false,
            Cell::Block(block) => block.fliph,
            Cell::Reference(reference) => reference.fliph,
        }
    }

    pub fn is_wall(&self) -> bool {
        match &self {
            Cell::Wall(_) => true,
            _ => false,
        }
    }

    pub fn block(&self) -> Option<&Block> {
        match &self {
            Cell::Block(block) => Some(block),
            _ => None,
        }
    }

    pub fn reference(&self) -> Option<&Reference> {
        match &self {
            Cell::Reference(reference) => Some(reference),
            _ => None,
        }
    }
}

impl Block {
    pub fn in_bounds(&self, Pos(x, y): Pos) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn can_enter(&self) -> bool {
        !self.filled && !self.locked
    }

    pub fn can_exit(&self) -> bool {
        !self.space
    }
}

impl Reference {
    pub fn can_enter(&self) -> bool {
        match self.link {
            ReferenceLink::InfExit { .. } => false,
            _ => true,
        }
    }
}

impl Game {
    const SPACE_SIZE: i32 = 3;
    const SPACE_CENTER: Pos = Pos(Self::SPACE_SIZE, Self::SPACE_SIZE);

    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            goals: Vec::new(),
            block_map: HashMap::new(),
            player_ids: Vec::new(),
        }
    }

    pub fn cells(&self) -> &Vec<Cell> {
        &self.cells
    }

    pub fn goals(&self) -> &Vec<Goal> {
        &self.goals
    }

    pub fn player_ids(&self) -> &Vec<usize> {
        &self.player_ids
    }

    pub fn cell_at(&self, gpos: GlobalPos) -> Option<&Cell> {
        return self.cells.iter().find(|cell| cell.gpos() == gpos);
    }

    fn check_pos(&self, gpos: GlobalPos) -> Result<(), String> {
        if gpos.block_id == usize::MAX {
            Ok(())
        } else {
            let block = self.cells[gpos.block_id].block().unwrap();
            if !block.in_bounds(gpos.pos) {
                Err(format!("Invalid position {:?}", gpos))
            } else if self.cell_at(gpos).is_some() {
                Err(format!("Cell already exists at {:?}", gpos))
            } else {
                Ok(())
            }
        }
    }

    pub fn is_block_trivial(&self, block: &Block) -> bool {
        if block.filled {
            // filled blocks are trivial
            return true;
        }

        for x in 0..block.width {
            for y in 0..block.height {
                let cell = self.cell_at(GlobalPos {
                    block_id: block.id,
                    pos: Pos(x, y),
                });

                if x == 0 || y == 0 || x == block.width - 1 || y == block.height - 1 {
                    // the border should be filled with non-possessable walls
                    if let Some(Cell::Wall(wall)) = cell {
                        if wall.possessable { return false; }
                    } else {
                        return false;
                    }
                } else {
                    // the inside should be empty
                    if cell.is_some() { return false; }
                }
            }
        }

        true
    }

    pub(super) fn add_space(&mut self) -> usize {
        let id = self.cells.len();
        let mut min_no = 0;
        for cell in &self.cells {
            match cell {
                Cell::Block(block) => min_no = min_no.min(block.block_no),
                _ => {}
            }
        }
        self.cells.push(Cell::Block(Block {
            id,
            gpos: GlobalPos { block_id: usize::MAX, pos: Pos(0, 0) },
            block_no: min_no - 1,
            width: 2 * Self::SPACE_SIZE + 1,
            height: 2 * Self::SPACE_SIZE + 1,
            hsv: Hsv::new(0.0, 0.0, 0.5),
            filled: false,
            space: true,
            locked: false,
            possessable: false,
            fliph: false,
        }));
        id
    }

    pub fn block_by_no(&self, block_no: i32) -> Option<&Block> {
        self.block_map.get(&block_no).map(|id| self.cells[*id].block().unwrap())
    }

    pub fn exit_id_for(&self, block: &Block) -> Option<usize> {
        if !block.can_exit() {
            return None;
        }
        for cell in &self.cells {
            if let Cell::Reference(reference) = cell {
                if reference.exit && reference.target_no == block.block_no {
                    return Some(reference.id);
                }
            }
        }
        Some(block.id)
    }

    pub fn inf_exit_id_for(&self, block_no: i32, degree: u32) -> Option<usize> {
        for cell in &self.cells {
            if let Cell::Reference(reference) = cell {
                if reference.target_no == block_no && reference.link == (ReferenceLink::InfExit { degree }) {
                    return Some(reference.id);
                }
            }
        }
        None
    }

    pub fn inf_enter_for(&self, block: &Block, degree: u32) -> Option<&Block> {
        for cell in &self.cells {
            if let Cell::Reference(reference) = cell {
                if reference.link == (ReferenceLink::InfEnter { degree, block_no: block.block_no }) {
                    return self.block_by_no(reference.target_no);
                }
            }
        }
        None
    }

    pub(super) fn add_inf_exit_for(&mut self, block_no: i32, degree: u32) -> usize {
        let gpos = GlobalPos {
            block_id: self.add_space(),
            pos: Self::SPACE_CENTER,
        };
        let id = self.cells.len();
        self.cells.push(Cell::Reference(Reference {
            id,
            gpos,
            target_no: block_no,
            link: ReferenceLink::InfExit { degree },
            exit: false,
            possessable: false,
            fliph: false,
        }));
        id
    }

    /// Headers
    /// ```plain
    /// version 4 (only required item)
    /// attempt_order push,enter,eat,possess (used in Priority area in-game with value "enter,eat,push,possess".)
    /// shed (enables Shed area behavior)
    /// inner_push (enables Inner Push area behavior)
    /// draw_style tui (Text graphics)
    /// draw_style grid (Like tui, but with blocks instead of text)
    /// draw_style oldstyle (Gallery area development graphics)
    /// custom_level_music -1 (-1 means no music)
    /// custom_level_palette -1 (-1 means no palette is applied)
    /// ```
    ///
    /// Objects
    /// ```plain
    /// Block x y id width height hue sat val zoomfactor fillwithwalls player possessable playerorder fliph floatinspace specialeffect
    /// Ref x y id exitblock infexit infexitnum infenter infenternum infenterid player posssessable playerorder fliph floatinspace ///pecialeffect
    /// Wall x y player possessable playerorder
    /// Floor x y type
    /// ```
    pub fn from_str(text: &str) -> Result<Self, String> {
        let mut game = Self::new();

        // whether we're still reading the header
        let mut reading_header = true;

        // cell id
        let mut stack: Vec<usize> = Vec::new();

        // player order -> cell id
        let mut players: Vec<(i32, usize)> = Vec::new();

        let mut process = |line: &str| -> Result<(), String> {
            if line == "#" {
                reading_header = false;
                return Ok(())
            }

            if reading_header {
                let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
                if parts.is_empty() {
                    return Ok(())
                }
                match parts[0] {
                    "version" => {
                        let version = parts[1];
                        if version != "4" {
                            return Err(format!("Unsupported version: {}", version));
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            let parts: Vec<&str> = line.split_ascii_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                return Ok(());
            }

            let depth = line.chars().take_while(|c| *c == '\t').count();
            if depth > stack.len() {
                return Err(format!("Invalid indentation"));
            }
            stack.truncate(depth);

            let parent_id = stack.last().map_or(usize::MAX, |id| *id);

            // println!("{:3} | {}", lineno + 1, line);
            // println!("depth = {}, parent_id = {}", depth, parent_id);

            match parts[0] {
                "Block" => {
                    if parts.len() < 17 {
                        return Err(format!("Invalid block"));
                    }

                    let x = parts[1].parse::<i32>().unwrap();
                    let y = parts[2].parse::<i32>().unwrap();
                    let block_no = parts[3].parse::<i32>().unwrap();
                    let width = parts[4].parse::<i32>().unwrap();
                    let height = parts[5].parse::<i32>().unwrap();

                    let hue = parts[6].parse::<f64>().unwrap();
                    let sat = parts[7].parse::<f64>().unwrap();
                    let val = parts[8].parse::<f64>().unwrap();

                    let filled = parts[10] == "1";
                    let player_order = if parts[11] == "1" {
                        Some(parts[13].parse::<i32>().unwrap())
                    } else {
                        None
                    };
                    let possessable = parts[12] == "1";
                    let fliph = parts[14] == "1";
                    let floating = parts[15] == "1";

                    if !filled && (width <= 0 || height <= 0) {
                        panic!("Invalid block size");
                    }

                    let gpos = if floating {
                        GlobalPos {
                            block_id: game.add_space(),
                            pos: Self::SPACE_CENTER
                        }
                    } else {
                        GlobalPos { block_id: parent_id, pos: Pos(x, y) }
                    };
                    game.check_pos(gpos)?;

                    let id = game.cells.len();
                    game.cells.push(Cell::Block(Block {
                        id,
                        gpos,
                        block_no,
                        width,
                        height,
                        hsv: Hsv::new(360.0 * hue, sat, val),
                        filled,
                        space: false,
                        locked: false,
                        possessable,
                        fliph,
                    }));

                    if let Some(i) = player_order {
                        players.push((i, id));
                    }

                    game.block_map.insert(block_no, id);

                    stack.push(id);
                },

                "Ref" => {
                    if parts.len() < 16 {
                        return Err(format!("Invalid reference"));
                    }

                    let x = parts[1].parse::<i32>().unwrap();
                    let y = parts[2].parse::<i32>().unwrap();
                    let target_no = parts[3].parse::<i32>().unwrap();

                    let mut exit = parts[4] == "1";
                    let link = if parts[5] == "1" {
                        exit = false; // inf exits don't serve as an exit
                        ReferenceLink::InfExit {
                            degree: parts[6].parse::<u32>().unwrap()
                        }
                    } else if parts[7] == "1" {
                        ReferenceLink::InfEnter {
                            degree: parts[8].parse::<u32>().unwrap(),
                            block_no: parts[9].parse::<i32>().unwrap(),
                        }
                    } else {
                        ReferenceLink::None
                    };

                    let player_order = if parts[10] == "1" {
                        Some(parts[12].parse::<i32>().unwrap())
                    } else {
                        None
                    };
                    let possessable = parts[11] == "1";
                    let fliph = parts[13] == "1";
                    let floating = parts[14] == "1";

                    let gpos = if floating {
                        GlobalPos {
                            block_id: game.add_space(),
                            pos: Self::SPACE_CENTER
                        }
                    } else {
                        GlobalPos { block_id: parent_id, pos: Pos(x, y) }
                    };
                    game.check_pos(gpos)?;

                    let id = game.cells.len();
                    game.cells.push(Cell::Reference(Reference {
                        id,
                        gpos,
                        target_no,
                        link,
                        exit,
                        possessable,
                        fliph,
                    }));

                    if let Some(i) = player_order {
                        players.push((i, id));
                    }
                },

                "Wall" => {
                    if parts.len() < 6 {
                        return Err(format!("Invalid wall"));
                    }

                    let x = parts[1].parse::<i32>().unwrap();
                    let y = parts[2].parse::<i32>().unwrap();

                    let player_order = if parts[3] == "1" {
                        Some(parts[5].parse::<i32>().unwrap())
                    } else {
                        None
                    };
                    let possessable = parts[4] == "1";

                    if parent_id == usize::MAX {
                        return Err(format!("Wall outside of block"));
                    }

                    let gpos = GlobalPos { block_id: parent_id, pos: Pos(x, y) };
                    game.check_pos(gpos)?;

                    let id = game.cells.len();
                    game.cells.push(Cell::Wall(Wall {
                        id,
                        gpos,
                        possessable,
                    }));

                    if let Some(i) = player_order {
                        players.push((i, id));
                    }
                },

                "Floor" => {
                    if parts.len() < 4 {
                        return Err(format!("Invalid floor"));
                    }

                    let x = parts[1].parse::<i32>().unwrap();
                    let y = parts[2].parse::<i32>().unwrap();

                    let player = match parts[3] {
                        "Button" => false,
                        "PlayerButton" => true,
                        _ => return Err(format!("Unknown floor type {}", parts[3])),
                    };

                    game.goals.push(Goal {
                        gpos: GlobalPos {
                            block_id: parent_id,
                            pos: Pos(x, y),
                        },
                        player,
                    });
                },

                _ => return Err(format!("Unknown object type {}", parts[0])),
            }

            Ok(())
        };

        for (lineno, line) in text.lines().enumerate() {
            if let Err(e) = process(line) {
                return Err(format!("{}\n{} | {}", e, lineno + 1, line));
            }
        }

        // check if all block_no are valid
        for cell in &game.cells {
            if let Cell::Reference(reference) = cell {
                if game.block_map.get(&reference.target_no).is_none() {
                    return Err(format!("Invalid reference target {}", reference.target_no));
                }
                if let ReferenceLink::InfEnter { block_no, .. } = reference.link {
                    if game.block_map.get(&block_no).is_none() {
                        return Err(format!("Invalid inf enter link {}", block_no));
                    }
                }
            }
        }

        // sort players by order
        players.sort_by_key(|(i, _)| *i);
        game.player_ids.extend(players.iter().map(|(_, id)| *id));

        Ok(game)
    }

    pub fn play(&mut self, direction: Direction) {
        use super::simulation::Simulator;
        let mut simulator = Simulator::new(self);
        simulator.play(direction);
    }

    pub fn won(&self) -> bool {
        for goal in &self.goals {
            let cell = self.cell_at(goal.gpos);
            if cell.is_none() {
                return false;
            }

            let cell = cell.unwrap();
            let player = self.player_ids.contains(&cell.id());
            if player != goal.player {
                return false;
            }
        }
        !self.goals.is_empty()
    }
}
