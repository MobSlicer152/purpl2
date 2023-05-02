mod rendersystem;

use crate::platform;
use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use std::sync::Mutex;
use std::{fs, io};

const FRAME_SMOOTHING: f64 = 0.9;

#[derive(Default)]
struct State {
    start_time: i64,
    last_time: i64,
    runtime: i64,
    fps: f64,
    delta: i64,
}

impl State {
    pub fn update(&mut self) {
        // TODO: fix. milis aren't precise enough, and this is called many times per milisecond
        // As a result, almost always, delta = 0

        let now = chrono::Local::now().timestamp_millis();
        self.delta = now - self.last_time;
        println!(
            "{} - {} = {} ({})",
            now, self.last_time, self.delta, self.fps
        );
        self.runtime += self.delta;
        self.fps = (self.fps * FRAME_SMOOTHING) + (self.delta as f64 * (1.0 - FRAME_SMOOTHING));

        self.last_time = now; // added because otherwise delta = timestamp, so runtime overflows quickly.
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
macro_rules! get_state {
    () => {
        STATE.lock().unwrap().as_mut().unwrap()
    };
}

fn setup_logger() -> Result<(), fern::InitError> {
    let dt = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::BrightCyan)
        .trace(Color::Cyan);

    let dispatch = fern::Dispatch::new()
        .format(move |out, message, record| {
            let dt = Local::now();
            out.finish(format_args!(
                "\x1B[{}m[{} {} {}] {}\x1B[0m",
                colors_line.get_color(&record.level()).to_fg_str(),
                dt.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(fern::log_file(
            DataDirs::logs() + crate::GAME_EXECUTABLE_NAME + "-" + &dt + ".log",
        )?);

    #[cfg(build = "debug")]
    let dispatch = dispatch.level(log::LevelFilter::Debug);
    #[cfg(all(not(build = "debug"), feature = "release_log"))]
    let dispatch = dispatch.level(log::LevelFilter::Info);
    #[cfg(any(build = "debug", all(not(build = "debug"), feature = "release_log")))]
    let dispatch = dispatch.chain(io::stdout());

    dispatch.apply()?;

    Ok(())
}

pub fn init() {
    for dir in DataDirs::all() {
        if fs::create_dir_all(dir.clone()).is_err() {
            panic!("Failed to create engine data directory {dir}")
        }
    }

    if setup_logger().is_err() {
        panic!("Failed to set up logger");
    }

    info!("Engine initialization started");

    platform::video::init();
    rendersystem::init();
}

pub fn update() {
    if !platform::video::focused() || platform::video::resized() {
        return;
    }

    if STATE.lock().unwrap().is_some() {
        get_state!().update();
    } else {
        *STATE.lock().unwrap() = Some(State {
            ..Default::default()
        });
        get_state!().start_time = chrono::Local::now().timestamp();
    }

    rendersystem::begin_cmds();

    rendersystem::present();
}

pub fn shutdown() {
    info!("Engine shutdown started");

    rendersystem::shutdown();
    platform::video::shutdown();

    info!("Engine shutdown succeeded");
}

use crate::GAME_NAME;
pub struct DataDirs;
impl DataDirs {
    pub fn all() -> Vec<String> {
        vec![Self::base(), Self::logs(), Self::saves()]
    }

    pub fn base() -> String {
        let basedirs = directories::BaseDirs::new().unwrap();
        let subdir_path = basedirs.data_dir().to_str().unwrap().replace('\\', "/");
        format!("{subdir_path}/{GAME_NAME}/")
    }

    pub fn logs() -> String {
        Self::base() + "logs/"
    }

    pub fn saves() -> String {
        Self::base() + "saves/"
    }
}
