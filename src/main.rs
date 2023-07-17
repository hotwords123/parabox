use std::io::{Write, BufWriter};
use crossterm::{
    cursor, event, style::{self, Stylize}, terminal,
    QueueableCommand
};
use color_space::{ToRgb, Hsv};
use parabox::engine::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).unwrap();

    let mut history = vec![Game::parse(&text).unwrap()];

    // execute the startup sequence
    if let Some(sequence) = args.get(2) {
        let game = history.last_mut().unwrap();
        for c in sequence.chars() {
            let direction = match c {
                'U' => Direction::Up,
                'D' => Direction::Down,
                'L' => Direction::Left,
                'R' => Direction::Right,
                ' ' => continue,
                _ => panic!("invalid sequence character: {c}"),
            };
            game.play(direction);
        }
    }

    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout);
    render(history.last().unwrap(), &mut writer).unwrap();

    let mut repaint = true;

    loop {
        let event = event::read();
        if let event::Event::Key(event) = event.unwrap() {
            if event.kind == event::KeyEventKind::Press {
                let mut play = |direction: Direction| {
                    let mut game = history.last().unwrap().clone();
                    game.play(direction);
                    history.push(game);
                };

                match event.code {
                    event::KeyCode::Char('w') => play(Direction::Up),
                    event::KeyCode::Char('a') => play(Direction::Left),
                    event::KeyCode::Char('s') => play(Direction::Down),
                    event::KeyCode::Char('d') => play(Direction::Right),
                    event::KeyCode::Char('r') => history.push(history.first().unwrap().clone()),
                    event::KeyCode::Char('z') => {
                        if history.len() > 1 {
                            history.pop();
                        }
                    },
                    event::KeyCode::Char('p') => {
                        debug(history.last().unwrap());
                        continue;
                    },
                    event::KeyCode::Char('e') => repaint = !repaint,
                    event::KeyCode::Char('q') => break,
                    _ => continue,
                }

                let game = history.last().unwrap();
                if repaint {
                    render(game, &mut writer).unwrap();
                }
                if game.won() {
                    println!("You won!");
                    break;
                }
            }
        }
    }
}

fn debug(game: &Game) {
    for cell in game.cells() {
        println!("{cell:?}");
    }
}

fn color_from_hsv(hsv: Hsv) -> style::Color {
    let rgb = hsv.to_rgb();
    style::Color::Rgb { r: rgb.r as u8, g: rgb.g as u8, b: rgb.b as u8 }
}

fn block_no_to_char(block_no: BlockNo) -> char {
    "0123456789ABCDEF".chars().nth(block_no.0 as usize).unwrap_or('G')
}

fn render(game: &Game, out: &mut impl Write) -> crossterm::Result<()> {
    out.queue(terminal::Clear(terminal::ClearType::All))?;

    const WIDTH: u16 = 19;
    const HEIGHT: u16 = 16;
    const COLUMNS: u16 = 8;
    let mut counter = 0u16;

    for block in game.cells().iter().filter_map(|cell| cell.block()) {
        if game.is_block_trivial(block) {
            continue;
        }

        let area_x = WIDTH * (counter % COLUMNS);
        let area_y = HEIGHT * (counter / COLUMNS);
        let padding_x = (WIDTH - block.width as u16) / 2;
        let padding_y = (HEIGHT - 1 - block.height as u16) / 2;
        let offset_x = area_x + padding_x;
        let offset_y = area_y + padding_y;

        counter += 1;

        let color = color_from_hsv(block.hsv);
        let title = format!("[{}]", block_no_to_char(block.block_no));

        out
            .queue(cursor::MoveTo(
                area_x + (WIDTH - title.len() as u16) / 2,
                offset_y
            ))?
            .queue(style::PrintStyledContent(title.with(color)))?;

        for y in (0..block.height).rev() {
            out.queue(cursor::MoveTo(
                offset_x,
                offset_y + (block.height - y) as u16
            ))?;

            for x in 0..block.width {
                let gpos = GlobalPos { block_id: block.id, pos: Pos(x, y) };

                let mut color = color;
                let mut inverted = false;
                let mut underlined = false;
                let mark = if let Some(cell) = game.cell_at(gpos) {
                    match &cell {
                        Cell::Wall(_) => '#',
                        Cell::Block(block) => {
                            color = color_from_hsv(block.hsv);

                            if block.fliph {
                                underlined = true;
                            }

                            if game.player_ids().contains(&block.id) {
                                'p'
                            } else if game.is_block_trivial(block) {
                                'b'
                            } else {
                                if let Some(exit_id) = game.exit_id_for(block) {
                                    inverted = exit_id != block.id;
                                }
                                block_no_to_char(block.block_no)
                            }
                        },
                        Cell::Reference(reference) => {
                            let target_no = reference.target_no;
                            let target = game.block_by_no(target_no).unwrap();
                            color = color_from_hsv(target.hsv);

                            if reference.fliph {
                                underlined = true;
                            }

                            if let Some(degree) = reference.inf_exit {
                                "IJKLMN".chars().nth(degree as usize).unwrap_or('O')
                            } else {
                                inverted = !reference.exit;
                                block_no_to_char(target_no)
                            }
                        },
                    }
                } else {
                    match game.goals().iter().find(|goal| goal.gpos == gpos) {
                        Some(goal) => {
                            color = style::Color::White;
                            if goal.player { '=' } else { '_' }
                        },
                        None => {
                            color = style::Color::Grey;
                            '.'
                        }
                    }
                };

                let mut content = mark.with(color);
                if inverted {
                    content = content.negative();
                }
                if underlined {
                    content = content.underlined();
                }
                out.queue(style::PrintStyledContent(content))?;
            }
        }
    }

    let row_count = (counter + COLUMNS - 1) / COLUMNS;
    out.queue(cursor::MoveTo(0, HEIGHT * row_count))?;
    out.flush()
}
