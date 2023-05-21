pub mod video;

use windows_sys::Win32::System::Diagnostics::Debug::*;

pub unsafe fn init() {}

pub unsafe fn shutdown() {}

pub unsafe fn have_debugger() -> bool {
    IsDebuggerPresent() != 0
}
