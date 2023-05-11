use crate::{engine::rendersystem, platform};
use ash::{extensions, vk};
use log::{debug, error, log, trace};
use std::rc::Rc;
use std::{alloc, ffi, mem, ptr, sync::Arc};
use vk_mem::*;

macro_rules! vulkan_check {
    ($call: expr) => {
        $call.unwrap_or_else(|err| panic!("Vulkan call {} failed: {}", stringify!($call), err))
    };
}

extern "system" fn vulkan_alloc(
    _p_user_data: *mut ffi::c_void,
    size: usize,
    alignment: usize,
    _allocation_scope: vk::SystemAllocationScope,
) -> *mut ffi::c_void {
    unsafe {
        alloc::alloc(alloc::Layout::from_size_align(size, alignment).unwrap()) as *mut ffi::c_void
    }
}

extern "system" fn vulkan_realloc(
    _p_user_data: *mut ffi::c_void,
    p_original: *mut ffi::c_void,
    size: usize,
    alignment: usize,
    _allocation_scope: vk::SystemAllocationScope,
) -> *mut ffi::c_void {
    unsafe {
        alloc::realloc(
            p_original as *mut u8,
            alloc::Layout::from_size_align(size, alignment).unwrap(),
            size,
        ) as *mut ffi::c_void
    }
}

extern "system" fn vulkan_dealloc(_p_user_data: *mut ffi::c_void, p_memory: *mut ffi::c_void) {
    unsafe {
        alloc::dealloc(
            p_memory as *mut u8,
            alloc::Layout::from_size_align(0, 1).unwrap(),
        )
    }
}

const FRAME_COUNT: usize = 3;

struct GpuInfo {
    device: vk::PhysicalDevice,

    properties: vk::PhysicalDeviceProperties,

    surface_formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,

    graphics_family_index: u32,
    compute_family_index: u32,

    // Vague guess at how powerful the GPU is
    performance_score: u32,
}

struct Image {
    handle: vk::Image,
    allocation: Option<vk_mem::Allocation>,
    view: vk::ImageView,
    format: vk::Format,
}

impl Image {
    pub fn new(
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        format: vk::Format,
        create_info: &mut vk::ImageCreateInfo,
        view_info: &mut vk::ImageViewCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
    ) -> Result<Self, vk::Result> {
        create_info.format = format;
        let result = unsafe { allocator.create_image(create_info, allocation_info) };
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        let (handle, allocation) = result.unwrap();
        view_info.image = handle;
        view_info.format = format;

        let view = unsafe {
            device.create_image_view(view_info, Some(&State::get_allocation_callbacks()))
        };
        if view.is_err() {
            return Err(view.unwrap_err());
        }
        let view = view.unwrap();

        Ok(Self {
            handle,
            allocation: Some(allocation),
            view,
            format,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_image_view(self.view, Some(&State::get_allocation_callbacks()));
            allocator.destroy_image(self.handle, self.allocation.take().unwrap());
        }
    }

    pub fn choose_fmt(
        instance: &ash::Instance,
        gpu: &GpuInfo,
        formats: &Vec<vk::Format>,
        tiling: vk::ImageTiling,
        required_features: vk::FormatFeatureFlags,
    ) -> vk::Format {
        debug!("Finding format with feature flags {:#?}", required_features);

        let formats: Vec<&vk::Format> = formats
            .iter()
            .filter(|format| {
                let props =
                    unsafe { instance.get_physical_device_format_properties(gpu.device, **format) };
                match tiling {
                    vk::ImageTiling::LINEAR => {
                        props.linear_tiling_features.contains(required_features)
                    }
                    vk::ImageTiling::OPTIMAL => {
                        props.optimal_tiling_features.contains(required_features)
                    }
                    _ => false,
                }
            })
            .collect();

        if !formats.is_empty() {
            *formats[0]
        } else {
            vk::Format::UNDEFINED
        }
    }

    pub fn handle(&self) -> &vk::Image {
        &self.handle
    }

    pub fn allocation(&self) -> &Option<vk_mem::Allocation> {
        &self.allocation
    }

    pub fn allocation_mut(&mut self) -> &mut Option<vk_mem::Allocation> {
        &mut self.allocation
    }

    pub fn view(&self) -> &vk::ImageView {
        &self.view
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }
}

struct Buffer {
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,
    size: vk::DeviceSize,
}

impl Buffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        flags: vk::MemoryPropertyFlags,
    ) -> Result<Self, vk::Result> {
        let result = unsafe {
            allocator.create_buffer(
                &vk::BufferCreateInfo {
                    size,
                    usage,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                },
                &vk_mem::AllocationCreateInfo {
                    required_flags: flags,
                    ..Default::default()
                },
            )
        };

        if result.is_err() {
            return Err(result.unwrap_err());
        }

        let (handle, allocation) = result.unwrap();
        Ok(Self {
            handle,
            allocation,
            size,
        })
    }

    pub fn copy(
        &self,
        device: &ash::Device,
        queue: &vk::Queue,
        transfer_pool: &vk::CommandPool,
        fence: &vk::Fence,
        destination: &Self,
    ) {
        let transfer_buffer = unsafe {
            vulkan_check!(
                device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_pool: transfer_pool.clone(),
                    command_buffer_count: FRAME_COUNT as u32,
                    ..Default::default()
                })
            )
        }[0];

        unsafe {
            vulkan_check!(device.begin_command_buffer(
                transfer_buffer,
                &vk::CommandBufferBeginInfo {
                    flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                }
            ));

            device.cmd_copy_buffer(
                transfer_buffer,
                self.handle,
                destination.handle,
                &[vk::BufferCopy {
                    size: self.size,
                    ..Default::default()
                }],
            );

            vulkan_check!(device.end_command_buffer(transfer_buffer));
            vulkan_check!(device.queue_submit(
                queue.clone(),
                &[vk::SubmitInfo {
                    command_buffer_count: 1,
                    p_command_buffers: ptr::addr_of!(transfer_buffer),
                    ..Default::default()
                }],
                fence.clone()
            ));
            vulkan_check!(device.queue_wait_idle(queue.clone()));

            device.free_command_buffers(transfer_pool.clone(), &[transfer_buffer]);
        }
    }

    pub fn destroy(self, allocator: &vk_mem::Allocator) {
        unsafe { allocator.destroy_buffer(self.handle, self.allocation) };
    }

    pub fn handle(&self) -> &vk::Buffer {
        &self.handle
    }

    pub fn allocation(&self) -> &vk_mem::Allocation {
        &self.allocation
    }

    pub fn allocation_mut(&mut self) -> &mut vk_mem::Allocation {
        &mut self.allocation
    }

    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
}

