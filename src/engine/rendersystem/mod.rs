use log::info;
use std::sync::Mutex;

pub struct Shader {
    name: String,
    vertex_binary: Vec<u8>,
    fragment_binary: Vec<u8>,
    //#[cfg(not(any(macos, ios)))]
    //handle: VulkanShader,
}

pub struct RenderTexture {
    name: String,
    texture: crate::texture::Texture,
    //#[cfg(not(any(macos, ios)))]
    //handle: VulkanTexture,
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
    //#[cfg(not(any(macos, ios)))]
    //handle: VulkanModel
}

#[cfg(not(any(macos, ios, xbox)))]
mod vulkan;

mod render_impl {
    #[cfg(not(any(macos, ios, xbox)))]
    pub use crate::engine::rendersystem::vulkan::*;
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
    //get_state!().begin_cmds()
}

pub fn present() {
    //get_state!().present()
}

pub fn shutdown() {
    info!("Render system shutdown started");
    get_state!().shutdown();
    info!("Render system shutdown succeeded");
}

impl Renderable for Model {
    fn render() {
        //get_state!().render_model()
    }
}
