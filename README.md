# Parabox

Parabox is a project written in [Rust](https://www.rust-lang.org/) that simulates the gameplay of the puzzle game _[Patrick's Parabox](https://www.patricksparabox.com/)_. It provides a simple text-based UI for playing and testing.

## Installation and Build

To install and build the project, make sure you have [Cargo](https://doc.rust-lang.org/cargo/) installed. Then install the dependencies and build the project by running `cargo build`. If you want a Release build, add the argument `--release`.

## Running the Game

The command line arguments are as follows:

```
parabox <puzzle> [sequence]
```

- `puzzle` is the path to the puzzle file you want to play, e.g. `levels/vanilla/enter.txt`. [File format](https://www.patricksparabox.com/custom-levels/)
- `sequence` is a movement sequence specified as `LRUD` (Left, Right, Up, Down). This sequence will be executed when the game starts.

With Cargo, you can use `cargo run -- <args>` to run the game.

## Gameplay Controls

The text-based UI looks similar to the vanilla one.

- `#` for walls
- `.` for empty cells
- `p` for player
- `b` for solid blocks
- `=` for player goals
- `_` for block goals
- `0-9`, `A-F` for blocks, in their own color
- `I-N` for infinite exit blocks, in the corresponding block's color
- clones (not an exit block) are rendered in an "inverted" style
- horizontally flipped blocks are rendered with an underline

During gameplay, you can use the following controls:

- **WASD**: Move the player.
- **R**: Restart the current puzzle.
- **Z**: Undo the previous move.
- **P**: Print debug information.
- **Q**: Quit the game.

## Testing

The project includes vanilla levels from the original game stored in `levels/vanilla/{level_name}.txt`, along with their corresponding solutions (with the `.solution` extension). You can test the simulator using these levels by running the following command:

```
cargo test --test levels
```

The test program, located in `tests/levels.rs`, will run the simulator through all puzzles under the `levels/` folder, comparing the solutions to ensure they work correctly. Please note that the simulator might have some differences compared to the vanilla game in certain details or edge cases.

## Documentation

This project serves as a prototype, so documentation is currently sparse. However, there are comments within the code that can help you understand its functionality. In the future, more detailed documentation may be added.

## Modules

The project is organized into the following modules:

- `parabox::engine::game`: Contains the game logic and related data structures, including the `Game` struct.
- `parabox::engine::simulation`: Exports the `Simulator` struct for simulation purposes.
- `parabox::engine::utility`: Contains utility functions and structures.
- `main.rs`: Implements the text-based UI and basic input handling.

## Acknowledgements

- [Patrick Traynor](https://cwpat.me/about), for designing and developing the awesome game
- [Patrick's Parabox Walkthrough - All Levels](https://steamcommunity.com/sharedfiles/filedetails/?id=2786724419), where I have taken the solutions to all vanilla levels (some are revised for horizontal flips)
- [ChatGPT](https://chat.openai.com/), for writing the README
