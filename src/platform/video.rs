#[cfg(any(linux, freebsd, dragonfly, openbsd, netbsd))]
mod video_impl {
    use crate::platform::unix::video;
    use std::sync::{Arc, Mutex};

    static STATE: Mutex<Option<video::State>> = Mutex::new(None);
    pub fn init() {
        *STATE.lock().unwrap() = Some(video::State::init())
    }
    pub fn update() -> bool {
        STATE.lock().unwrap().as_mut().unwrap().update()
    }
    pub fn shutdown() {
        STATE.lock().unwrap().as_mut().unwrap().shutdown()
    }
    pub fn get_size() -> (u32, u32) {
        STATE.lock().unwrap().as_ref().unwrap().get_size()
    }
    pub fn resized() -> bool {
        STATE.lock().unwrap().as_mut().unwrap().resized()
    }
    pub fn focused() -> bool {
        STATE.lock().unwrap().as_ref().unwrap().focused()
    }
    pub unsafe fn create_vulkan_surface(entry: ash::Entry, instance: ash::Instance, alloc_callbacks: vk::AllocationCallbacks) -> extensions::khr::SurfaceKHR {
        STATE.lock().unwrap().as_ref().unwrap().create_vulkan_surface(entry, instance, alloc_callbacks)
    }
}
#[cfg(any(windows, xbox))]
mod video_impl {
    use std::sync::Arc;

    use crate::platform::win32::video;
    pub fn init() {
        unsafe { video::init() }
    }
    pub fn update() -> bool {
        unsafe { video::update() }
    }
    pub fn shutdown() {
        unsafe { video::shutdown() }
    }
    pub fn get_size() -> (u32, u32) {
        unsafe { video::get_size() }
    }
    pub fn resized() -> bool {
        unsafe { video::resized() }
    }
    pub fn focused() -> bool {
        unsafe { video::focused() }
    }
    #[cfg(all(windows, not(xbox)))]
    pub fn create_vulkan_surface(entry: ash::Entry, instance: ash::Instance, alloc_callbacks: ash::vk::AllocationCallbacks) -> ash::vk::SurfaceKHR {
        video::create_vulkan_surface(entry, instance, alloc_callbacks)
    }
}

pub use video_impl::*;
