use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use glium::backend::Facade;
use image::{self, DynamicImage, Rgba};
use texture_packer::{SkylinePacker, Rect};
use texture_packer::{TexturePacker, TexturePackerConfig};
use texture_packer::importer::ImageImporter;
use texture_packer::exporter::ImageExporter;

use {AtlasRect, Texture2d, make_texture};
use tile_atlas_config::TileAtlasConfig;

pub type TileOffset = (u32, u32);
pub type TileIndex = usize;

type AnimFrames = u64;
type AnimMillisDelay = u64;
#[derive(Serialize, Deserialize, Clone)]
pub enum TileKind {
    Static,
    Animated(AnimFrames, AnimMillisDelay),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AtlasFrame {
    tile_size: (u32, u32),
    texture_idx: usize,
    rect: AtlasRect,
    offsets: HashMap<TileIndex, TileOffset>,
}

impl AtlasFrame {
    pub fn new(texture_idx: usize, rect: Rect, tile_size: (u32, u32)) -> Self {
        AtlasFrame {
            tile_size: tile_size,
            texture_idx: texture_idx,
            rect: AtlasRect::from(rect),
            offsets: HashMap::new(),
        }
    }
}

pub type TilePacker<'a> = TexturePacker<'a, DynamicImage, SkylinePacker<Rgba<u8>>>;

pub struct TileAtlas {
    locations: HashMap<TileIndex, String>,
    frames: HashMap<String, AtlasFrame>,
    textures: Vec<Texture2d>,
}

pub struct TileAtlasBuilder<'a> {
    locations: HashMap<TileIndex, String>,
    frames: HashMap<String, AtlasFrame>,
    packers: Vec<TilePacker<'a>>,
}

impl <'a> TileAtlasBuilder<'a> {
    pub fn new() -> Self {
        let mut builder = TileAtlasBuilder {
            locations: HashMap::new(),
            frames: HashMap::new(),
            packers: Vec::new(),
        };
        builder.add_packer();
        builder
    }

    pub fn add_tile(&mut self, path_str: &str, index: TileIndex, offset: TileOffset) {
        let key = path_str.to_string();
        assert!(self.frames.contains_key(&path_str.to_string()));

        {
            let mut frame = self.frames.get_mut(&key).unwrap();
            assert!(!frame.offsets.contains_key(&index));
            frame.offsets.insert(index, offset);
            self.locations.insert(index, key);
        }
    }

    pub fn add_frame(&mut self, path_string: &str, tile_size: (u32, u32)) {
        if self.frames.contains_key(path_string) {
            return;
        }

        let path = Path::new(&path_string);
        let texture = ImageImporter::import_from_file(&path).unwrap();

        for (idx, packer) in self.packers.iter_mut().enumerate() {
            if packer.can_pack(&texture) {
                packer.pack_own(path_string.to_string(), texture).unwrap();
                let rect = packer.get_frame(&path_string).unwrap().frame.clone();
                self.frames.insert(path_string.to_string(), AtlasFrame::new(idx, rect, tile_size));
                // cannot return self here, since self already borrowed, so
                // cannot use builder pattern.
                return;
            }
        }

        self.add_packer();

        {
            // complains that borrow doesn't last long enough
            // len mut packer = self.newest_packer_mut();

            let packer_idx = self.packers.len() - 1;
            let mut packer = self.packers.get_mut(packer_idx).unwrap();
            packer.pack_own(path_string.to_string(), texture).unwrap();
            let rect = packer.get_frame(&path_string).unwrap().frame.clone();
            self.frames.insert(path_string.to_string(), AtlasFrame::new(packer_idx, rect, tile_size));
        }
    }

    fn add_packer(&mut self) {
        let config = TexturePackerConfig {
            max_width: 2048,
            max_height: 2048,
            allow_rotation: false,
            texture_outlines: false,
            trim: false,
            texture_padding: 0,
            ..Default::default()
        };

        self.packers.push(TexturePacker::new_skyline(config));
    }

