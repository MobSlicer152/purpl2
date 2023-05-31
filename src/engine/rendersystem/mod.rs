use log::info;
use nalgebra::*;

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

pub trait RenderBackend {
    fn init(video: &Box<dyn crate::platform::video::VideoBackend>) -> Box<dyn RenderBackend>
    where
        Self: Sized;
    fn load_resources(&mut self, models: &Vec<u8>);
    fn begin_commands(&mut self, video: &Box<dyn crate::platform::video::VideoBackend>);
    fn render_model(&mut self, model: &Model);
    fn present(&mut self);
    fn unload_resources(&mut self);
    fn shutdown(&mut self);
    fn set_gpu(&mut self, gpu_index: usize) -> usize;
    fn is_initialized(&self) -> bool;
    fn is_loaded(&self) -> bool;
    fn is_in_frame(&self) -> bool;
}

#[derive(Clone, Debug)]
pub enum RenderApi {
    None,
    #[cfg(not(any(macos, ios)))]
    Vulkan,
    #[cfg(windows)]
    DirectX,
}

impl clap::ValueEnum for RenderApi {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            #[cfg(not(any(macos, ios)))]
            Self::Vulkan,
            #[cfg(windows)]
            Self::DirectX,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::None => None,
            #[cfg(not(any(macos, ios)))]
            Self::Vulkan => Some(clap::builder::PossibleValue::new("Vulkan")),
            #[cfg(windows)]
            Self::DirectX => Some(clap::builder::PossibleValue::new("DirectX")),
        }
    }
}

impl std::fmt::Display for RenderApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            #[cfg(not(any(macos, ios)))]
            Self::Vulkan => f.write_str("Vulkan"),
            #[cfg(windows)]
            Self::DirectX => f.write_str("DirectX"),
        }
    }
}

pub struct State {
    render_api: RenderApi,
    backend: Box<dyn RenderBackend>,
    models: Vec<u8>,
}

impl State {
    pub fn init(
        video: &Box<dyn crate::platform::video::VideoBackend>,
        render_api: RenderApi,
    ) -> Self {
        info!("Render system initialization started");
        let backend = match render_api {
            #[cfg(not(any(macos, ios)))]
            RenderApi::Vulkan => vulkan::State::init(video),
            //#[cfg(windows)]
            //RenderApi::DirectX => directx::State::init(video),
            _ => panic!("Unimplemented render backend requested"),
        };
        info!("Render system initialization succeeded");

        Self {
            render_api,
            backend,
            models: Vec::new(),
        }
    }

    pub fn load_resources(&mut self) {
        if self.backend.is_initialized() && !self.backend.is_loaded() {
            info!("Loading resources");
            self.backend.load_resources(&mut self.models);
            info!("Done loading resources");
        }
    }

    pub fn begin_commands(&mut self, video: &Box<dyn crate::platform::video::VideoBackend>) {
        self.backend.begin_commands(video)
    }

    pub fn present(&mut self) {
        self.backend.present()
    }

    pub fn unload_resources(&mut self) {
        if self.backend.is_initialized() && self.backend.is_loaded() {
            info!("Unloading resources");
            self.backend.unload_resources();
            info!("Done unloading resources");
        }
    }

    pub fn shutdown(mut self) {
        info!("Render system shutdown started");
        self.unload_resources();
        self.backend.shutdown();
        info!("Render system shutdown succeeded");
    }
}

pub struct Shader {
    name: String,
}

#[repr(C)]
pub struct UniformData {
    model: Matrix4<f64>,
    view: Matrix4<f64>,
    projection: Matrix4<f64>,
}

pub struct RenderTexture {
    name: String,
    //texture: crate::texture::Texture,
}

pub struct Material {
    name: String,
    shader: Shader,
    texture: RenderTexture,
}

impl Material {
    pub fn new(
        state: &mut State,
        name: &str,
        shader: Shader,
        texture: RenderTexture,
    ) -> Result<Self, ()> {
        Ok(Self {
            name: String::from(name),
            shader,
            texture,
        })
    }

    pub fn name(&self) -> &String {
        &self.name
    }
}

pub trait Renderable {
    fn render(&self, state: &mut State);
}

#[derive(PartialEq)]
pub struct Vertex {
    position: Vector3<f32>,
    texture_coordinate: Vector2<f32>,
    normal: Vector3<f32>,
}

pub struct Model {
    name: String,
    size: usize,
    offset: usize,
}

impl Renderable for Model {
    fn render(&self, state: &mut State) {
        if state.backend.is_in_frame() {
            state.backend.render_model(self);
        }
    }
}
