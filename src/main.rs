#![cfg_attr(
    all(
        windows,
        not(any(build = "debug", all(not(build = "debug"), feature = "release_log")))
    ),
    windows_subsystem = "windows"
)]

mod engine;
mod game;
mod platform;

pub use game::*;

pub use texture;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = GAME_EXECUTABLE_NAME.clone().to_str())]
    game: String,
}

fn main() {
    platform::init();
    engine::init();

    while platform::video::update() {
        engine::update();
    }

    engine::shutdown();
    platform::shutdown();
}
