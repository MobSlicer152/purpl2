use log::info;

pub struct Shader {
    name: String,
    vertex_binary: Vec<u8>,
    fragment_binary: Vec<u8>,
    //#[cfg(feature = "vulkan")]
    //handle: VulkanShader,
}

pub struct RenderTexture {
    name: String,
    //texture: Texture,
    //#[cfg(feature = "vulkan")]
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
    //#[cfg(feature = "vulkan")]
    //handle: VulkanModel
}

pub trait Renderable {
    fn render();
}

#[cfg(feature = "vulkan")]
mod vulkan;

mod render_impl {
    #[cfg(feature = "vulkan")]
    pub use crate::engine::rendersystem::vulkan::*;
}

pub fn init() {
    info!("Render system initialization started");
    render_impl::init();
    info!("Render system initialization succeeded");
}

pub fn begin_cmds() {
    render_impl::begin_cmds()
}

pub fn present() {
    render_impl::present()
}

pub fn shutdown() {
    info!("Render system shutdown started");
    render_impl::shutdown();
    info!("Render system shutdown succeeded");
}
