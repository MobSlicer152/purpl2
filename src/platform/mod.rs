pub mod video;

#[cfg(unix)]
mod unix;
#[cfg(any(windows, xbox))]
mod win32;

mod platform_impl {
    #[cfg(unix)]
    pub use crate::platform::unix::*;
    #[cfg(any(windows, xbox))]
    pub use crate::platform::win32::*;
}

pub fn init() {
    unsafe { platform_impl::init() }
}

pub fn shutdown() {
    unsafe { platform_impl::shutdown() }
}

pub fn have_debugger() -> bool {
    unsafe { platform_impl::have_debugger() }
}
