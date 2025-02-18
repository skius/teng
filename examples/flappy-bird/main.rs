use std::io;
use std::io::stdout;
use teng::Game;

fn main() -> io::Result<()> {
    let mut game = Game::new(stdout());
    game.run()
}
