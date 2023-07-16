use std::{fs, ffi::OsStr};
use parabox::engine::*;

fn scan_level_dir(path: &str) {
    fs::read_dir(path).unwrap().for_each(|entry| {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            scan_level_dir(path.to_str().unwrap());
        } else if path.extension() == Some(OsStr::new("txt")) {
            let solution_path = path.with_extension("solution");
            if solution_path.is_file() {
                println!("Testing level: {:?}", path.file_stem().unwrap());

                let text = fs::read_to_string(path).unwrap();
                let mut game = Game::from_str(&text).unwrap();
                let solution = fs::read_to_string(solution_path).unwrap();

                for c in solution.chars() {
                    assert!(!game.won(), "should not win now");

                    match c {
                        'U' => game.play(Direction::Up),
                        'D' => game.play(Direction::Down),
                        'L' => game.play(Direction::Left),
                        'R' => game.play(Direction::Right),
                        ' ' | '\n' => (),
                        _ => panic!("Invalid solution character: {}", c),
                    }
                }

                assert!(game.won(), "should win now");
            }
        }
    });
}

#[test]
fn test_levels() {
    scan_level_dir("levels");
}
