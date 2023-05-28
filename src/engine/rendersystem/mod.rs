use log::{error, info};
use nalgebra::*;
use std::{cell::SyncUnsafeCell, collections::HashMap, fs, io, mem, sync::Arc};

#[cfg(not(any(target_os = "macos", target_os = "ios", xbox)))]
mod vulkan;

pub trait RenderBackend {
    fn init(video: &Box<dyn crate::platform::video::VideoBackend>) -> Self;
    fn load_resources(&mut self, models: Vec<u8>);
    fn begin_commands(&mut self, video: &Box<dyn crate::platform::video::VideoBackend>);
    fn present(&mut self);
    fn unload_resources(&mut self);
    fn shutdown(self);
}

pub struct State {
    backend: Box<dyn RenderBackend>,
    models: Vec<u8>,
}

impl State {
    pub fn init(video: &crate::platform::video::State) -> Self {
        info!("Render system initialization started");
        let backend = ;
        info!("Render system initialization succeeded");

        Self {
            backend,
            models: Vec::new()
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
    pub fn new(state: &mut State, name: &str, shader: Shader, texture: RenderTexture) -> Result<Self, ()> {
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
    material: ThingHolder<Material>,
}

impl Renderable for Model {
    fn render(&self, state: &mut State) {
        if state.backend.is_in_frame() {
            state.backend.render_model(self);
        }
    }
}
