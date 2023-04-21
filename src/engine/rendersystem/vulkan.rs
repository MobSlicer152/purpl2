use ash::{extensions, vk};
use log::debug;
use std::{mem, os, ptr};
use std::sync::{Arc, Mutex};

macro_rules! vulkan_check {
    ($call: expr) => {
        $call.unwrap_or_else(|err| panic!("Vulkan call {} failed: {}", stringify!($call), err))
    };
}

extern "system" fn vulkan_alloc(p_user_data: *mut os::raw::c_void, size: usize, alignment: usize, allocation_scope: vk::SystemAllocationScope) -> *mut os::raw::c_void {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(size, alignment).unwrap()) as *mut os::raw::c_void }
}

extern "system" fn vulkan_realloc(p_user_data: *mut os::raw::c_void, p_original: *mut os::raw::c_void, size: usize, alignment: usize, allocation_scope: vk::SystemAllocationScope) -> *mut os::raw::c_void {
    unsafe { std::alloc::realloc(p_original as *mut u8, std::alloc::Layout::from_size_align(size, alignment).unwrap(), size) as *mut os::raw::c_void }
}

extern "system" fn vulkan_dealloc(p_user_data: *mut os::raw::c_void, p_memory: *mut os::raw::c_void) {
    unsafe { std::alloc::dealloc(p_memory as *mut u8, std::alloc::Layout::from_size_align(0, 1).unwrap()) }
}

const ALLOCATION_CALLBACKS: vk::AllocationCallbacks = vk::AllocationCallbacks {
    pfn_allocation: Some(vulkan_alloc),
    pfn_reallocation: Some(vulkan_realloc),
    pfn_free: Some(vulkan_dealloc),
    ..Default::default()
};

const FRAME_COUNT: usize = 3;

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
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    
    surface: vk::SurfaceKHR,
    
    gpu: GpuInfo,
    gpus: Vec<GpuInfo>,
    
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    fences: [vk::Fence; FRAME_COUNT],
    acquire_semaphores: [vk::Semaphore; FRAME_COUNT],
    render_complete_semaphores: [vk::Semaphore; FRAME_COUNT],

    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
}

impl State {
    fn create_instance(entry: ash::Entry) -> ash::Instance {
        debug!("Creating Vulkan instance");
    
        let app_info = vk::ApplicationInfo {
            p_application_name: unsafe { crate::GAME_NAME.as_ptr() as *const os::raw::c_char },
            application_version: vk::make_api_version(0, crate::GAME_VERSION_MAJOR.into(), crate::GAME_VERSION_MINOR.into(), crate::GAME_VERSION_PATCH.into()),
            p_engine_name: "Purpl Engine".as_ptr() as *const os::raw::c_char,
            engine_version: 2,
            api_version: vk::make_api_version(0, 1, 3, 0),
            ..Default::default()
        };

        let extensions = vec![
            extensions::khr::DynamicRendering::name(),
            extensions::khr::Surface::name(),
            extensions::khr::Swapchain::name(),

            #[cfg(windows)]
            extensions::khr::Win32Surface::name(),
            #[cfg(unix)]
            extensions::khr::XcbSurface::name(),
        ];

        let validation_layers = vec![
            "VK_LAYER_KHRONOS_VALIDATION".as_ptr() as *const os::raw::c_char
        ];

        let create_info = vk::InstanceCreateInfo {
            p_application_info: ptr::addr_of!(app_info),
            #[cfg(feature = "graphics_debug")]
            enabled_layer_count: validation_layers.len() as u32,
            #[cfg(feature = "graphics_debug")]
            pp_enabled_layer_names: validation_layers.as_ptr(),
            ..Default::default()
        };

        let instance = unsafe { vulkan_check!(entry.create_instance(&create_info, Some(&ALLOCATION_CALLBACKS))) };

        debug!("Created Vulkan instance successfully");
        instance
    }

    fn get_gpus(instance: vk::Instance) -> Vec<GpuInfo> {
        vec![]
    }

    fn select_gpu(gpus: Vec<GpuInfo>) -> usize {
        0
    }

    fn create_device(instance: vk::Instance, gpu: GpuInfo) -> (ash::Device, vk::Queue, vk::Queue) {

    }

    fn create_fences(device: ash::Device) -> [vk::Fence; FRAME_COUNT] {

    }

    fn create_semaphores(device: ash::Device) -> ([vk::Semaphore; FRAME_COUNT], [vk::Semaphore; FRAME_COUNT]) {
        
    }

    fn choose_surface_format(gpu: GpuInfo) -> vk::SurfaceFormatKHR {

    }

    fn create_swapchain(device: ash::Device, surface_format: vk::SurfaceFormatKHR, swapchain_extent: vk::Extent2D) -> (vk::SwapchainKHR, Vec<vk::Image>) {

    }

    pub fn init() -> Self {
        debug!("Vulkan initialization started");

        debug!("Loading Vulkan library");
        let entry = unsafe { vulkan_check!(ash::Entry::load()) };

        let instance = Self::create_instance(entry);
        let surface = crate::platform::video::create_vulkan_surface(entry, instance, ALLOCATION_CALLBACKS);
        let gpus = Self::get_gpus(instance.handle());
        let gpu_idx = Self::select_gpu(gpus);
        let gpu = gpus[gpu_idx];
        let (device, graphics_queue, present_queue) = Self::create_device(instance.handle(), gpu);
        let fences = Self::create_fences(device);
        let (acquire_semaphores, render_complete_semaphores) = Self::create_semaphores(device);
        let surface_format = Self::choose_surface_format(gpu);
        let video_size = crate::platform::video::get_size();
        let swapchain_extent = vk::Extent2D { width: video_size.0, height: video_size.1 };
        let (swapchain, swapchain_images) = Self::create_swapchain(device, surface_format, swapchain_extent);

        debug!("Vulkan initialization succeeded");

        Self {
            entry: entry,
            instance: instance,
            surface: surface,
            device: device,
            gpu: gpu,
            gpus: gpus,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
            fences: fences,
            acquire_semaphores: acquire_semaphores,
            render_complete_semaphores: render_complete_semaphores,
            swapchain: swapchain,
            surface_format: surface_format,
            
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
