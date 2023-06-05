use log::{error, info};
use nalgebra::*;
use std::{any::Any, mem};

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

pub trait RenderBackend {
    fn as_any(&self) -> &dyn Any;
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

    fn create_shader(&self, shader_path: &String, name: &String) -> Result<Box<dyn ShaderData>, String>;
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
        info!("Render system initialization started with backend {render_api}");
        let backend = match render_api {
            #[cfg(not(any(macos, ios)))]
            RenderApi::Vulkan => vulkan::State::init(video),
            #[cfg(windows)]
            RenderApi::DirectX => todo!(), //directx::State::init(video),
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
            self.models.clear();
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

pub trait ShaderData {
    fn as_any(&self) -> &dyn Any;
    fn destroy(&mut self, state: &Box<dyn RenderBackend>);
}

pub struct Shader {
    name: String,
    data: Box<dyn ShaderData>
}

impl Shader {
    pub fn new(state: &super::State, name: &str) -> Result<Self, String> {
        let name = String::from(name);
        let shader_path = format!("{}/{name}", super::GameDirs::shaders(state));
        let data = state.render_state().backend.create_shader(&shader_path, &name)?;
        Ok(Self {
            name,
            data
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
    texture: image::RgbaImage,
}

impl RenderTexture {
    pub fn new(_state: &State, name: &str, texture: image::RgbaImage) -> Result<Self, String> {
        Ok(Self {
            name: String::from(name),
            texture
        })
    }
}

pub struct Material<'a> {
    name: String,
    shader: &'a Shader,
    texture: &'a RenderTexture,
}

impl<'a> Material<'a> {
    pub fn new(
        state: &mut State,
        name: &str,
        shader: &'a Shader,
        texture: &'a RenderTexture,
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

#[derive(Clone)]
pub struct Model<'a> {
    name: String,
    offset: usize,
    vertices_size: usize,
    indices_size: usize,
    material: &'a Material<'a>
}

impl<'a> Model<'a> {
    pub fn new(state: &mut State, name: &str, models: &mut Vec<tobj::Model>, material: &'a Material<'a>) -> Self {
        if !state.backend.is_initialized() || state.backend.is_loaded() {
            error!("Not creating model {name} at this time");
        }

        info!("Creating model {name}");

        // largely based on https://github.com/bwasty/learn-opengl-rs/blob/master/src/model.rs
        let mut all_vertices = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();
        for model in models {
            let mesh = &mut model.mesh;

            assert!(!mesh.normals.is_empty() && !mesh.texcoords.is_empty());

            let vertex_count = mesh.positions.len() / 3;
            let mut vertices = Vec::with_capacity(vertex_count);
            let (p, t, n) = (&mesh.positions, &mesh.texcoords, &mesh.normals);
            for i in 0..vertex_count {
                let position = Vector3::new(p[i * 3 + 0], p[i * 3 + 1], p[i * 3 + 2]);
                let texture_coordinate = Vector2::new(t[i * 2 + 0], t[i * 2 + 1]);
                let normal = Vector3::new(0f32, 0f32, 0f32); // Vector3::new(n[i * 3 + 0], n[i * 3 + 1], n[i * 3 + 2]);
                vertices.push(Vertex {
                    position,
                    texture_coordinate,
                    normal
                })
            }

            all_vertices.append(&mut vertices);
            all_indices.append(&mut mesh.indices);
        }

        let vertices_size = all_vertices.len() * mem::size_of::<Vertex>();
        let indices_size = all_indices.len() * mem::size_of::<u32>();

        let mut data = Vec::new();
        let vertices = all_vertices.into_raw_parts();
        data.append(&mut unsafe { Vec::from_raw_parts(vertices.0 as *mut u8, vertices_size, vertices_size) });
        let indices = all_indices.into_raw_parts();
        data.append(&mut unsafe { Vec::from_raw_parts(indices.0 as *mut u8, indices_size, indices_size) });

        let offset = state.models.len();
        state.models.append(&mut data);

        Self { 
            name: String::from(name),
            offset,
            vertices_size,
            indices_size,
            material
        }
    }
}

impl<'a> Renderable for Model<'a> {
    fn render(&self, state: &mut State) {
        if state.backend.is_in_frame() {
            state.backend.render_model(self);
        }
    }
}
