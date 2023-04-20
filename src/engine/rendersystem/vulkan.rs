use log::debug;
use std::sync::{Arc, Mutex};

struct GpuInfo {
    device: vulkano::device::physical::PhysicalDevice,
    props: vulkano::device::Properties,

}

struct State {
    library: Arc<vulkano::VulkanLibrary>,
    instance: Arc<vulkano::instance::Instance>,
    gpu: GpuInfo,
    gpus: Vec<GpuInfo>,
    device: Arc<vulkano::device::Device>,

}

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub fn init() {
    debug!("Vulkan initialization started");
    debug!("Vulkan initialization succeeded");
}

pub fn begin_cmds() {
}

pub fn present() {
}

pub fn shutdown() {
    debug!("Vulkan shutdown started");
    debug!("Vulkan shutdown succeeded");
}
