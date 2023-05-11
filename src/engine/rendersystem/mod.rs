use log::info;
use nalgebra::*;
use std::sync::Mutex;

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

mod render_impl {
    #[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
    pub use crate::engine::rendersystem::vulkan::*;
}

pub struct Shader {
    name: String,
    vertex_binary: Vec<u8>,
    fragment_binary: Vec<u8>,
    handle: render_impl::ShaderData,
}

#[repr(C)]
pub struct UniformData {
    model: Matrix4<f64>,
    view: Matrix4<f64>,
    projection: Matrix4<f64>,
}

pub struct RenderTexture {
    name: String,
    texture: crate::texture::Texture,
    //handle: render_impl::TextureData
}

pub struct Material {
    name: String,
    shader: Shader,
    texture: RenderTexture,
}

pub trait Renderable {
    fn render();
}

pub struct Model {
    name: String,
    //mesh: Mesh,
    material: Material,
    //handle: render_impl::ModelData
}

static STATE: Mutex<Option<render_impl::State>> = Mutex::new(None);
macro_rules! get_state {
    () => {
        STATE.lock().unwrap().as_mut().unwrap()
    };
}

pub fn init() {
    info!("Render system initialization started");
    *STATE.lock().unwrap() = Some(render_impl::State::init());
    info!("Render system initialization succeeded");
}

pub fn begin_cmds() {
    get_state!().begin_cmds()
}

pub fn present() {
    get_state!().present()
}

pub fn shutdown() {
    info!("Render system shutdown started");
    STATE.lock().unwrap().take().unwrap().shutdown();
    info!("Render system shutdown succeeded");
}

impl Renderable for Model {
    fn render() {
        //get_state!().render_model()
    }
}
