use log::debug;
use std::sync::{Arc, Mutex};

macro_rules! vulkan_check {
    ($call: expr) => {
        let result = $call;
        result.unwrap_or_else(|err| panic!("Vulkan call {} failed: {}", stringify!($call), err))
    };
}

struct GpuInfo {
    device: vulkano::device::physical::PhysicalDevice,
    props: vulkano::device::Properties,
    surface_caps: vulkano::swapchain::SurfaceCapabilities,
    surface_fmts: Vec<(vulkano::format::Format, vulkano::swapchain::ColorSpace)>,
    present_modes: Vec<vulkano::swapchain::PresentMode>,
    queue_family_props: Vec<vulkano::device::QueueFamilyProperties>,
    graphics_family_index: u32,
    present_family_index: u32,
    extension_properties: Vec<vulkano::ExtensionProperties>
}

struct State {
    library: Arc<vulkano::VulkanLibrary>,
    instance: Arc<vulkano::instance::Instance>,
    gpu: GpuInfo,
    gpus: Vec<GpuInfo>,
    device: Arc<vulkano::device::Device>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn create_instance() {
    debug!("Creating Vulkan instance");
}

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
