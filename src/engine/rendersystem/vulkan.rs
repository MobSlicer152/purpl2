use ash::{extensions, vk};
use log::debug;
use std::sync::{Arc, Mutex};

macro_rules! vulkan_check {
    ($call: expr) => {
        $call.unwrap_or_else(|err| panic!("Vulkan call {} failed: {}", stringify!($call), err))
    };
}

struct GpuInfo {
    device: vk::PhysicalDevice,
    props: vk::PhysicalDeviceProperties,
    surface_caps: vk::SurfaceCapabilitiesKHR,
    surface_fmts: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
    queue_family_props: Vec<vk::QueueFamilyProperties>,
    graphics_family_index: u32,
    present_family_index: u32,
    extension_properties: Vec<vk::ExtensionProperties>
}

pub struct State {
    instance: vk::Instance,
    
    surface: extensions::khr::Surface,
    
    gpu: GpuInfo,
    gpus: Vec<GpuInfo>,
    device: vk::Device,
    
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    swapchain: extensions::khr::Swapchain,
    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
}

impl State {
    fn create_instance(library: ash::Entry) -> vk::Instance {
        debug!("Creating Vulkan instance");
    
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 3, 0),
            ..Default::default()
        };

        let instance = unsafe { entry.create_instance(&create_info, ) };

        debug!("Create Vulkan instance {} successfully");
    }

    fn get_gpus(instance: Arc<vulkano::instance::Instance) -> Vec<GpuInfo> {
        vec![]
    }

    fn select_gpu(gpus: Vec<GpuInfo>) -> usize {
        0
    }

    unsafe fn create_device(instance: vk::Instance, gpu: GpuInfo) -> (vk::Device, vk::Queue, vk::Queue) {

    }
    
    pub fn init() -> Self {
        debug!("Vulkan initialization started");

        debug!("Loading Vulkan library");
        let library = vulkan_check!(ash::Entry::load());

        let instance = Self::create_instance(library);
        let surface = crate::platform::video::create_vulkan_surface(instance);
        let gpus = Self::get_gpus(instance);
        let gpu_idx = Self::select_gpu(gpus);
        let gpu = gpus[gpu_idx];
        let (device, graphics_queue, present_queue) = Self::create_device(instance, gpu);

        debug!("Vulkan initialization succeeded");

        Self {
            library: library,
            instance: instance,
            surface: surface,
            gpu: gpu,
            gpus: gpus,
            device: device,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
        }
    }
    
    pub fn begin_cmds() {
    }
    
    pub fn present() {
    }
    
    pub fn shutdown() {
        debug!("Vulkan shutdown started");
        debug!("Vulkan shutdown succeeded");
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
macro_rules! get_state {
    () => {
        STATE.lock().unwrap().as_ref().unwrap()
    }
}
