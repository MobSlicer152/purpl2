use log::{error, info};
use nalgebra::*;
use std::{fs, io, sync::Mutex};

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

mod render_impl {
    #[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
    pub use crate::engine::rendersystem::vulkan::*;
}

#[derive(Debug)]
pub enum ShaderError {
    Io(io::Error)
}

pub struct Shader {
    name: String,
    handle: render_impl::ShaderData,
}

impl Shader {
    pub fn new(name: String) -> Result<Self, ShaderError> {
        let vertex_binary = match fs::read(crate::engine::GameDirs::shaders() + name.as_str() + ".vert.spv") {
            Ok(data) => data,
            Err(err) => {
                error!("Failed to read vertex binary for shader {name}: {err}");
                return Err(ShaderError::Io(err));
            }
        };
        let fragment_binary = match fs::read(crate::engine::GameDirs::shaders() + name.as_str() + ".frag.spv") {
            Ok(data) => data,
            Err(err) => {
                error!("Failed to read fragment binary for shader {name}: {err}");
                return Err(ShaderError::Io(err));
            }
        };
        let handle = match render_impl::ShaderData::new(&name, vertex_binary, fragment_binary) {
            Ok(handle) => handle,
            Err(err) => {
                error!("Failed to create shader {name}: {err:?}");
                return Err(err);
            }
        };

        Ok(Self {
            name,
            handle
        })
    }
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
