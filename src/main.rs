mod engine;
mod platform;

use const_format::concatcp;
include!("game.rs");

fn main() {
    let mut running = true;

    platform::init();
    engine::init();

    while running {
        running = platform::video::update();
    }

    engine::shutdown();
    platform::shutdown();
}
