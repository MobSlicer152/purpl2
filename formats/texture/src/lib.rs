use serde::{Deserialize, Serialize};

const TEXTURE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct DepthPixel {
    depth: f32,
}
#[derive(Serialize, Deserialize)]
pub struct RgbPixel {
    r: u8,
    g: u8,
    b: u8,
}
#[derive(Serialize, Deserialize)]
pub struct RgbaPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Serialize, Deserialize)]
pub enum Pixel {
    Depth(DepthPixel),
    Rgb(RgbPixel),
    Rgba(RgbaPixel),
}

pub mod pixel_format {
    use crate::{DepthPixel, Pixel, RgbPixel, RgbaPixel};

    const DEPTH: Pixel = Pixel::Depth(DepthPixel { depth: 0.0 });
    const RGB: Pixel = Pixel::Rgb(RgbPixel { r: 0, g: 0, b: 0 });
    const RGBA: Pixel = Pixel::Rgba(RgbaPixel {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    });
}

impl From<DepthPixel> for Pixel {
    fn from(p: DepthPixel) -> Self {
        Self::Depth(p)
    }
}

impl From<RgbPixel> for Pixel {
    fn from(p: RgbPixel) -> Self {
        Self::Rgb(p)
    }
}

impl From<RgbaPixel> for Pixel {
    fn from(p: RgbaPixel) -> Self {
        Self::Rgba(p)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Texture {
    version: u32,
    format: Pixel,
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Texture {
    pub fn new(format: Pixel, width: u32, height: u32, pixels: Vec<Pixel>) -> Self {
        let mut raw_pixels = Vec::new();
        for pixel in pixels {
            let mut tmp = zstd::bulk::compress(
                &mut postcard::to_allocvec_cobs(&pixel).unwrap(),
                zstd::DEFAULT_COMPRESSION_LEVEL,
            )
            .unwrap();
            raw_pixels.append(&mut tmp);
        }

        Self {
            version: TEXTURE_VERSION,
            format,
            width,
            height,
            data: raw_pixels,
        }
    }
}

impl From<Vec<u8>> for Texture {
    fn from(v: Vec<u8>) -> Self {
        let mut decompressed = zstd::bulk::decompress(&v, usize::MAX).unwrap();
        postcard::from_bytes_cobs(&mut decompressed).unwrap()
    }
}
