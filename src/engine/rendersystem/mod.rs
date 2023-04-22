use log::info;
use std::sync::{Mutex};

pub struct Shader {
    name: String,
    vertex_binary: Vec<u8>,
    fragment_binary: Vec<u8>,
    //#[cfg(not(any(macos, ios)))]
    //handle: VulkanShader,
}

pub struct RenderTexture {
    name: String,
    //texture: Texture,
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

pub fn init() {
    info!("Render system initialization started");
    *STATE.lock().unwrap() = Some(render_impl::State::init());
    info!("Render system initialization succeeded");
}

pub fn begin_cmds() {
    //STATE.lock().unwrap().as_ref().unwrap().begin_cmds()
}

pub fn present() {
    //STATE.lock().unwrap().as_ref().unwrap().present()
}

pub fn shutdown() {
    info!("Render system shutdown started");
    STATE.lock().unwrap().as_ref().unwrap().shutdown();
    info!("Render system shutdown succeeded");
}

impl Renderable for Model {
    fn render() {
//        STATE.lock().unwrap().as_ref().unwrap().render_model()
    }
}
