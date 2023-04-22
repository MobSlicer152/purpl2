use log::{debug, info};
use std::mem;
use xcb::x;

pub struct State {
    connection: xcb::Connection,
    window: x::Window,
    title: String,
    width: u32,
    height: u32,
    resized: bool,
    focused: bool,
    closed: bool,
}

fn get_xcb_atom(connection: &xcb::Connection, name: &str) -> x::Atom {
    let reply = connection.send_request(&x::InternAtom {
        only_if_exists: true,
        name: name.as_bytes(),
    });
    connection.wait_for_reply(reply).unwrap().atom()
}

impl State {
    pub fn init() -> Self {
        info!("XCB video initialization started");

        let (connection, screen_num) = xcb::Connection::connect(None).unwrap();
        let screen = connection
            .get_setup()
            .roots()
            .nth(screen_num as usize)
            .unwrap();
        let height = (screen.height_in_pixels() as f32 / 1.5) as u32;
        let width = height * 16 / 9;
        let window = connection.generate_id();
        let title = format!(
            "{} v{}.{}.{} by {}",
            crate::GAME_NAME,
            crate::GAME_VERSION_MAJOR,
            crate::GAME_VERSION_MINOR,
            crate::GAME_VERSION_PATCH,
            crate::GAME_ORGANIZATION_NAME
        );

        let cookie = connection.send_request_checked(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: window,
            parent: screen.root(),
            x: 0,
            y: 0,
            width: width as u16,
            height: height as u16,
            border_width: 0,
            class: x::WindowClass::InputOutput,
            visual: screen.root_visual(),
            value_list: &[
                x::Cw::BackPixel(screen.black_pixel()),
                x::Cw::EventMask(x::EventMask::FOCUS_CHANGE | x::EventMask::STRUCTURE_NOTIFY),
            ],
        });
        if connection.check_request(cookie).is_err() {
            panic!("Failed to create window");
        }

        let cookie = connection.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: x::ATOM_WM_NAME,
            r#type: x::ATOM_STRING,
            data: title.as_bytes(),
        });
        if connection.check_request(cookie).is_err() {
            panic!("Failed to set window title to {title}");
        }

        let protocols = get_xcb_atom(&connection, "WM_PROTOCOLS");
        let delete_data = get_xcb_atom(&connection, "WM_DELETE_WINDOW");

        connection.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: protocols,
            r#type: x::ATOM_ATOM,
            data: &[delete_data],
        });

        connection.send_request(&x::MapWindow { window });

        if connection.flush().is_err() {
            panic!("Failed to flush XCB connection");
        }

        info!("XCB video initialization succeeded");

        State {
            connection,
            window,
            title,
            width,
            height,
            resized: false,
            focused: false,
            closed: false,
        }
    }

    pub fn update(&mut self) -> bool {
        if let Ok(Some(xcb::Event::X(event))) = self.connection.poll_for_event() {
            match event {
                x::Event::ConfigureNotify(ev) => {
                    let new_width = ev.width() as u32;
                    let new_height = ev.height() as u32;

                    if new_width != self.width || new_height != self.height {
                        self.resized = true;
                        info!(
                            "Window resized from {}x{} to {}x{}",
                            self.width, self.height, new_width, new_height
                        );
                        self.width = new_width;
                        self.height = new_height;
                    }
                }
                x::Event::FocusIn(_) => {
                    info!("Window focused");
                    self.focused = true;
                }
                x::Event::FocusOut(_) => {
                    info!("Window unfocused");
                    self.focused = false;
                }
                x::Event::ClientMessage(ev) => {
                    if let x::ClientMessageData::Data32(atom) = ev.data() {
                        let delete_atom = get_xcb_atom(&self.connection, "WM_DELETE_WINDOW");
                        let atom = unsafe { mem::transmute::<u32, x::Atom>(atom[0]) };
                        self.closed = atom == delete_atom;
                    }
                }
                _ => {}
            }
        }

        !self.closed
    }

    pub fn shutdown(&mut self) {
        info!("XCB video shutdown started");

        debug!("Destroying window");
        self.connection.send_request(&x::DestroyWindow {
            window: self.window,
        });

        info!("XCB video shutdown succeeded");
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn resized(&mut self) -> bool {
        let ret = self.resized;
        self.resized = false;
        ret
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn create_vulkan_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        alloc_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> vk::SurfaceKHR {
        unsafe {
            extensions::khr::XcbSurface::new(&entry, &instance)
                .create_win32_surface(
                    &vk::XcbSurfaceCreateInfoKHR {
                        connection: self.connection,
                        window: self.window,
                        ..Default::default()
                    },
                    alloc_callbacks,
                )
                .unwrap_or_else(|err| panic!("Failed to create XCB surface: {}", err))
        }
    }
}
