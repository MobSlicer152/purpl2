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

use clap::Parser;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = String::from(GAME_EXECUTABLE_NAME.clone()))]
    game: String,
}

fn main() {
    platform::init();
    engine::init(Args::parse());

    engine::rendersystem::Shader::new("basic").unwrap();

    while platform::video::update() {
        engine::update();
    }

    engine::shutdown();
    platform::shutdown();
}