struct HostBuffer {
    buffer: Buffer,
    address: u64,
}

impl HostBuffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        flags: vk::MemoryPropertyFlags,
    ) -> Result<Self, vk::Result> {
        Self::from_buffer(
            allocator,
            match Buffer::new(allocator, size, usage, flags) {
                Ok(buffer) => buffer,
                Err(err) => return Err(err),
            },
        )
    }

    pub fn from_buffer(allocator: &vk_mem::Allocator, buffer: Buffer) -> Result<Self, vk::Result> {
        let mut self_ = Self { buffer, address: 0 };
        self_.address = unsafe {
            match allocator.map_memory(&mut self_.buffer.allocation) {
                Ok(address) => address as u64,
                Err(err) => return Err(err),
            }
        };
        Ok(self_)
    }

    pub fn destroy(mut self, allocator: &vk_mem::Allocator) {
        unsafe { allocator.unmap_memory(&mut self.buffer.allocation) };
        self.buffer.destroy(allocator);
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub fn address(&self) -> u64 {
        self.address
    }
}

pub struct State {
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: extensions::khr::Surface,
    swapchain_loader: extensions::khr::Swapchain,
    surface: vk::SurfaceKHR,

    allocator: vk_mem::Allocator,

    gpu: usize,
    gpus: Vec<GpuInfo>,
    graphics_queue: vk::Queue,
    compute_queue: vk::Queue,

    command_pool: vk::CommandPool,
    transfer_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    fences: Vec<vk::Fence>,
    acquire_semaphores: Vec<vk::Semaphore>,
    render_complete_semaphores: Vec<vk::Semaphore>,

    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_views: Vec<vk::ImageView>,
    swapchain_index: usize,
    surface_format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    swapchain_extent: vk::Extent2D,

    depth_image: Image,

    descriptor_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    uniform_buffers: Vec<HostBuffer>,

    frame_index: usize,
    resized: bool,

    last_shader: Option<Arc<rendersystem::Shader>>,
    last_model: Option<Arc<rendersystem::Model>>,
}

impl State {
    fn get_allocation_callbacks<'a>() -> vk::AllocationCallbacks<'a> {
        vk::AllocationCallbacks {
            pfn_allocation: Some(vulkan_alloc),
            pfn_reallocation: Some(vulkan_realloc),
            pfn_free: Some(vulkan_dealloc),
            p_user_data: ptr::null_mut(),
            pfn_internal_allocation: None,
            pfn_internal_free: None,
            ..Default::default()
        }
    }

    unsafe extern "system" fn debug_log(
        severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        types: vk::DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        #[allow(unused_variables)] user_data: *mut ffi::c_void,
    ) -> u32 {
        let log_level = match severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::Level::Trace,
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::Level::Debug,
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::Level::Info,
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::Level::Warn,
            _ => log::Level::Debug,
        };

