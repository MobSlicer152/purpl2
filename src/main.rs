// #![allow(unused)]

mod engine;
mod game;
mod platform;

pub use game::*;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    platform::init();
    engine::init();

    while platform::video::update() {
        engine::update();
    }

    engine::shutdown();
    platform::shutdown();
}
