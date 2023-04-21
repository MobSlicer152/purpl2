use ash::{extensions, vk};
use log::{debug, info};
use std::mem;
use std::os;
use std::ptr;
use std::sync::Arc;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const IDI_ICON1: u32 = 103;

static mut WND: HWND = 0;

const WND_CLASS_NAME: &str = "PurplWindow";

static mut WND_TITLE: String = String::new();
static mut WND_WIDTH: u32 = 0;
static mut WND_HEIGHT: u32 = 0;

static mut WND_RESIZED: bool = false;
static mut WND_FOCUSED: bool = false;
static mut WND_CLOSED: bool = false;

unsafe extern "system" fn wndproc(
    msg_wnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if WND == 0 || msg_wnd == WND {
        match msg {
            WM_SIZE => {
                let mut client_area: RECT = mem::zeroed();

                GetClientRect(msg_wnd, ptr::addr_of_mut!(client_area));
                let new_width = (client_area.right - client_area.left) as u32;
                let new_height = (client_area.bottom - client_area.top) as u32;

                if new_width != WND_WIDTH || new_height != WND_HEIGHT {
                    WND_RESIZED = true;
                    info!(
                        "Window resized from {}x{} to {}x{}",
                        WND_WIDTH, WND_HEIGHT, new_width, new_height
                    );
                }

                WND_WIDTH = new_width;
                WND_HEIGHT = new_height;
                0
            }
            WM_ACTIVATEAPP => {
                WND_FOCUSED = wparam != 0;
                info!(
                    "Window {}",
                    if WND_FOCUSED { "focused" } else { "unfocused" }
                );
                0
            }
            WM_CLOSE => {
                info!("Window closed");
                WND_CLOSED = true;
                0
            }
            _ => DefWindowProcA(msg_wnd, msg, wparam, lparam),
        }
    } else {
        DefWindowProcA(msg_wnd, msg, wparam, lparam)
    }
}

unsafe fn register_wndclass() {
    let mut wnd_class: WNDCLASSEXA = mem::zeroed();
    let base_addr = GetModuleHandleA(ptr::null_mut());

    debug!("Registering window class");

    wnd_class.cbSize = mem::size_of::<WNDCLASSEXA>() as u32;
    wnd_class.lpfnWndProc = Some(wndproc);
    wnd_class.hInstance = base_addr;
    wnd_class.hCursor = LoadCursorA(0, IDC_ARROW as *const u8);
    wnd_class.hIcon = LoadIconA(base_addr, IDI_ICON1 as *const u8);
    wnd_class.lpszClassName = WND_CLASS_NAME.as_ptr();
    if RegisterClassExA(ptr::addr_of_mut!(wnd_class)) == 0 {
        let err = GetLastError();
        panic!(
            "Failed to register window class: error 0x{:X} ({})",
            err, err
        );
    }

    debug!("Window class registered");
}

unsafe fn init_wnd() {
    let mut client_area: RECT = mem::zeroed();
    let base_addr = GetModuleHandleA(ptr::null_mut());

    client_area.left = 0;
    client_area.right = (GetSystemMetrics(SM_CXSCREEN) as f32 / 1.5) as i32;
    client_area.top = 0;
    client_area.bottom = (GetSystemMetrics(SM_CYSCREEN) as f32 / 1.5) as i32;
    AdjustWindowRect(
        ptr::addr_of_mut!(client_area),
        WS_OVERLAPPEDWINDOW,
        false as i32,
    );
    WND_WIDTH = (client_area.right - client_area.left) as u32;
    WND_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WND_TITLE = format!(
        "{} v{}.{}.{} by {}",
        crate::GAME_NAME,
        crate::GAME_VERSION_MAJOR,
        crate::GAME_VERSION_MINOR,
        crate::GAME_VERSION_PATCH,
        crate::GAME_ORGANIZATION_NAME
    );
    debug!(
        "Creating {}x{} window titled {}",
        WND_WIDTH, WND_HEIGHT, WND_TITLE
    );

    WND = CreateWindowExA(
        0,
        WND_CLASS_NAME.as_ptr(),
        WND_TITLE.as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        WND_WIDTH as i32,
        WND_HEIGHT as i32,
        0,
        0,
        base_addr,
        ptr::null_mut()
    );
    if WND == 0 {
        let err = GetLastError();
        panic!("Failed to create window: error 0x{:X} {}", err, err);
    }

    GetClientRect(WND, ptr::addr_of_mut!(client_area));
    WND_WIDTH = (client_area.right - client_area.left) as u32;
    WND_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WND_RESIZED = false;
    WND_FOCUSED = true;
    WND_CLOSED = false;

    debug!(
        "Successfully created window with handle 0x{:X}",
        WND as usize
    );
}

pub unsafe fn init() {
    info!("Windows video initialization started");

    register_wndclass();
    init_wnd();

    debug!("Showing window");
    ShowWindow(WND, SW_SHOW);

    info!("Windows video initialization succeeded");
}

pub unsafe fn update() -> bool {
    let mut msg: MSG = mem::zeroed();

    while PeekMessageA(
        ptr::addr_of_mut!(msg),
        0,
        0,
        0,
        PM_REMOVE,
    ) != 0
    {
        TranslateMessage(ptr::addr_of_mut!(msg));
        DispatchMessageA(ptr::addr_of_mut!(msg));
    }

    !WND_CLOSED
}

pub unsafe fn shutdown() {
    info!("Windows video shutdown started");

    debug!("Destroying window");
    DestroyWindow(WND);

    info!("Windows video shutdown succeeded");
}

pub unsafe fn get_size() -> (u32, u32) {
    (WND_WIDTH, WND_HEIGHT)
}

pub unsafe fn resized() -> bool {
    let ret = WND_RESIZED;
    WND_RESIZED = false;
    ret
}

pub unsafe fn focused() -> bool {
    WND_FOCUSED
}

#[cfg(not(xbox))]
pub fn create_vulkan_surface(entry: ash::Entry, instance: ash::Instance, alloc_callbacks: vk::AllocationCallbacks) -> vk::SurfaceKHR {
    unsafe {
        extensions::khr::Win32Surface::new(&entry, &instance)
            .create_win32_surface(&vk::Win32SurfaceCreateInfoKHR {
                hinstance: GetModuleHandleA(ptr::null_mut()) as *const os::raw::c_void,
                hwnd: WND as *const os::raw::c_void,
                ..Default::default()
            }, Some(&alloc_callbacks))
            .unwrap_or_else(|err| panic!("Failed to create HWND surface: {}", err))
    }
}
