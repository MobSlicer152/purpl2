use image::io::Reader;
use std::{mem, path, process};

include!("../../../src/game.rs");

pub fn main() {
    let args = clap::Command::new("Purpl Texture Converter")
        .author("MobSlicer152")
        .about("Convert between Purpl textures and regular formats")
        .arg(clap::Arg::new("source")
            .help("the input image to convert")
            .action(clap::ArgAction::Set)
            .required(true))
        .arg(clap::Arg::new("destination")
            .help("the destination for the converted image")
            .action(clap::ArgAction::Set)
            .required(true))
        .arg(clap::Arg::new("format")
            .help("the format to write the destination image in, if source is a Purpl texture")
            .action(clap::ArgAction::Set)
            .default_value("png")
            .required(false))
        .try_get_matches()
        .unwrap_or_else(|err| panic!("Failed to parse arguments: {err}"));
        
    let source = path::Path::new(args.get_one::<String>("source").unwrap());
    let destination = path::Path::new(args.get_one::<String>("destination").unwrap());

    let source_extension = source.extension().unwrap().to_str().unwrap();
    let destination_extension = destination.extension().unwrap().to_str().unwrap();
    if source_extension == destination_extension {
        println!("Not converting file to same format ({source:#?}, {destination:#?})");
        process::exit(1);
    }

    println!("Converting {source:#?} -> {destination:#?}");

    match source_extension {
        "ptex" => {
            let tex = match texture::Texture::load(source) {
                Ok(tex) => tex,
                Err(err) => {
                    println!("Failed to load texture {source:#?}: {err:#?}");
                    process::exit(1);
                }
            };
            let image = match tex.format() {
                texture::Pixel::Rgb(_) => image::DynamicImage::new_rgb8(tex.data()),
                texture::Pixel::Rgba(_) => image::DynamicImage::new_rgba8(tex.data()),
                format => {
                    println!("No equivalent format for {format:#?}");
                    process::exit(1);
                }
            };

            match image.save(destination) {
                Ok(_) => {},
                Err(err) => {
                    println!("Failed to save texutre {source:#?} as {destination:#?}: {err}");
                    process::exit(1);
                }
            }
        },
        _ => {
            let image = match Reader::open(source) {
                Ok(reader) => match reader.decode() {
                    Ok(image) => image,
                    Err(err) => {
                        println!("Failed to decode {source:#?}: {err}");
                        process::exit(1)
                    }
                },
                Err(err) => {
                    println!("Failed to read {source:#?}: {err}");
                    process::exit(1)
                }
            };
            let format = match image {
                image::DynamicImage::ImageRgb8(_) => texture::pixel_format::RGB,
                image::DynamicImage::ImageRgba8(_) => texture::pixel_format::RGBA,
                format => {
                    println!("No equivalent format for {format:#?}");
                    process::exit(1);
                }
            };
            let tex = texture::Texture::new(format, image.width(), image.height(), unsafe { mem::transmute::<Vec<texture::Pixel>>(image.into_bytes()) });
        }
    };
}