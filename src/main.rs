mod engine;
mod platform;

const GAME_NAME: &str = "Purpl";
const GAME_EXECUTABLE_NAME: &str = "purpl";
const GAME_VERSION_STRING: &str = "1.0.0";

fn main() {
    let mut running = false;

    platform::init();
    engine::init();

    running = true;
    while running {
        running = platform::video::update();
    }

    engine::shutdown();
    platform::shutdown();
}
