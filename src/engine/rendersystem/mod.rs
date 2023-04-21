use log::info;

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

pub struct Model {
    name: String,
    //mesh: Mesh,
    material: Material,
    //#[cfg(not(any(macos, ios)))]
    //handle: VulkanModel
}

pub trait Renderable {
    fn render();
}

#[cfg(not(any(macos, ios, xbox)))]
mod vulkan;

mod render_impl {
    #[cfg(not(any(macos, ios, xbox)))]
    pub use crate::engine::rendersystem::vulkan::*;
}

pub fn init() {
    info!("Render system initialization started");
    render_impl::State::init();
    info!("Render system initialization succeeded");
}

pub fn begin_cmds() {
    render_impl::State::begin_cmds()
}

pub fn present() {
    render_impl::State::present()
}

pub fn shutdown() {
    info!("Render system shutdown started");
    render_impl::State::shutdown();
    info!("Render system shutdown succeeded");
}
