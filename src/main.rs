#![cfg_attr(
    all(
        windows,
        not(any(build = "debug", feature = "release_log"))
    ),
    windows_subsystem = "windows"
)]

#![feature(closure_lifetime_binder)]
#![feature(sync_unsafe_cell)]
#![feature(vec_into_raw_parts)]

mod engine;
mod game;
mod platform;

use engine::rendersystem::Renderable;

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
    wait_for_debugger: bool,
    #[cfg_attr(not(any(macos, ios)), arg(short, long, default_value_t = engine::rendersystem::RenderApi::Vulkan))]
    render_api: engine::rendersystem::RenderApi,

}

fn main() {
    platform::init();
    let mut engine_state = engine::State::init(Args::parse());

    let shader = engine::rendersystem::Shader::new(engine_state.render_state(), "basic").unwrap();
    let mut image = image::RgbaImage::new(1, 1);
    image.fill(0xFF);
    let texture = engine::rendersystem::RenderTexture::new(engine_state.render_state(), "test", image).unwrap();
    let material = engine::rendersystem::Material::new(engine_state.render_state(), "basic", &shader, &texture).unwrap();
    let mut obj = tobj::load_obj("test.obj", &tobj::LoadOptions {
        triangulate: true,
        ..Default::default()
    }).unwrap().0;
    let model = engine::rendersystem::Model::new(engine_state.render_state(), "test", &mut obj, &material);

    engine_state.render_state().load_resources();

    while engine_state.video_state().update() {
        engine_state.update(Some(for <'a> |state: &'a mut engine::State| -> () {
            model.render(state.render_state());
        }));
    }

    engine_state.shutdown();
    platform::shutdown();
}
