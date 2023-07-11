use std::io::{Write, BufWriter};
use crossterm::{
    cursor, event, style::{self, Stylize}, terminal,
    QueueableCommand
};
use color_space::{ToRgb, Hsv};
use parabox::engine::*;

fn main() {
    let level_file = "levels/enter.txt";
    let text = std::fs::read_to_string(level_file).unwrap();

    let mut game = Game::from_str(&text).unwrap();

    // let sequence = "RUUUL URRRR RRDRU UUUDD DDDDL LLL";
    // for c in sequence.chars() {
    //     match c {
    //         'U' => game.play(Direction::Up),
    //         'D' => game.play(Direction::Down),
    //         'L' => game.play(Direction::Left),
    //         'R' => game.play(Direction::Right),
    //         _ => (),
    //     }
    // }

    (|| {
        let mut stdout = BufWriter::new(std::io::stdout());
        render(&game, &mut stdout).unwrap();

        loop {
            let event = event::read();
            if let event::Event::Key(event) = event.unwrap() {
                if event.kind == event::KeyEventKind::Press {
                    match event.code {
                        event::KeyCode::Char('w') => game.play(Direction::Up),
                        event::KeyCode::Char('a') => game.play(Direction::Left),
                        event::KeyCode::Char('s') => game.play(Direction::Down),
                        event::KeyCode::Char('d') => game.play(Direction::Right),
                        event::KeyCode::Char('r') => {
                            game = Game::from_str(&text).unwrap();
                        },
                        event::KeyCode::Char('p') => {
                            debug(&game);
                            continue;
                        },
                        event::KeyCode::Char('q') => break,
                        _ => continue,
                    }

                    render(&game, &mut stdout).unwrap();

                    if game.won() {
                        println!("You won!");
                        break;
                    }
                }
            }
        }
    })();
}

fn debug(game: &Game) {
    for cell in game.cells() {
        println!("{:?}", cell);
    }
}

fn color_from_hsv(hsv: Hsv) -> style::Color {
    let rgb = hsv.to_rgb();
    style::Color::Rgb { r: rgb.r as u8, g: rgb.g as u8, b: rgb.b as u8 }
}

fn render(game: &Game, out: &mut impl Write) -> crossterm::Result<()> {
    out.queue(terminal::Clear(terminal::ClearType::All))?;

    const WIDTH: u16 = 19;
    const HEIGHT: u16 = 12;
    const COLUMNS: u16 = 5;
    let mut counter = 0u16;

    for cell in game.cells() {
        let block = cell.block();
        if !block.is_some() {
            continue;
        }

        let block = block.unwrap();
        if block.filled {
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
        let title = format!("[{}]", block.block_no);

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
                let mark = if let Some(cell) = game.cell_at(gpos) {
                    match &cell {
                        Cell::Wall(_) => '#',
                        Cell::Block(block) => {
                            color = color_from_hsv(block.hsv);

                            if game.player_ids().contains(&block.id) {
                                'P'
                            } else if block.filled {
                                'B'
                            } else {
                                if let Some(exit) = game.exit_for(block) {
                                    inverted = exit.id() != block.id;
                                }
                                "0123456789ABCDEF".chars().nth(block.block_no as usize).unwrap_or('G')
                            }
                        },
                        Cell::Reference(reference) => {
                            let target_no = reference.target_no;
                            let target = game.block_by_no(target_no).unwrap();
                            color = color_from_hsv(target.hsv);
                            match reference.link {
                                ReferenceLink::InfExit { degree } => {
                                    "IJKLMNOPQRST".chars().nth(degree as usize).unwrap_or('U')
                                },
                                ReferenceLink::InfEnter { degree, .. } => {
                                    "ijklmnopqrst".chars().nth(degree as usize).unwrap_or('u')
                                },
                                ReferenceLink::None => {
                                    inverted = !reference.exit;
                                    "0123456789ABCDEF".chars().nth(target_no as usize).unwrap_or('G')
                                },
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

                let mut content = mark.to_string().with(color);
                if inverted {
                    content = content.negative();
                }
                out.queue(style::PrintStyledContent(content))?;
            }
        }
    }

    let row_count = (counter + COLUMNS - 1) / COLUMNS;
    out.queue(cursor::MoveTo(0, HEIGHT * row_count))?;
    out.flush()
}
