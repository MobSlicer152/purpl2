## Purpl Engine

This is a game engine I'm making.

### Build instructions

Use Cargo.

### Build requirements

[Install Rust and its dependencies.](https://www.rust-lang.org/tools/install)

### Interesting stuff I guess

This is a Rust rewrite of [my C engine](https://github.com/MobSlicer152/purpl-engine).

The engine is/will be made of these components:

- `platform` - Platform abstraction, handles the compiler(s) used for that platform but also OS functions,
also handles "video" (another Quake 2 idea sort of), through functions that hide most details
about the underlying window and such.
- `engine` - Currently contains a camera structure and function, a transform structure, and some ECS stuff.
- `rendersystem` (`engine/rendersystem`) - API-independant frontend for rendering, also inspired a bit by Quake 2.
- `rendersystem-vk` (`engine/rendersystem/vulkan`) - Vulkan render backend, probably most of the code.
- `rendersystem-dx` (`engine/rendersystem/directx`) - DirectX 12 backend, planned but currently empty.
- `texture` (`util/texture`) - Texture format library. No compression, basically a header and pixels in RGB, RGBA, or depth 
(32-bit float) formats.
- `texturetool` (`util/texture`) - Converts to and from the texture format. Similar tools will exist for other formats.
- `model` (`util/model`) - Model format library. Extremely primitive, like the texture format.

I suck at optimization, but I'm also trying to avoid worrying about it until it's an issue.

I heard that Doom Eternal uses thread jobs for everything so I hope to figure out how to do something like that.

### Things that need to be changed eventually

- Make video backends more idiomatic by using nested functions or whatever the Rust way is

### Dependencies

See [Cargo.toml](Cargo.toml)

