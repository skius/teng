use std::io;
use std::io::stdout;
use teng::components::KeyPressRecorderComponent;
use teng::Game;

fn main() -> io::Result<()> {
    let mut game = Game::new(stdout());
    game.install_recommended_components();

    game.run()
}
