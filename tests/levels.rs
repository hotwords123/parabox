use std::{fs, ffi::OsStr};
use parabox::engine::*;

struct LevelResult {
    level_name: String,
    message: String,
}

fn test_level(path: &std::path::PathBuf) -> Result<(), String> {
    let solution_path = path.with_extension("solution");
    if !solution_path.is_file() {
        return Ok(());
    }

    let text = fs::read_to_string(path).unwrap();
    let mut game = Game::from_str(&text).unwrap();

    let solution = fs::read_to_string(&solution_path).unwrap();

    let mut steps = 0;

    for c in solution.chars() {
        let direction = match c {
            'U' => Direction::Up,
            'D' => Direction::Down,
            'L' => Direction::Left,
            'R' => Direction::Right,
            ' ' | '\n' => continue,
            _ => return Err(format!("invalid solution character: {c}")),
        };

        if game.won() {
            return Err(format!("should not win now after {steps} steps"));
        }

        game.play(direction);
        steps += 1;
    }

    if !game.won() {
        return Err(format!("should win now after {steps} steps"));
    }

    Ok(())
}

fn scan_level_dir(path: &str, results: &mut Vec<LevelResult>) {
    fs::read_dir(path).unwrap().for_each(|entry| {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            scan_level_dir(path.to_str().unwrap(), results);
        } else if path.extension() == Some(OsStr::new("txt")) {
            if let Err(message) = test_level(&path) {
                let level_name = path.file_stem().unwrap().to_str().unwrap().to_string();
                results.push(LevelResult { level_name, message });
            }
        }
    });
}

#[test]
fn test_levels() {
    let mut results = Vec::new();
    scan_level_dir("levels", &mut results);

    if !results.is_empty() {
        for result in &results {
            println!("[{}] {}", result.level_name, result.message);
        }
        panic!("{} levels failed", results.len());
    }
}
