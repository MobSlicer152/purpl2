#[cfg(not(any(macos, ios, xbox)))]
use ash::vk;

pub trait VideoBackend {
    fn init() -> Box<dyn VideoBackend>
    where
        Self: Sized;
    fn update(&mut self) -> bool;
    fn shutdown(&mut self);

    fn get_size(&self) -> (u32, u32);
    fn focused(&self) -> bool;
    fn resized(&mut self) -> bool;

    #[cfg(any(windows, xbox))]
    fn get_handle(&self) -> usize;

    #[cfg(not(any(macos, ios, xbox)))]
    fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        alloc_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> vk::SurfaceKHR;
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
pub use crate::platform::unix::video::*;
#[cfg(any(windows, xbox))]
pub use crate::platform::win32::video::*;