        let mut location = String::new();

        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL) {
            location += "GENERAL ";
        }
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE) {
            location += "PERFORMANCE ";
        }
        if types.contains(vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION) {
            location += "VALIDATION ";
        }

        let message_ptr = (*callback_data).p_message as *const ffi::c_char;
        let message_raw = unsafe { ffi::CStr::from_ptr(message_ptr) };
        let message = message_raw.to_str().unwrap();
        log!(log_level, "VULKAN {}MESSAGE: {}", location, message);

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
            #[cfg(feature = "graphics_debug")]
            "VK_EXT_debug_utils",
            "VK_KHR_surface",
            #[cfg(windows)]
            "VK_KHR_win32_surface",
            #[cfg(unix)]
            "VK_KHR_xcb_surface",
        ];

        let validation_layers = ["VK_LAYER_KHRONOS_validation"];

        let extensions_cstr: Vec<ffi::CString> = extensions
            .iter()
            .map(|extension_name| ffi::CString::new(*extension_name).unwrap())
            .collect();
        let extensions_raw: Vec<*const ffi::c_char> = extensions_cstr
            .iter()
            .map(|extension_name| extension_name.as_ptr())
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
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            pfn_user_callback: Some(Self::debug_log),
            ..Default::default()
        };

        let mut create_info = vk::InstanceCreateInfo {
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

        let result = unsafe {
            entry.create_instance(&create_info, Some(&State::get_allocation_callbacks()))
        };
        let instance = match result {
            Ok(val) => val,
            Err(err) if err == vk::Result::ERROR_LAYER_NOT_PRESENT => {
                debug!("Validation layers not available, retrying with them disabled");
                create_info.enabled_layer_count = 0;
                unsafe {
                    vulkan_check!(entry.create_instance(&create_info, Some(&State::get_allocation_callbacks())))
                }
            }
            Err(err) =>
                panic!("Vulkan call entry.create_instance(&create_info, Some(&State::get_allocation_callbacks())) failed: {err}")
        };

        debug!(
            "Created Vulkan instance {:?} successfully",
            instance.handle()
        );
        instance
    }

    fn get_required_device_exts() -> [&'static str; 2] {
        ["VK_KHR_swapchain", "VK_EXT_shader_object"]
    }

    fn get_gpus(
        instance: &ash::Instance,
        surface_loader: &extensions::khr::Surface,
        surface: &vk::SurfaceKHR,
    ) -> Vec<GpuInfo> {
        debug!("Enumerating devices (scores loosely based on memory, maximum viewport size, and discrete/integrated/CPU)");
        let devices = unsafe { vulkan_check!(instance.enumerate_physical_devices()) };
        let devices = devices
            .into_iter()
            .enumerate()
            .map(|(i, device)| (i + 1, device));

        let mut gpus: Vec<GpuInfo> = Vec::new();
        let mut usable_count = 0;
        for (i, device) in devices {
            debug!("Getting information for device {i}");
            let queue_family_props =
                unsafe { instance.get_physical_device_queue_family_properties(device) };
            if queue_family_props.is_empty() {
                error!("Ignoring GPU {i} because it has no queue families");
                continue;
            }

            let Some((graphics_family_index, _)) = queue_family_props
                .iter()
                .enumerate()
                .map(|(i, props)| (i as u32, props))
                .find(|(_, props)| {
                    props.queue_count >= 1
                        && props.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                }) else {
                error!("Failed to get all required queue familiy indices for device {i}");
                continue;
            };

            let Some((compute_family_index, _)) = queue_family_props
                .iter()
                .enumerate()
                .map(|(i, props)| (i as u32, props))
                .find(|(_, props)| {
                    props.queue_count >= 1
                        && props.queue_flags.contains(vk::QueueFlags::COMPUTE)
                }) else {
                error!("Failed to get all required queue familiy indices for device {i}");
                continue;
            };

            let extension_props = unsafe { instance.enumerate_device_extension_properties(device) };
            match extension_props {
                Ok(val) if val.len() >= Self::get_required_device_exts().len() => val,
                Ok(val) => {
                    error!(
                        "Ignoring device {} because it has {} extension(s) when {} are required",
                        i,
                        val.len(),
                        Self::get_required_device_exts().len()
                    );
                    continue;
                }
                Err(err) => {
                    error!("Failed to get extension properties for device {i}: {err}");
                    continue;
                }
            };

            let surface_caps = unsafe {
                surface_loader.get_physical_device_surface_capabilities(device, *surface)
            };
            let surface_caps = match surface_caps {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to get surface capabilities for device {i}: {err}");
                    continue;
                }
            };

            let fmts =
                unsafe { surface_loader.get_physical_device_surface_formats(device, *surface) };
            let surface_formats = match fmts {
                Ok(val) if !val.is_empty() => val,
                Ok(_) => {
                    error!("Ignoring device {i} because it has no surface formats");
                    continue;
                }
                Err(err) => {
                    error!("Failed to get surface formats for device {i}: {err}");
                    continue;
                }
            };

            let present_modes = unsafe {
                surface_loader.get_physical_device_surface_present_modes(device, *surface)
            };
            let present_modes = match present_modes {
                Ok(val) if !val.is_empty() => val,
                Ok(_) => {
                    error!("Ignoring device {i} because it has no present modes");
                    continue;
                }
                Err(err) => {
                    error!("Failed to get present modes for device {i}: {err}");
                    continue;
                }
            };

            let memory_properties =
                unsafe { instance.get_physical_device_memory_properties(device) };
            let properties = unsafe { instance.get_physical_device_properties(device) };

            let mut score = (memory_properties.memory_heaps[0].size / 1_000) as u32
                + (properties.limits.max_viewport_dimensions[0] as u64
                    * properties.limits.max_viewport_dimensions[1] as u64
                    / 1_000) as u32;
            if [
                vk::PhysicalDeviceType::DISCRETE_GPU,
                vk::PhysicalDeviceType::VIRTUAL_GPU,
            ]
            .contains(&properties.device_type)
            {
                score *= 10;
            } else if properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
                score *= 2;
            }

            let name = unsafe {
                ffi::CStr::from_ptr(properties.device_name.as_ptr())
                    .to_str()
                    .unwrap()
            };

            // Doesn't work on my systems, but it should
            let mut properties2 = vk::PhysicalDeviceProperties2::default();
            unsafe { instance.get_physical_device_properties2(device, &mut properties2) };
            let mut p_next = properties2.p_next;
            let mut driver_info_ptr = ptr::null();
            while !p_next.is_null() {
                let tagged = unsafe { (*(p_next as *mut vk::BaseOutStructure)) };
                if tagged.s_type == vk::StructureType::PHYSICAL_DEVICE_DRIVER_PROPERTIES_KHR {
                    driver_info_ptr = p_next;
                    break;
                }

                p_next = tagged.p_next as *mut ffi::c_void;
            }

            let driver_info = if !driver_info_ptr.is_null() {
                unsafe { *(driver_info_ptr as *mut vk::PhysicalDeviceDriverProperties) }
            } else {
                vk::PhysicalDeviceDriverProperties::default()
            };

            debug!("Device {i}:");
            debug!("\tName: {name}");
            trace!("\tDriver information:\n{driver_info:#?}");
            debug!("\tScore: {score}");
            debug!("\tType: {:#?}", properties.device_type);
            debug!("\tHandle: {device:#?}");

            gpus.push(GpuInfo {
                device,
                properties,
                surface_formats,
                present_modes,
                graphics_family_index,
                compute_family_index,
                performance_score: score,
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

        debug!("Sorting device(s)");
        gpus.sort_by_key(|gpu| -(gpu.performance_score as i32));

        gpus
    }

    fn create_device(
        instance: &ash::Instance,
        gpu: &GpuInfo,
    ) -> (ash::Device, vk::Queue, vk::Queue) {
        debug!("Creating logical device");

        let queue_priority: f32 = 1.0;
        let graphics_queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index: gpu.graphics_family_index,
            p_queue_priorities: ptr::addr_of!(queue_priority),
            queue_count: 1,
            ..Default::default()
        };
        let present_queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index: gpu.compute_family_index,
            p_queue_priorities: ptr::addr_of!(queue_priority),
            queue_count: 1,
            ..Default::default()
        };
        let queue_create_infos = if gpu.graphics_family_index != gpu.compute_family_index {
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

        let extensions_cstr: Vec<ffi::CString> = Self::get_required_device_exts()
            .iter()
            .map(|extension_name| ffi::CString::new(*extension_name).unwrap())
            .collect();
        let extensions_raw: Vec<*const ffi::c_char> = extensions_cstr
            .iter()
            .map(|extension_name| extension_name.as_ptr())
            .collect();

        let device_info = vk::DeviceCreateInfo {
            p_queue_create_infos: queue_create_infos.as_ptr(),
            queue_create_info_count: queue_create_infos.len() as u32,
            p_enabled_features: ptr::addr_of!(device_features),
            pp_enabled_extension_names: extensions_raw.as_ptr(),
            enabled_extension_count: extensions_raw.len() as u32,
            p_next: ptr::addr_of!(device_13_features) as *const ffi::c_void,
            ..Default::default()
        };

        let device = unsafe {
            vulkan_check!(instance.create_device(
                gpu.device,
                &device_info,
                Some(&State::get_allocation_callbacks())
            ))
        };

        debug!("Created logical device {:#?} successfully", device.handle());

        debug!("Retrieving queues");
        let graphics_queue = unsafe { device.get_device_queue(gpu.graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(gpu.compute_family_index, 0) };
        debug!(
            "Got graphics queue {:#?} and present queue {:#?}",
            graphics_queue, present_queue
        );

        (device, graphics_queue, present_queue)
    }

    fn create_fences(device: &ash::Device) -> Vec<vk::Fence> {
        debug!("Creating {FRAME_COUNT} command fences");

        let fence_create_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        let mut fences = Vec::new();
        for _ in 0..FRAME_COUNT {
            fences.push(unsafe {
                vulkan_check!(device
                    .create_fence(&fence_create_info, Some(&State::get_allocation_callbacks())))
            })
        }

        fences
    }

    fn create_semaphores(device: &ash::Device) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>) {
        debug!("Creating {} semaphores", FRAME_COUNT * 2);

        let semaphore_create_info = vk::SemaphoreCreateInfo {
            ..Default::default()
        };
        let mut acquire_semaphores = Vec::new();
        let mut complete_semaphores = Vec::new();
        acquire_semaphores.resize_with(3, || unsafe {
            vulkan_check!(device.create_semaphore(
                &semaphore_create_info,
                Some(&State::get_allocation_callbacks())
            ))
        });
        complete_semaphores.resize_with(3, || unsafe {
            vulkan_check!(device.create_semaphore(
                &semaphore_create_info,
                Some(&State::get_allocation_callbacks())
            ))
        });

        (acquire_semaphores, complete_semaphores)
    }

    fn create_command_pools(
        device: &ash::Device,
        gpu: &GpuInfo,
    ) -> (vk::CommandPool, vk::CommandPool) {
        debug!("Creating command pools");

        let main_pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: gpu.graphics_family_index,
            ..Default::default()
        };
        let transfer_pool_info = vk::CommandPoolCreateInfo {
            flags: main_pool_info.flags | vk::CommandPoolCreateFlags::TRANSIENT,
            ..main_pool_info
        };

        let main_pool = unsafe {
            vulkan_check!(device
                .create_command_pool(&main_pool_info, Some(&State::get_allocation_callbacks())))
        };
        let transfer_pool = unsafe {
            vulkan_check!(device.create_command_pool(
                &transfer_pool_info,
                Some(&State::get_allocation_callbacks())
            ))
        };

        debug!(
            "Created main command pool {main_pool:#?} and transfer command pool {transfer_pool:#?}"
        );

        (main_pool, transfer_pool)
    }

    fn allocate_command_buffers(
        device: &ash::Device,
        cmd_pool: &vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        debug!("Allocating {FRAME_COUNT} command buffers");

        unsafe {
            vulkan_check!(
                device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_pool: cmd_pool.clone(),
                    command_buffer_count: FRAME_COUNT as u32,
                    ..Default::default()
                })
            )
        }
    }

    fn create_allocator(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
    ) -> vk_mem::Allocator {
        debug!("Creating Vulkan allocator");
        vulkan_check!(vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
            Rc::from(instance),
            Rc::from(device),
            physical_device
        )))
    }

    fn choose_surface_format(gpu: &GpuInfo) -> vk::SurfaceFormatKHR {
        debug!("Choosing surface format");

        if gpu.surface_formats.len() == 1 && gpu.surface_formats[0].format == vk::Format::UNDEFINED
        {
            return vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }

        for &format in &gpu.surface_formats {
            if format.format == vk::Format::B8G8R8A8_UNORM
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                debug!("Chose format {format:#?}");
                return format;
            }
        }

        debug!("Chose format {:#?}", gpu.surface_formats[0]);
        gpu.surface_formats[0]
    }

    fn choose_present_mode(gpu: &GpuInfo) -> vk::PresentModeKHR {
        debug!("Choosing presentation mode");

        for &mode in &gpu.present_modes {
            if mode == vk::PresentModeKHR::MAILBOX {
                debug!("Chose present mode {mode:#?}");
                return mode;
            }
        }

        debug!("Chose FIFO present mode");
        vk::PresentModeKHR::FIFO
    }

    fn create_swapchain(
        device: &ash::Device,
        gpu: &GpuInfo,
        surface: &vk::SurfaceKHR,
        present_mode: &vk::PresentModeKHR,
        surface_format: &vk::SurfaceFormatKHR,
        image_extent: &vk::Extent2D,
        loader: &extensions::khr::Swapchain,
    ) -> (vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>) {
        debug!("Creating swap chain");

        let queue_family_indices = [gpu.graphics_family_index, gpu.compute_family_index];
        let (image_sharing_mode, queue_family_index_count, p_queue_family_indices) =
            if gpu.graphics_family_index != gpu.compute_family_index {
                (
                    vk::SharingMode::CONCURRENT,
                    2,
                    queue_family_indices.as_ptr(),
                )
            } else {
                (vk::SharingMode::EXCLUSIVE, 0, ptr::null())
            };

        let swapchain_info = vk::SwapchainCreateInfoKHR {
            surface: *surface,
            min_image_count: FRAME_COUNT as u32,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: *image_extent,
            image_array_layers: 1,

            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,

            image_sharing_mode,
            queue_family_index_count,
            p_queue_family_indices,

            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: *present_mode,

            clipped: vk::TRUE,

            ..Default::default()
        };

        let swapchain = unsafe {
            vulkan_check!(
                loader.create_swapchain(&swapchain_info, Some(&State::get_allocation_callbacks()))
            )
        };
        let images = unsafe { vulkan_check!(loader.get_swapchain_images(swapchain)) };

        debug!("Creating swap chain image views");
        let mut views = Vec::new();
        let mut i = 0;
        views.resize_with(FRAME_COUNT, || unsafe {
            let image = images[i];
            i += 1;
            vulkan_check!(device.create_image_view(
                &vk::ImageViewCreateInfo {
                    image,

                    view_type: vk::ImageViewType::TYPE_2D,

                    format: surface_format.format,

                    components: vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    },

                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },

                    ..Default::default()
                },
                Some(&State::get_allocation_callbacks())
            ))
        });

        debug!("Created swapchain {swapchain:#?}");

        (swapchain, images, views)
    }

    fn destroy_swapchain(&mut self) {
        debug!("Destroying {FRAME_COUNT} swap chain image views");
        self.swapchain_views.iter().for_each(|view| unsafe {
            self.device
                .destroy_image_view(*view, Some(&State::get_allocation_callbacks()))
        });

        debug!("Destroying swap chain {:#?}", self.swapchain);
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, Some(&State::get_allocation_callbacks()))
        };
    }

    fn create_render_targets(
        instance: &ash::Instance,
        gpu: &GpuInfo,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
    ) -> (Image) {
        debug!("Creating render target images");

        let depth_formats = vec![
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];

        let depth_format = Image::choose_fmt(
            instance,
            gpu,
            &depth_formats,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        );
        if depth_format == vk::Format::UNDEFINED {
            panic!("No supported depth formats found");
        }

        debug!("Creating depth image");
        let (width, height) = crate::platform::video::get_size();
        let depth_image = vulkan_check!(Image::new(
            device,
            allocator,
            depth_format,
            &mut vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                image_type: vk::ImageType::TYPE_2D,
                ..Default::default()
            },
            &mut vk::ImageViewCreateInfo {
                view_type: vk::ImageViewType::TYPE_2D,
                subresource_range: vk::ImageSubresourceRange {
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    ..Default::default()
                },
                ..Default::default()
            },
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            }
        ));
        debug!("Created depth image {:#?}", depth_image.handle());

        (depth_image)
    }

    fn create_descriptor_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        debug!("Creating descriptor set layout");

        let ubo_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        };

        let descriptor_layout_info = vk::DescriptorSetLayoutCreateInfo {
            p_bindings: ptr::addr_of!(ubo_layout_binding),
            binding_count: 1,
            ..Default::default()
        };

        let layout = unsafe {
            vulkan_check!(device.create_descriptor_set_layout(
                &descriptor_layout_info,
                Some(&State::get_allocation_callbacks())
            ))
        };

        debug!("Created descriptor set layout {layout:#?}");

        layout
    }

    fn destroy_render_targets(&mut self) {
        debug!("Destroying render target images");
        debug!("Destroying depth image {:#?}", self.depth_image.handle());
        self.depth_image.destroy(&self.device, &self.allocator);
    }

    fn resize(&mut self) {
        debug!("Recreating swap chain");

        debug!("Waiting for device idle");
        unsafe { vulkan_check!(self.device.device_wait_idle()) };

        self.destroy_render_targets();
        self.destroy_swapchain();
        let (width, height) = platform::video::get_size();
        self.swapchain_extent = vk::Extent2D { width, height };
        (self.swapchain, self.swapchain_images, self.swapchain_views) = Self::create_swapchain(
            &self.device,
            &self.gpus[self.gpu],
            &self.surface,
            &self.present_mode,
            &self.surface_format,
            &self.swapchain_extent,
            &self.swapchain_loader,
        );
        (self.depth_image) = Self::create_render_targets(
            &self.instance,
            &self.gpus[self.gpu],
            &self.device,
            &self.allocator,
        );
    }

    fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
        debug!("Creating descriptor pool");

        const POOL_SIZES: [vk::DescriptorPoolSize; 11] = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_TEXEL_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 1000,
            },
        ];

        let pool = unsafe {
            vulkan_check!(device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo {
                    flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                    pool_size_count: POOL_SIZES.len() as u32,
                    p_pool_sizes: POOL_SIZES.as_ptr(),
                    max_sets: 1000 * POOL_SIZES.len() as u32,
                    ..Default::default()
                },
                Some(&State::get_allocation_callbacks())
            ))
        };

        debug!("Created descriptor pool {pool:#?}");

        pool
    }

    fn allocate_uniform_buffers(allocator: &vk_mem::Allocator) -> Vec<HostBuffer> {
        debug!("Allocating {FRAME_COUNT} uniform buffers");
        let mut buffers = Vec::new();
        buffers.resize_with(3, || {
            vulkan_check!(HostBuffer::new(
                allocator,
                mem::size_of::<rendersystem::UniformData>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ))
        });

        buffers
    }

    fn allocate_descriptor_sets(
        device: &ash::Device,
        layout: &vk::DescriptorSetLayout,
        pool: &vk::DescriptorPool,
        uniform_buffers: &Vec<HostBuffer>,
    ) -> Vec<vk::DescriptorSet> {
        debug!("Allocating {FRAME_COUNT} descriptor sets");

        let mut layouts = Vec::new();
        layouts.resize_with(3, || layout.clone());

        let descriptor_sets = unsafe {
            vulkan_check!(
                device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                    descriptor_pool: pool.clone(),
                    descriptor_set_count: FRAME_COUNT as u32,
                    p_set_layouts: layouts.as_ptr(),
                    ..Default::default()
                })
            )
        };

        let mut i = 0;
        let mut buffer_infos = Vec::new();
        buffer_infos.resize_with(FRAME_COUNT, || {
            let info = vk::DescriptorBufferInfo {
                offset: 0,
                range: mem::size_of::<rendersystem::UniformData>() as u64,
                buffer: uniform_buffers[i].buffer().handle().clone(),
            };
            i += 1;

            info
        });
        i = 0;
        let mut write_infos = Vec::new();
        write_infos.resize_with(FRAME_COUNT, || {
            let write = vk::WriteDescriptorSet {
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: ptr::addr_of!(buffer_infos[i]),
                dst_set: descriptor_sets[i],
                ..Default::default()
            };
            i += 1;
            write
        });

        unsafe { device.update_descriptor_sets(write_infos.as_slice(), &[]) };

        descriptor_sets
    }

    pub fn init() -> Self {
        debug!("Vulkan initialization started");

        debug!("Loading Vulkan library");
        let entry = unsafe { vulkan_check!(ash::Entry::load()) };

        let instance = Self::create_instance(&entry);
        let surface_loader = extensions::khr::Surface::new(&entry, &instance);
        let surface = crate::platform::video::create_vulkan_surface(
            &entry,
            &instance,
            Some(&State::get_allocation_callbacks()),
        );
        let gpus = Self::get_gpus(&instance, &surface_loader, &surface);
        let gpu = 0;
        let (device, graphics_queue, compute_queue) = Self::create_device(&instance, &gpus[gpu]);
        let (command_pool, transfer_pool) = Self::create_command_pools(&device, &gpus[gpu]);
        let command_buffers = Self::allocate_command_buffers(&device, &command_pool);
        let allocator = Self::create_allocator(&instance, &device, gpus[gpu].device);
        let fences = Self::create_fences(&device);
        let (acquire_semaphores, render_complete_semaphores) = Self::create_semaphores(&device);
        let surface_format = Self::choose_surface_format(&gpus[gpu]);
        let present_mode = Self::choose_present_mode(&gpus[gpu]);
        let video_size = crate::platform::video::get_size();
        let swapchain_extent = vk::Extent2D {
            width: video_size.0,
            height: video_size.1,
        };
        let swapchain_loader = extensions::khr::Swapchain::new(&instance, &device);
        let (swapchain, swapchain_images, swapchain_views) = Self::create_swapchain(
            &device,
            &gpus[gpu],
            &surface,
            &present_mode,
            &surface_format,
            &swapchain_extent,
            &swapchain_loader,
        );
        let (depth_image) = Self::create_render_targets(&instance, &gpus[gpu], &device, &allocator);
        let descriptor_layout = Self::create_descriptor_layout(&device);
        let descriptor_pool = Self::create_descriptor_pool(&device);
        let uniform_buffers = Self::allocate_uniform_buffers(&allocator);
        let descriptor_sets = Self::allocate_descriptor_sets(
            &device,
            &descriptor_layout,
            &descriptor_pool,
            &uniform_buffers,
        );

        debug!("Vulkan initialization succeeded");

        let mut self_ = Self {
            entry,
            instance,
            device,
            surface_loader,
            swapchain_loader,
            surface,
            gpu,
            gpus,
            graphics_queue,
            compute_queue,
            command_pool,
            transfer_pool,
            command_buffers,
            fences,
            acquire_semaphores,
            render_complete_semaphores,
            allocator,
            swapchain,
            swapchain_images,
            swapchain_views,
            surface_format,
            present_mode,
            swapchain_extent,
            depth_image,
            descriptor_layout,
            descriptor_pool,
            descriptor_sets,
            uniform_buffers,

            frame_index: 0,
            resized: false,
            swapchain_index: 0,

            last_shader: None,
            last_model: None,
        };
        self_.set_gpu(self_.gpu);

        self_
    }

    pub fn begin_cmds(&mut self) {
        unsafe {
            vulkan_check!(self.device.wait_for_fences(
                &[self.fences[self.frame_index]],
                true,
                u64::MAX
            ))
        };

        (self.swapchain_index, self.resized) = unsafe {
            match self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.acquire_semaphores[self.frame_index],
                vk::Fence::null(),
            ) {
                Ok(values) => (values.0 as usize, values.1),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => (0, true),
                Err(err) => {
                    panic!("Failed to acquire next image: {err}");
                }
            }
        };
        if self.resized {
            self.resize();
            return;
        }

        unsafe {
            vulkan_check!(self.device.reset_fences(&[self.fences[self.frame_index]]));
            vulkan_check!(self.device.reset_command_buffer(
                self.command_buffers[self.frame_index],
                vk::CommandBufferResetFlags::empty()
            ));
            vulkan_check!(self.device.begin_command_buffer(
                self.command_buffers[self.frame_index],
                &vk::CommandBufferBeginInfo {
                    flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                }
            ));
        }

        let layout_barrier = vk::ImageMemoryBarrier {
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            image: self.swapchain_images[self.swapchain_index],
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                self.command_buffers[self.frame_index],
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_barrier],
            )
        };

        let color_attachment = vk::RenderingAttachmentInfo {
            image_view: self.swapchain_views[self.swapchain_index],
            image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            clear_value: vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            ..Default::default()
        };
        let depth_attachment = vk::RenderingAttachmentInfo {
            image_view: self.depth_image.view().clone(),
            image_layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            clear_value: vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
            ..Default::default()
        };
        let rendering_info = vk::RenderingInfo {
            color_attachment_count: 1,
            p_color_attachments: ptr::addr_of!(color_attachment),
            p_depth_attachment: ptr::addr_of!(depth_attachment),
            layer_count: 1,
            render_area: vk::Rect2D {
                extent: self.swapchain_extent,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            self.device
                .cmd_begin_rendering(self.command_buffers[self.frame_index], &rendering_info)
        };
    }

    pub fn present(&mut self) {
        if self.resized {
            self.resized = false;
            return;
        }

        unsafe {
            self.device
                .cmd_end_rendering(self.command_buffers[self.frame_index])
        };

        let layout_barrier = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            image: self.swapchain_images[self.swapchain_index],
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                self.command_buffers[self.frame_index],
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_barrier],
            );

            vulkan_check!(self
                .device
                .end_command_buffer(self.command_buffers[self.frame_index]));
        };

        let wait_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        let submit_info = vk::SubmitInfo {
            p_wait_dst_stage_mask: ptr::addr_of!(wait_stage),
            wait_semaphore_count: 1,
            p_wait_semaphores: ptr::addr_of!(self.acquire_semaphores[self.frame_index]),
            signal_semaphore_count: 1,
            p_signal_semaphores: ptr::addr_of!(self.render_complete_semaphores[self.frame_index]),
            command_buffer_count: 1,
            p_command_buffers: ptr::addr_of!(self.command_buffers[self.frame_index]),
            ..Default::default()
        };

        unsafe {
            vulkan_check!(self.device.queue_submit(
                self.compute_queue,
                &[submit_info],
                self.fences[self.frame_index]
            ))
        }

        let index = self.swapchain_index as u32;
        let present_info = vk::PresentInfoKHR {
            p_swapchains: ptr::addr_of!(self.swapchain),
            swapchain_count: 1,
            p_wait_semaphores: ptr::addr_of!(self.render_complete_semaphores[self.frame_index]),
            wait_semaphore_count: 1,
            p_image_indices: ptr::addr_of!(index),
            ..Default::default()
        };

        match unsafe {
            self.swapchain_loader
                .queue_present(self.compute_queue, &present_info)
        } {
            Ok(_) => {}
            Err(err) if err == vk::Result::ERROR_OUT_OF_DATE_KHR => {}
            Err(err) => {
                panic!(
                    "Failed to present frame {} (swapchain image {}): {err}",
                    self.frame_index, self.swapchain_index
                )
            }
        }

        self.frame_index = (self.frame_index + 1) % FRAME_COUNT;
    }

    pub fn shutdown(mut self) {
        debug!("Vulkan shutdown started");

        debug!("Waiting for device idle");
        unsafe { vulkan_check!(self.device.device_wait_idle()) };

        unsafe {
            debug!("Freeing {FRAME_COUNT} uniform buffers");
            for _ in 0..self.uniform_buffers.len() {
                self.uniform_buffers.remove(0).destroy(&self.allocator)
            }

            debug!("Destroying descriptor pool {:#?}", self.descriptor_pool);
            self.device.destroy_descriptor_pool(
                self.descriptor_pool,
                Some(&State::get_allocation_callbacks()),
            );

            debug!(
                "Destroying descriptor set layout {:#?}",
                self.descriptor_layout
            );
            self.device.destroy_descriptor_set_layout(
                self.descriptor_layout,
                Some(&State::get_allocation_callbacks()),
            );

            self.destroy_render_targets();
            self.destroy_swapchain();
            debug!("Destroying {} semaphores", FRAME_COUNT * 2);
            self.acquire_semaphores.iter().for_each(|semaphore| {
                self.device
                    .destroy_semaphore(*semaphore, Some(&State::get_allocation_callbacks()))
            });
            self.render_complete_semaphores
                .iter()
                .for_each(|semaphore| {
                    self.device
                        .destroy_semaphore(*semaphore, Some(&State::get_allocation_callbacks()))
                });

            debug!("Destroying {FRAME_COUNT} fences");
            self.fences.iter().for_each(|fence| {
                self.device
                    .destroy_fence(*fence, Some(&State::get_allocation_callbacks()))
            });
            self.acquire_semaphores.iter().for_each(|semaphore| {
                self.device
                    .destroy_semaphore(*semaphore, Some(&State::get_allocation_callbacks()))
            });
            debug!("Destroying transfer command pool {:#?}", self.transfer_pool);
            self.device
                .destroy_command_pool(self.transfer_pool, Some(&State::get_allocation_callbacks()));
            debug!("Destroying command pool {:#?}", self.command_pool);
            self.device
                .destroy_command_pool(self.command_pool, Some(&State::get_allocation_callbacks()));
            debug!("Destroying allocator");
            ptr::drop_in_place(ptr::addr_of_mut!(self.allocator));
            debug!("Destroying logical device {:#?}", self.device.handle());
            self.device
                .destroy_device(Some(&State::get_allocation_callbacks()));
            debug!("Destroying surface {:#?}", self.surface);
            self.surface_loader
                .destroy_surface(self.surface, Some(&State::get_allocation_callbacks()));
            debug!("Destroying instance {:#?}", self.instance.handle());
            self.instance
                .destroy_instance(Some(&State::get_allocation_callbacks()));
        }

        debug!("Vulkan shutdown succeeded");
    }

    pub fn set_gpu(&mut self, gpu_idx: usize) -> usize {
        let old_idx = self.gpu;
        if gpu_idx < self.gpus.len() {
            self.gpu = gpu_idx;
            let gpu = &self.gpus[self.gpu];

            let name = unsafe {
                ffi::CStr::from_ptr(gpu.properties.device_name.as_ptr())
                    .to_str()
                    .unwrap()
            };
            debug!(
                "Selected {:#?} device {}, {} [{:04x}:{:04x}] with score {}",
                gpu.properties.device_type,
                gpu_idx,
                name,
                gpu.properties.vendor_id,
                gpu.properties.device_id,
                gpu.performance_score
            );
        }

        old_idx
    }
}

pub struct ShaderData {
    handle: vk::ShaderEXT,
}
