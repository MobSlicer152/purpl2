mod engine;
mod platform;

include!("game.rs");

fn main() {
    platform::init();
    engine::init();

    while platform::video::update() {
        engine::update();
    }

    engine::shutdown();
    platform::shutdown();
}
