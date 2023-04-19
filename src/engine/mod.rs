mod rendersystem;

use crate::platform;
use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use std::{fs, io};

fn setup_logger() -> Result<(), fern::InitError> {
    let dt = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::BrightCyan)
        .trace(Color::Cyan);

    fern::Dispatch::new()
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
        .level(log::LevelFilter::Debug)
        .chain(io::stdout())
        .chain(fern::log_file(
            DataDirs::logs() + crate::GAME_EXECUTABLE_NAME + "-" + &dt + ".log",
        )?)
        .apply()?;
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
