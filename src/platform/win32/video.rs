use ash::{extensions, vk};
use log::{debug, info};
use std::{ffi, mem, ptr};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const IDI_ICON1: u32 = 103;

static mut WINDOW: HWND = 0;

const WINDOW_CLASS_NAME: &str = "PurplWindow";

static mut WINDOW_TITLE: String = String::new();
static mut WINDOW_WIDTH: u32 = 0;
static mut WINDOW_HEIGHT: u32 = 0;

static mut WINDOW_RESIZED: bool = false;
static mut WINDOW_FOCUSED: bool = false;
static mut WINDOW_CLOSED: bool = false;

unsafe extern "system" fn wndproc(message_window: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    if WINDOW == 0 || message_window == WINDOW {
        match message {
            WM_SIZE => {
                let mut client_area: RECT = mem::zeroed();

                GetClientRect(message_window, ptr::addr_of_mut!(client_area));
                let new_width = (client_area.right - client_area.left) as u32;
                let new_height = (client_area.bottom - client_area.top) as u32;

                if new_width != WINDOW_WIDTH || new_height != WINDOW_HEIGHT {
                    WINDOW_RESIZED = true;
                    info!(
                        "Window resized from {}x{} to {}x{}",
                        WINDOW_WIDTH, WINDOW_HEIGHT, new_width, new_height
                    );
                }

                WINDOW_WIDTH = new_width;
                WINDOW_HEIGHT = new_height;
                0
            }
            WM_ACTIVATEAPP => {
                WINDOW_FOCUSED = wparam != 0;
                info!(
                    "Window {}",
                    if WINDOW_FOCUSED { "focused" } else { "unfocused" }
                );
                0
            }
            WM_CLOSE => {
                info!("Window closed");
                WINDOW_CLOSED = true;
                0
            }
            _ => DefWindowProcA(message_window, message, wparam, lparam),
        }
    } else {
        DefWindowProcA(message_window, message, wparam, lparam)
    }
}

unsafe fn register_wndclass() {
    let mut window_class: WNDCLASSEXA = mem::zeroed();
    let base_addr = GetModuleHandleA(ptr::null_mut());

    debug!("Registering window class");

    window_class.cbSize = mem::size_of::<WNDCLASSEXA>() as u32;
    window_class.lpfnWndProc = Some(wndproc);
    window_class.hInstance = base_addr;
    window_class.hCursor = LoadCursorA(0, IDC_ARROW as *const u8);
    window_class.hIcon = LoadIconA(base_addr, IDI_ICON1 as *const u8);
    window_class.lpszClassName = WINDOW_CLASS_NAME.as_ptr();
    if RegisterClassExA(ptr::addr_of_mut!(window_class)) == 0 {
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
    WINDOW_WIDTH = (client_area.right - client_area.left) as u32;
    WINDOW_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WINDOW_TITLE = format!(
        "{} v{}.{}.{} by {}",
        crate::GAME_NAME,
        crate::GAME_VERSION_MAJOR,
        crate::GAME_VERSION_MINOR,
        crate::GAME_VERSION_PATCH,
        crate::GAME_ORGANIZATION_NAME
    );
    debug!(
        "Creating {}x{} window titled {}",
        WINDOW_WIDTH, WINDOW_HEIGHT, WINDOW_TITLE
    );

    WINDOW = CreateWindowExA(
        0,
        WINDOW_CLASS_NAME.as_ptr(),
        WINDOW_TITLE.as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        WINDOW_WIDTH as i32,
        WINDOW_HEIGHT as i32,
        0,
        0,
        base_addr,
        ptr::null_mut(),
    );
    if WINDOW == 0 {
        let err = GetLastError();
        panic!("Failed to create window: error 0x{:X} {}", err, err);
    }

    GetClientRect(WINDOW, ptr::addr_of_mut!(client_area));
    WINDOW_WIDTH = (client_area.right - client_area.left) as u32;
    WINDOW_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WINDOW_RESIZED = false;
    WINDOW_FOCUSED = true;
    WINDOW_CLOSED = false;

    debug!(
        "Successfully created window with handle 0x{:X}",
        WINDOW as usize
    );
}

pub unsafe fn init() {
    info!("Windows video initialization started");

    register_wndclass();
    init_wnd();

    debug!("Showing window");
    ShowWindow(WINDOW, SW_SHOW);

    info!("Windows video initialization succeeded");
}

pub unsafe fn update() -> bool {
    let mut msg: MSG = mem::zeroed();

    while PeekMessageA(ptr::addr_of_mut!(msg), 0, 0, 0, PM_REMOVE) != 0 {
        TranslateMessage(ptr::addr_of_mut!(msg));
        DispatchMessageA(ptr::addr_of_mut!(msg));
    }

    !WINDOW_CLOSED
}

pub unsafe fn shutdown() {
    info!("Windows video shutdown started");

    debug!("Destroying window");
    DestroyWindow(WINDOW);

    info!("Windows video shutdown succeeded");
}

pub unsafe fn get_size() -> (u32, u32) {
    (WINDOW_WIDTH, WINDOW_HEIGHT)
}

pub unsafe fn resized() -> bool {
    let ret = WINDOW_RESIZED;
    WINDOW_RESIZED = false;
    ret
}

pub unsafe fn focused() -> bool {
    WINDOW_FOCUSED
}

#[cfg(not(xbox))]
pub fn create_vulkan_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    alloc_callbacks: Option<&vk::AllocationCallbacks>,
) -> vk::SurfaceKHR {
    unsafe {
        extensions::khr::Win32Surface::new(&entry, &instance)
            .create_win32_surface(
                &vk::Win32SurfaceCreateInfoKHR {
                    hinstance: GetModuleHandleA(ptr::null_mut()) as *const ffi::c_void,
                    hwnd: WINDOW as *const ffi::c_void,
                    ..Default::default()
                },
                alloc_callbacks,
            )
            .unwrap_or_else(|err| panic!("Failed to create HWND surface: {}", err))
    }
}
