use log::{error, info};
use nalgebra::*;
use std::{fs, io, collections::HashMap, sync::Mutex};

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

mod render_impl {
    #[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
    pub use crate::engine::rendersystem::vulkan::*;
}

static BACKEND: Mutex<Option<render_impl::State>> = Mutex::new(None);
macro_rules! get_backend {
    () => {
        BACKEND.lock().unwrap().as_mut().unwrap()
    };
}
macro_rules! have_backend {
    () => {
        BACKEND.lock().unwrap().is_some()
    };
}

struct State {
    shaders: HashMap<String, Option<Shader>>
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
macro_rules! get_state {
    () => {
        STATE.lock().unwrap().as_mut().unwrap()
    };
}
macro_rules! have_state {
    () => {
        STATE.lock().unwrap().is_some()
    };
}

#[derive(Debug)]
pub enum ShaderError {
    Io(io::Error),
    Backend(render_impl::ShaderErrorType),
}

pub struct Shader {
    name: String,
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

impl Shader {
    pub fn new(name: &str) -> Result<(), ShaderError> {
        assert!(have_state!() && have_backend!());
        
        let vertex_path = format!("{}{}{}", crate::engine::GameDirs::shaders(), name, render_impl::ShaderData::vertex_extension());
        let fragment_path = format!("{}{}{}", crate::engine::GameDirs::shaders(), name, render_impl::ShaderData::fragment_extension());
        let vertex_binary =
            match fs::read(&vertex_path) {
                Ok(data) => data,
                Err(err) => {
                    error!("Failed to read vertex binary {vertex_path} for shader {name}: {err}");
                    return Err(ShaderError::Io(err));
                }
            };
        let fragment_binary =
            match fs::read(&fragment_path) {
                Ok(data) => data,
                Err(err) => {
                    error!("Failed to read fragment binary {fragment_path} for shader {name}: {err}");
                    return Err(ShaderError::Io(err));
                }
            };
        let handle = match render_impl::ShaderData::new(
            get_backend!(),
            name,
            vertex_binary,
            fragment_binary,
        ) {
            Ok(handle) => handle,
            Err(err) => {
                error!("Failed to create shader {name}: {err:?}");
                return Err(err);
            }
        };

        get_state!().shaders.insert(String::from(name), Some(Self {
            name: String::from(name),
            handle,
        }));

        Ok(())
    }

    pub fn destroy(&self) {
        self.handle.destroy(get_backend!());
        get_state!().shaders.remove(&self.name);
    }
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

pub fn init() {
    info!("Render system initialization started");
    *BACKEND.lock().unwrap() = Some(render_impl::State::init());
    *STATE.lock().unwrap() = Some(State {
        shaders: HashMap::new()
    });
    info!("Render system initialization succeeded");
}

pub fn begin_cmds() {
    get_backend!().begin_cmds()
}

pub fn present() {
    get_backend!().present()
}

pub fn shutdown() {
    info!("Render system shutdown started");

    get_state!().shaders.iter().for_each(|(_, shader)| { shader.as_ref().unwrap().destroy() });

    BACKEND.lock().unwrap().take().unwrap().shutdown();
    info!("Render system shutdown succeeded");
}

impl Renderable for Model {
    fn render() {
        //get_state!().render_model()
    }
}
