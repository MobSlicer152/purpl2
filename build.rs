use std::{env, fs, io};

include!("src/game.rs");

fn main() {
    let profile = env::var("PROFILE").unwrap();

    println!("cargo:rustc-cfg=build={:?}", profile);

    #[cfg(windows)]
    embed_resource::compile(
        "src/platform/win32/purpl.rc",
        [
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

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "xbox")))]
    {
        const SHADER_DIR: &'static str = "src/engine/rendersystem/shaders/vulkan/";
        let output_dir = format!("target/{profile}/{GAME_EXECUTABLE_NAME}/shaders/");

        fs::create_dir_all(&output_dir).unwrap();

        let compiler = shaderc::Compiler::new().unwrap();

        let mut entries = fs::read_dir(SHADER_DIR)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();
        entries.sort();

        entries.iter().for_each(|entry| {
            let binary_path = entry
                .to_str()
                .unwrap()
                .replace(SHADER_DIR, output_dir.as_str())
                + ".spv";
            let binary_meta = fs::metadata(&binary_path);
            // only compile if source has probably changed
            if binary_meta.is_err()
                || (binary_meta.is_ok()
                    && fs::metadata(&entry).unwrap().modified().unwrap()
                        > binary_meta.unwrap().modified().unwrap())
            {
                let content = String::from_utf8(fs::read(&entry).unwrap()).unwrap();
                let extension = entry.as_path().extension().unwrap().to_str().unwrap();
                let binary = compiler
                    .compile_into_spirv(
                        content.as_str(),
                        match extension {
                            "vert" => shaderc::ShaderKind::Vertex,
                            "frag" => shaderc::ShaderKind::Fragment,
                            "comp" => shaderc::ShaderKind::Compute,
                            _ => panic!("Unknown Vulkan shader file extension {extension}"),
                        },
                        entry.as_os_str().to_str().unwrap(),
                        "main",
                        None,
                    )
                    .unwrap();

                fs::write(&binary_path, binary.as_binary_u8()).unwrap();
            }
        });
    }
}
