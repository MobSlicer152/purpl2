use ash::{extensions, vk};
use log::{debug, error};
use std::sync::{Arc, Mutex};
use std::{cmp, ffi, mem, os, ptr};

macro_rules! vulkan_check {
    ($call: expr) => {
        $call.unwrap_or_else(|err| panic!("Vulkan call {} failed: {}", stringify!($call), err))
    };
}

extern "system" fn vulkan_alloc(
    p_user_data: *mut ffi::c_void,
    size: usize,
    alignment: usize,
    allocation_scope: vk::SystemAllocationScope,
) -> *mut ffi::c_void {
    unsafe {
        std::alloc::alloc(std::alloc::Layout::from_size_align(size, alignment).unwrap())
            as *mut ffi::c_void
    }
}

extern "system" fn vulkan_realloc(
    p_user_data: *mut ffi::c_void,
    p_original: *mut ffi::c_void,
    size: usize,
    alignment: usize,
    allocation_scope: vk::SystemAllocationScope,
) -> *mut ffi::c_void {
    unsafe {
        std::alloc::realloc(
            p_original as *mut u8,
            std::alloc::Layout::from_size_align(size, alignment).unwrap(),
            size,
        ) as *mut ffi::c_void
    }
}

extern "system" fn vulkan_dealloc(p_user_data: *mut ffi::c_void, p_memory: *mut ffi::c_void) {
    unsafe {
        std::alloc::dealloc(
            p_memory as *mut u8,
            std::alloc::Layout::from_size_align(0, 1).unwrap(),
        )
    }
}

const ALLOCATION_CALLBACKS: vk::AllocationCallbacks = vk::AllocationCallbacks {
    pfn_allocation: Some(vulkan_alloc),
    pfn_reallocation: Some(vulkan_realloc),
    pfn_free: Some(vulkan_dealloc),
    p_user_data: ptr::null_mut(),
    pfn_internal_allocation: None,
    pfn_internal_free: None,
};

const FRAME_COUNT: usize = 3;

struct GpuInfo {
    device: vk::PhysicalDevice,
    usable: bool,

    mem_props: vk::PhysicalDeviceMemoryProperties,
    props: vk::PhysicalDeviceProperties,

    surface_caps: vk::SurfaceCapabilitiesKHR,
    surface_fmts: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,

    queue_family_props: Vec<vk::QueueFamilyProperties>,
    graphics_family_idx: Option<u32>,
    present_family_idx: Option<u32>,

    ext_props: Vec<vk::ExtensionProperties>,
}

pub struct State {
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: extensions::khr::Surface,
    //swapchain_loader: extensions::khr::Swapchain,
    surface: vk::SurfaceKHR,

    gpu: usize,
    gpus: Vec<GpuInfo>,
    //    graphics_queue: vk::Queue,
    //    present_queue: vk::Queue,

    //    fences: [vk::Fence; FRAME_COUNT],
    //    acquire_semaphores: [vk::Semaphore; FRAME_COUNT],
    //    render_complete_semaphores: [vk::Semaphore; FRAME_COUNT],

    //    swapchain: vk::SwapchainKHR,
    //    swapchain_images: Vec<vk::Image>,
    //    surface_format: vk::SurfaceFormatKHR,
    //    swapchain_extent: vk::Extent2D,
}

impl State {
    unsafe extern "system" fn debug_log(
        severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        types: vk::DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        user_data: *mut ffi::c_void,
    ) -> u32 {
        let log_level = match severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::Level::Trace,
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::Level::Debug,
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::Level::Info,
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::Level::Warn,
            _ => log::Level::Debug,
        };

