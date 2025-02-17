use std::io;
use std::io::stdout;
use teng::components::KeyPressRecorderComponent;
use teng::Game;

fn main() -> io::Result<()> {
    let mut game = Game::new(stdout());
    game.add_component(Box::new(KeyPressRecorderComponent::new()));

    game.run()
}