    pub fn build<F: Facade>(&self, display: &F, packed_tex_folder: Option<&str>) -> TileAtlas {
        let mut textures = Vec::new();

        for (idx, packer) in self.packers.iter().enumerate() {
            let image = ImageExporter::export(packer).unwrap();

            if let Some(s) = packed_tex_folder {
                let mut file_path = PathBuf::from(s);
                file_path.push(&format!("{}.png", idx));

                let mut file = File::create(file_path).unwrap();

                image.save(&mut file, image::PNG).unwrap();
            }

            textures.push(make_texture(display, image));
        }
        TileAtlas {
            locations: self.locations.clone(),
            frames: self.frames.clone(),
            textures: textures,
        }
    }
}

impl TileAtlas {
    pub fn new(locations: HashMap<TileIndex, String>,
               frames: HashMap<String, AtlasFrame>,
               textures: Vec<Texture2d>) -> Self {
        TileAtlas {
            locations: locations,
            frames: frames,
            textures: textures,
        }
    }

    pub fn make_config(&self, file_hash: String) -> TileAtlasConfig {
        TileAtlasConfig {
            locations: self.locations.clone(),
            frames: self.frames.clone(),
            file_hash: file_hash,
        }
    }

    pub fn get_frame(&self, tile_type: TileIndex) -> &AtlasFrame {
        let tex_name = self.locations.get(&tile_type).unwrap();
        self.frames.get(tex_name).unwrap()
    }

    pub fn get_tile_texture_idx(&self, tile_type: TileIndex) -> usize {
        self.get_frame(tile_type).texture_idx
    }


    pub fn get_tilemap_tex_ratio(&self, texture_idx: usize) -> [f32; 2] {
        let dimensions = self.textures.get(texture_idx).unwrap().dimensions();

        let cols: u32 = dimensions.0 / 24;
        let rows: u32 = dimensions.1 / 24;
        [1.0 / cols as f32, 1.0 / rows as f32]
    }

    pub fn get_sprite_tex_ratio(&self, tile_type: TileIndex) -> [f32; 2] {
        let frame = self.get_frame(tile_type);
        let (sx, sy) = frame.tile_size;

        let texture_idx = self.get_frame(tile_type).texture_idx;
        let dimensions = self.textures.get(texture_idx).unwrap().dimensions();

        let cols: f32 = dimensions.0 as f32 / sx as f32;
        let rows: f32 = dimensions.1 as f32 / sy as f32;
        [1.0 / cols, 1.0 / rows]
    }

    pub fn get_tile_texture_size(&self, tile_type: TileIndex) -> (u32, u32) {
        self.get_frame(tile_type).tile_size
    }

    pub fn get_texture_offset(&self, tile_type: TileIndex) -> (f32, f32) {
        let frame = self.get_frame(tile_type);
        let offset = frame.offsets.get(&tile_type).unwrap();

        let get_tex_coords = |index: (u32, u32)| {
            let tex_ratio = self.get_sprite_tex_ratio(tile_type);
            let add_offset = get_add_offset(&frame.rect, &frame.tile_size);

            let tx = (index.0 + add_offset.0) as f32 * tex_ratio[0];
            let ty = (index.1 + add_offset.1) as f32 * tex_ratio[1];

            (tx, ty)
        };

        get_tex_coords(*offset)
    }

    pub fn get_texture(&self, idx: usize) -> &Texture2d {
        self.textures.get(idx).unwrap()
    }

    pub fn passes(&self) -> usize {
        self.textures.len()
    }
}

fn get_add_offset(rect: &AtlasRect, tile_size: &(u32, u32)) -> (u32, u32) {
    let ceil = |a, b| (a + b - 1) / b;
    let cols: u32 = ceil(rect.x, tile_size.0);
    let rows: u32 = ceil(rect.y, tile_size.1);
    (cols, rows)
}
