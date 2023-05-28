pub mod rendersystem;

use crate::platform;
use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use log::{debug, info};
use std::{fs, io};

const FRAME_SMOOTHING: f64 = 0.9;

pub struct State {
    game_dir: String,
    start_time: i64,
    last_time: i64,
    runtime: i64,
    fps: f64,
    delta: i64,

    video: platform::video::State,
    render: rendersystem::State,
}

impl State {
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
        #[cfg(feature = "verbose_log")]
        let dispatch = dispatch.level(log::LevelFilter::Trace);
        #[cfg(any(build = "debug", all(not(build = "debug"), feature = "release_log")))]
        let dispatch = dispatch.chain(io::stdout());

        dispatch.apply()?;

        Ok(())
    }

    pub fn init(args: crate::Args) -> Self {
        debug!("{args:#?}");

        if args.wait_for_debugger {
            while !platform::have_debugger() {}
        }

        for dir in DataDirs::all() {
            if fs::create_dir_all(dir.clone()).is_err() {
                panic!("Failed to create engine data directory {dir}")
            }
        }

        if Self::setup_logger().is_err() {
            panic!("Failed to set up logger");
        }

        info!("Engine initialization started");

        let video = platform::video::State::init();
        let render = rendersystem::State::init(&video);

        Self {
            game_dir: args.game,
            start_time: 0,
            last_time: 0,
            runtime: 0,
            fps: 0.0,
            delta: 0,
            video,
            render,
        }
    }

    pub fn update(&mut self) {
        if !self.video.focused() || self.video.resized() {
            return;
        }

        if self.last_time == 0 {
            let now = chrono::Local::now().timestamp_millis();
            self.delta = now - self.last_time;
            self.runtime += self.delta;
            self.fps = if self.delta > 0 {
                (self.fps * FRAME_SMOOTHING)
                    + ((1000.0 / self.delta as f64) * (1.0 - FRAME_SMOOTHING))
            } else {
                f64::INFINITY
            };

            self.last_time = now;
        } else {
            self.start_time = chrono::Local::now().timestamp();
        }

        self.render.begin_commands(&self.video);

        self.render.present();
    }

    pub fn shutdown(mut self) {
        info!("Engine shutdown started");

        self.render.shutdown();
        self.video.shutdown();

        info!("Engine shutdown succeeded");
    }

    pub fn video(&mut self) -> &mut platform::video::State {
        &mut self.video
    }

    pub fn render(&mut self) -> &mut rendersystem::State {
        &mut self.render
    }
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

pub struct GameDirs;
impl GameDirs {
    pub fn all(state: &State) -> Vec<String> {
        vec![Self::base(state), Self::models(state), Self::shaders(state)]
    }

    pub fn base(state: &State) -> String {
        format!("{}/", state.game_dir)
    }

    pub fn models(state: &State) -> String {
        Self::base(state) + "models/"
    }

    pub fn shaders(state: &State) -> String {
        Self::base(state) + "shaders/"
    }
}