        let mut location = "".to_owned();
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL) {
            location += "GENERAL ";
        }
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE) {
            location += "PERFORMANCE ";
        }
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION) {
            location += "VALIDATION ";
        }
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING) {
            location += "DEVICE ADDRESS BINDING ";
        }

        let message_ptr = (*callback_data).p_message as *const ffi::c_char;
        let message_raw = unsafe { ffi::CStr::from_ptr(message_ptr) };
        let message = message_raw.to_str().unwrap();
        log::log!(log_level, "VULKAN {}MESSAGE: {}", location, message);

        vk::TRUE
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        debug!("Creating Vulkan instance");

        let app_name = ffi::CString::new(crate::GAME_NAME).unwrap();
        let engine_name = ffi::CString::new("Purpl Engine").unwrap();
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr() as *const ffi::c_char,
            application_version: vk::make_api_version(
                0,
                crate::GAME_VERSION_MAJOR.into(),
                crate::GAME_VERSION_MINOR.into(),
                crate::GAME_VERSION_PATCH.into(),
            ),
            p_engine_name: engine_name.as_ptr() as *const ffi::c_char,
            engine_version: 2,
            api_version: vk::make_api_version(0, 1, 3, 0),
            ..Default::default()
        };

        let extensions = [
            extensions::khr::Surface::name(),
            #[cfg(feature = "graphics_debug")]
            extensions::ext::DebugUtils::name(),
            #[cfg(windows)]
            extensions::khr::Win32Surface::name(),
            #[cfg(unix)]
            extensions::khr::XcbSurface::name(),
        ];

        let validation_layers = ["VK_LAYER_KHRONOS_validation"];

        let extensions_raw: Vec<*const ffi::c_char> = extensions
            .iter()
            .map(|ext_name| ext_name.as_ptr())
            .collect();
        let layers_cstr: Vec<ffi::CString> = validation_layers
            .iter()
            .map(|layer_name| ffi::CString::new(*layer_name).unwrap())
            .collect();
        let layers_raw: Vec<*const ffi::c_char> = layers_cstr
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();

        let debug_messenger_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
            pfn_user_callback: Some(Self::debug_log),
            ..Default::default()
        };

        let create_info = vk::InstanceCreateInfo {
            p_application_info: ptr::addr_of!(app_info),
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_extension_names: extensions_raw.as_ptr(),
            #[cfg(feature = "graphics_debug")]
            enabled_layer_count: layers_raw.len() as u32,
            #[cfg(feature = "graphics_debug")]
            pp_enabled_layer_names: layers_raw.as_ptr(),
            #[cfg(feature = "graphics_debug")]
            p_next: ptr::addr_of!(debug_messenger_info) as *const ffi::c_void,
            ..Default::default()
        };

        let instance = unsafe {
            vulkan_check!(entry.create_instance(&create_info, Some(&ALLOCATION_CALLBACKS)))
        };

        debug!("Created Vulkan instance successfully");
        instance
    }

    fn get_required_device_exts() -> [&'static ffi::CStr; 1] {
        [extensions::khr::Swapchain::name()]
    }

    fn get_gpus(
        instance: &ash::Instance,
        surface_loader: &extensions::khr::Surface,
        surface: &vk::SurfaceKHR,
    ) -> Vec<GpuInfo> {
        debug!("Enumerating devices");
        let devices = unsafe { vulkan_check!(instance.enumerate_physical_devices()) };

        let mut gpus: Vec<GpuInfo> = Vec::new();
        let mut usable_count = 0;
        for i in 0..devices.len() {
            let device = devices[i];
            let mut usable = true;

            debug!("Getting information for device {}", i + 1);
            let queue_family_props =
                unsafe { instance.get_physical_device_queue_family_properties(device) };
            if queue_family_props.len() < 1 {
                usable = false;
                error!("Ignoring GPU {} because it has no queue families", i + 1);
                continue;
            }

            let mut graphics_family_idx = None;
            let mut present_family_idx = None;
            for j in 0..queue_family_props.len() {
                let props = &queue_family_props[j];

                if props.queue_count < 1 {
                    continue;
                }

                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    graphics_family_idx = Some(j as u32);
                }

                let surface_support = unsafe {
                    surface_loader
                        .get_physical_device_surface_support(device, j as u32, *surface)
                        .unwrap_or_else(|err| {
                            panic!(
                                "Failed to check surface support for device {}: {}",
                                i + 1,
                                err
                            )
                        })
                };
                if surface_support {
                    present_family_idx = Some(j as u32);
                }

                if graphics_family_idx.is_some() && present_family_idx.is_some() {
                    break;
                }
            }

            if graphics_family_idx.is_none() || present_family_idx.is_none() {
                usable = false;
                error!("Failed to get all required queue familiy indices for device {} (graphics {}, present {})", i + 1, graphics_family_idx.unwrap_or(u32::MAX), present_family_idx.unwrap_or(u32::MAX));
                continue;
            }

            let extension_props = unsafe { instance.enumerate_device_extension_properties(device) };
            if extension_props.is_err() {
                usable = false;
                error!(
                    "Failed to get extension properties for device {}: {}",
                    i + 1,
                    extension_props.unwrap_err()
                );
                continue;
            }
            let ext_props = extension_props.unwrap();
            if ext_props.len() < 1 {
                usable = false;
                error!(
                    "Ignoring device {} because it has no extensions when {} are required",
                    i + 1,
                    Self::get_required_device_exts().len()
                );
                continue;
            }

            let surface_caps = unsafe {
                surface_loader.get_physical_device_surface_capabilities(device, *surface)
            };
            if surface_caps.is_err() {
                usable = false;
                error!(
                    "Failed to get surface capabilities for device {}: {}",
                    i + 1,
                    surface_caps.unwrap_err()
                );
                continue;
            }
            let surface_caps = surface_caps.unwrap();

            let fmts =
                unsafe { surface_loader.get_physical_device_surface_formats(device, *surface) };
            if fmts.is_err() {
                usable = false;
                error!(
                    "Failed to get surface formats for device {}: {}",
                    i + 1,
                    fmts.unwrap_err()
                );
                continue;
            }
            let surface_fmts = fmts.unwrap();
            if surface_fmts.len() < 1 {
                usable = false;
                error!(
                    "Ignoring device {} because it has no surface formats",
                    i + 1
                );
                continue;
            }

            let present_modes = unsafe {
                surface_loader.get_physical_device_surface_present_modes(device, *surface)
            };
            if present_modes.is_err() {
                usable = false;
                error!(
                    "Failed to get present modes for device {}: {}",
                    i + 1,
                    present_modes.unwrap_err()
                );
                continue;
            }
            let present_modes = present_modes.unwrap();
            if present_modes.len() < 1 {
                usable = false;
                error!("Ignoring device {} because it has no present modes", i + 1);
                continue;
            }

            let mem_props = unsafe { instance.get_physical_device_memory_properties(device) };
            let props = unsafe { instance.get_physical_device_properties(device) };
            gpus.push(GpuInfo {
                device: device,
                usable: usable,
                mem_props: mem_props,
                props: props,
                surface_caps: surface_caps,
                surface_fmts: surface_fmts,
                present_modes: present_modes,
                queue_family_props: queue_family_props,
                graphics_family_idx: graphics_family_idx,
                present_family_idx: present_family_idx,
                ext_props,
            });

            usable_count += 1;
        }

        debug!(
            "Got information of {} device(s) (of which {} are usable)",
            gpus.len(),
            usable_count
        );
        if usable_count < 1 {
            panic!("Could not find any usable Vulkan devices");
        }

        for mut i in 0..gpus.len() {
            if !gpus[i].usable {
                gpus.remove(i);
                i -= 1;
            }
        }

        let name = unsafe {
            ffi::CStr::from_ptr(gpus[0].props.device_name.as_ptr())
                .to_str()
                .unwrap()
        };
        debug!(
            "Selected device {} [{:04x}:{:04x}]",
            name, gpus[0].props.vendor_id, gpus[0].props.device_id
        );

        gpus
    }

    fn create_device(
        instance: &ash::Instance,
        gpu: &GpuInfo,
    ) -> (ash::Device, vk::Queue, vk::Queue) {
        debug!("Creating logical device");

        let queue_priority = 1.0f32;
        let graphics_queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index: gpu.graphics_family_idx.unwrap(),
            p_queue_priorities: ptr::addr_of!(queue_priority),
            queue_count: 1,
            ..Default::default()
        };
        let present_queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index: gpu.present_family_idx.unwrap(),
            p_queue_priorities: ptr::addr_of!(queue_priority),
            queue_count: 1,
            ..Default::default()
        };
        let queue_create_infos = if gpu.graphics_family_idx != gpu.present_family_idx {
            vec![graphics_queue_info, present_queue_info]
        } else {
            vec![graphics_queue_info]
        };

        let device_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let device_13_features = vk::PhysicalDeviceVulkan13Features {
            dynamic_rendering: vk::TRUE,
            ..Default::default()
        };

        let device_exts: Vec<*const ffi::c_char> = Self::get_required_device_exts()
            .iter()
            .map(|ext_name| ext_name.as_ptr())
            .collect();

        let device_info = vk::DeviceCreateInfo {
            p_queue_create_infos: queue_create_infos.as_ptr(),
            queue_create_info_count: queue_create_infos.len() as u32,
            p_enabled_features: ptr::addr_of!(device_features),
            pp_enabled_extension_names: device_exts.as_ptr(),
            enabled_extension_count: device_exts.len() as u32,
            ..Default::default()
        };

        let device = unsafe {
            vulkan_check!(instance.create_device(
                gpu.device,
                &device_info,
                Some(&ALLOCATION_CALLBACKS)
            ))
        };

        debug!("Retrieving queues");
        let graphics_queue =
            unsafe { device.get_device_queue(gpu.graphics_family_idx.unwrap(), 0) };
        let present_queue = unsafe { device.get_device_queue(gpu.present_family_idx.unwrap(), 0) };

        (device, graphics_queue, present_queue)
    }

    //fn create_fences(device: ash::Device) -> [vk::Fence; FRAME_COUNT] {}

    //fn create_semaphores(
    //    device: ash::Device,
    //) -> ([vk::Semaphore; FRAME_COUNT], [vk::Semaphore; FRAME_COUNT]) {
    //}

    //fn choose_surface_format(gpu: GpuInfo) -> vk::SurfaceFormatKHR {}

    //fn create_swapchain(
    //    device: ash::Device,
    //    surface_format: vk::SurfaceFormatKHR,
    //    swapchain_extent: vk::Extent2D,
    //) -> (vk::SwapchainKHR, Vec<vk::Image>) {
    //}

    pub fn init() -> Self {
        debug!("Vulkan initialization started");

        debug!("Loading Vulkan library");
        let entry = unsafe { vulkan_check!(ash::Entry::load()) };

        let instance = Self::create_instance(&entry);
        let surface_loader = extensions::khr::Surface::new(&entry, &instance);
        let surface = crate::platform::video::create_vulkan_surface(
            &entry,
            &instance,
            Some(&ALLOCATION_CALLBACKS),
        );
        let gpus = Self::get_gpus(&instance, &surface_loader, &surface);
        let gpu_idx = 0;
        let (device, graphics_queue, present_queue) =
            Self::create_device(&instance, &gpus[gpu_idx]);
        //let fences = Self::create_fences(device);
        //let (acquire_semaphores, render_complete_semaphores) = Self::create_semaphores(device);
        //let surface_format = Self::choose_surface_format(gpu);
        //let video_size = crate::platform::video::get_size();
        //let swapchain_extent = vk::Extent2D {
        //    width: video_size.0,
        //    height: video_size.1,
        //};
        //let (swapchain, swapchain_images) =
        //    Self::create_swapchain(device, surface_format, swapchain_extent);

        debug!("Vulkan initialization succeeded");

        Self {
            entry: entry,
            instance: instance,
            device: device,
            surface_loader: surface_loader,
            surface: surface,
            gpu: gpu_idx,
            gpus: gpus,
        }
    }

    pub fn begin_cmds() {}

    pub fn present() {}

    pub fn shutdown(&self) {
        debug!("Vulkan shutdown started");

        unsafe {
            debug!("Destroying logical device");
            self.device.destroy_device(Some(&ALLOCATION_CALLBACKS));
            debug!("Destroying surface");
            self.surface_loader
                .destroy_surface(self.surface, Some(&ALLOCATION_CALLBACKS));
            debug!("Destroying instance");
            self.instance.destroy_instance(Some(&ALLOCATION_CALLBACKS));
        }

        debug!("Vulkan shutdown succeeded");
    }

    pub fn set_gpu(&mut self, gpu_idx: usize) -> usize {
        let old_idx = self.gpu;
        if gpu_idx < self.gpus.len() {
            self.gpu = gpu_idx;
            let gpu = &self.gpus[self.gpu];
            let name = unsafe {
                ffi::CStr::from_ptr(gpu.props.device_name.as_ptr())
                    .to_str()
                    .unwrap()
            };
            debug!(
                "Selected device {} [{:04x}:{:04x}]",
                name, gpu.props.vendor_id, gpu.props.device_id
            );
        }

        old_idx
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
macro_rules! get_state {
    () => {
        STATE.lock().unwrap().as_ref().unwrap()
    };
}
