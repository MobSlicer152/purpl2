include!("src/game.rs");

fn main() {
    #[cfg(windows)]
    embed_resource::compile(
        "src/platform/win32/purpl.rc",
        &[
            std::format!("GAME_NAME=\"{}\"", GAME_NAME),
            std::format!("GAME_EXECUTABLE_NAME=\"{}\"", GAME_EXECUTABLE_NAME),
            std::format!("GAME_ORGANIZATION_NAME=\"{}\"", GAME_ORGANIZATION_NAME),
            std::format!("GAME_VERSION_MAJOR_STRING=\"{}\"", GAME_VERSION_MAJOR),
            std::format!("GAME_VERSION_MINOR_STRING=\"{}\"", GAME_VERSION_MINOR),
            std::format!("GAME_VERSION_PATCH_STRING=\"{}\"", GAME_VERSION_PATCH),
            std::format!("GAME_VERSION_MAJOR={}", GAME_VERSION_MAJOR),
            std::format!("GAME_VERSION_MINOR={}", GAME_VERSION_MINOR),
            std::format!("GAME_VERSION_PATCH={}", GAME_VERSION_PATCH),
        ],
    );
}
