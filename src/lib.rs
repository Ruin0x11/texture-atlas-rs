#[macro_use] extern crate serde_derive;
extern crate bincode;
extern crate crypto;
extern crate glium;
extern crate glob;
extern crate image;
extern crate serde;
extern crate texture_packer;
extern crate toml;

mod texture_atlas;
mod tile_atlas;
mod tile_atlas_config;
mod toml_util;

pub use texture_atlas::{TextureAtlasBuilder, TextureAtlas};
pub use tile_atlas::{TileAtlasBuilder, TileAtlas};
use image::GenericImage;

type Texture2d = glium::texture::CompressedSrgbTexture2d;

#[derive(Serialize, Deserialize, Clone)]
pub struct AtlasRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl From<texture_packer::Rect> for AtlasRect {
    fn from(rect: texture_packer::Rect) -> AtlasRect {
        AtlasRect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }
}

fn make_texture<F: glium::backend::Facade>(display: &F, image: image::DynamicImage) -> Texture2d {
    let dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.to_rgba().into_raw(), dimensions);
    Texture2d::new(display, image).unwrap()
}
