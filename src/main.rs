#![cfg_attr(
    all(
        windows,
        not(any(build = "debug", all(not(build = "debug"), feature = "release_log")))
    ),
    windows_subsystem = "windows"
)]
#![feature(sync_unsafe_cell)]
#![feature(vec_into_raw_parts)]

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
    #[arg(short, long, default_value_t = false)]
    wait_for_debugger: bool
}

fn main() {
    platform::init();
    let mut engine_state = engine::State::init(Args::parse());
    
    while engine_state.video().update() {
        engine_state.update();
    }

    engine_state.shutdown();
    platform::shutdown();
}
