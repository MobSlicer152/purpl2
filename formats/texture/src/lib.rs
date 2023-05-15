use serde::{Deserialize, Serialize};
use std::{cmp, fs, io, path};

const TEXTURE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct RgbPixel {
    r: u8,
    g: u8,
    b: u8,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct RgbaPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Pixel {
    Rgb(RgbPixel),
    Rgba(RgbaPixel),
}

pub mod pixel_format {
    use crate::{Pixel, RgbPixel, RgbaPixel};
    
    pub const RGB: Pixel = Pixel::Rgb(RgbPixel { r: 0, g: 0, b: 0 });
    pub const RGBA: Pixel = Pixel::Rgba(RgbaPixel {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    });
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

#[derive(Debug)]
pub enum TextureError {
    Io(io::Error),
    Postcard(postcard::Error)
}

#[derive(Serialize, Deserialize)]
pub struct Texture {
    version: u32,
    format: Pixel,
    width: u32,
    height: u32,
    data: Vec<u8>,
}

pub struct TextureReader {
    texture: Texture,
    offset: usize
}

impl TextureReader {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            offset: 0
        }
    }
}

impl io::Read for TextureReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.offset += cmp::max(self.texture.data().len(), buf.len());
        self.texture.data().as_slice().read(buf)
    }
}

impl Texture {
    pub fn new(format: Pixel, width: u32, height: u32, pixels: Vec<Pixel>) -> Result<Self, TextureError> {
        let mut raw_pixels = Vec::new();
        for pixel in pixels {
            let mut pixel = match zstd::bulk::compress(
                &mut postcard::to_allocvec_cobs(&pixel).unwrap(),
                zstd::DEFAULT_COMPRESSION_LEVEL,
            ) {
                Ok(pixel) => pixel,
                Err(err) => { return Err(TextureError::Io(err)); }
            };
            raw_pixels.append(&mut pixel);
        }

        Ok(Self {
            version: TEXTURE_VERSION,
            format,
            width,
            height,
            data: raw_pixels,
        })
    }

    pub fn from_bytes(v: Vec<u8>) -> Result<Self, TextureError> {
        let mut decompressed = match zstd::bulk::decompress(&v, usize::MAX) {
            Ok(decompressed) => decompressed,
            Err(err) => { return Err(TextureError::Io(err)) }
        };
        match postcard::from_bytes_cobs(decompressed.as_mut_slice()) {
            Ok(texture) => Ok(texture),
            Err(err) => { return Err(TextureError::Postcard(err)) }
        }
    }

    pub fn load<Q>(path: Q) -> Result<Self, TextureError>
        where Q: AsRef<path::Path> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => { return Err(TextureError::Io(err)); }
        };

        Self::from_bytes(bytes)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn format(&self) -> &Pixel {
        &self.format
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}


