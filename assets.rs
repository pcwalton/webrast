/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use atlas::{Atlas, AtlasHandle};
use display_list::{DisplayItem, DisplayList};
use distance_field;
use job_server::JobServer;

use euclid::Size2D;
use freetype::{Face, Library};
use freetype::face::RENDER;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

const FONT_SIZE_FOR_RASTERIZATION: i32 = 96;

pub struct AssetContext {
    freetype_library: Library,
    loaded_fonts: HashMap<String,Face<'static>>,
}

impl AssetContext {
    pub fn new() -> AssetContext {
        AssetContext {
            freetype_library: Library::init().unwrap(),
            loaded_fonts: HashMap::new(),
        }
    }
}

pub struct Asset {
    description: AssetDescription,
    pub rasterization_status: AssetRasterizationStatus,
}

#[derive(Clone)]
pub enum AssetDescription {
    Glyph(Glyph),
}

impl AssetDescription {
    pub fn rasterize(&self, context: &mut AssetContext) -> AssetRasterization {
        match *self {
            AssetDescription::Glyph(ref glyph) => glyph.rasterize(context),
        }
    }
}

#[derive(Clone)]
pub struct Glyph {
    pub font_path: String,
    pub character: char,
}

impl Glyph {
    pub fn new(font_path: String, character: char) -> Glyph {
        Glyph {
            font_path: font_path,
            character: character,
        }
    }

    pub fn rasterize(&self, context: &mut AssetContext) -> AssetRasterization {
        let freetype_library = &mut context.freetype_library;
        let font_path = self.font_path.clone();
        let face = context.loaded_fonts
                          .entry(self.font_path.clone())
                          .or_insert_with(|| freetype_library.new_face(font_path, 0).unwrap());
        face.set_char_size(FONT_SIZE_FOR_RASTERIZATION as isize * 64, 0, 50, 0).unwrap();
        face.load_char(self.character as usize, RENDER).unwrap();

        let glyph = face.glyph();
        let bitmap = glyph.bitmap();
        let width = bitmap.width() as u32;
        let height = (FONT_SIZE_FOR_RASTERIZATION - glyph.bitmap_top()) as u32;
        let buffer = bitmap.buffer();
        let distance_field = distance_field::build(buffer, width, height);

        AssetRasterization {
            data: distance_field,
            size: Size2D::new(width, height),
        }

        /*
        for y in 0..height {
            for x in 0..width {
                print!("{}",
                       [' ', '.', ',', '"', ';', 'O', '@', '#'][(distance_field[(y * width + x) as
                       usize] >> 5) as usize])
            }
            println!("");
        }*/
    }
}

pub struct AssetRasterization {
    pub data: Vec<u8>,
    pub size: Size2D<u32>,
}

pub enum AssetRasterizationStatus {
    Pending,
    Waiting(Receiver<AssetRasterization>),
    InMemory(AssetRasterization),
    InAtlas(AssetRasterization, Rc<RefCell<AtlasHandle>>),
}

impl AssetRasterizationStatus {
    pub fn is_pending(&self) -> bool {
        match *self {
            AssetRasterizationStatus::Pending => true,
            _ => false,
        }
    }

    pub fn is_in_atlas(&self) -> bool {
        match *self {
            AssetRasterizationStatus::InAtlas(..) => true,
            _ => false,
        }
    }

    pub fn get_rasterization(&mut self) -> &mut AssetRasterization {
        let rasterized_asset = match *self {
            AssetRasterizationStatus::Pending => {
                panic!("Can't get a pending asset; begin rasterizing it first!")
            }
            AssetRasterizationStatus::InMemory(ref mut rasterized_asset) |
            AssetRasterizationStatus::InAtlas(ref mut rasterized_asset, _) => {
                return rasterized_asset
            }
            AssetRasterizationStatus::Waiting(ref mut receiver) => receiver.recv().unwrap(),
        };
        *self = AssetRasterizationStatus::InMemory(rasterized_asset);
        match *self {
            AssetRasterizationStatus::InMemory(ref mut rasterized_asset) => rasterized_asset,
            _ => unreachable!()
        }
    }

    pub fn set_atlas_handle(&mut self, handle: Rc<RefCell<AtlasHandle>>) {
        let status = mem::replace(&mut *self, AssetRasterizationStatus::Pending);
        let rasterized_asset = match status {
            AssetRasterizationStatus::Pending |
            AssetRasterizationStatus::Waiting(_) => {
                panic!("Can't set an asset handle for an asset that's pending or waiting!")
            }
            AssetRasterizationStatus::InMemory(rasterized_asset) |
            AssetRasterizationStatus::InAtlas(rasterized_asset, _) => rasterized_asset,
        };
        *self = AssetRasterizationStatus::InAtlas(rasterized_asset, handle)
    }

    pub fn get_atlas_handle(&self) -> Rc<RefCell<AtlasHandle>> {
        if let AssetRasterizationStatus::InAtlas(_, ref asset_handle) = *self {
            return (*asset_handle).clone()
        }
        panic!("No asset handle available for this asset!")
    }
}

pub struct AssetManager {
    job_server: Rc<RefCell<JobServer>>,
    pub atlas: Rc<RefCell<Atlas>>,
}

impl AssetManager {
    pub fn new(job_server: Rc<RefCell<JobServer>>, atlas: Rc<RefCell<Atlas>>) -> AssetManager {
        AssetManager {
            job_server: job_server,
            atlas: atlas,
        }
    }

    pub fn create_asset(&self, description: AssetDescription) -> Rc<RefCell<Asset>> {
        // TODO(pcwalton): Maintain a map of assets so we don't rasterize things multiple times.
        Rc::new(RefCell::new(Asset {
            description: description,
            rasterization_status: AssetRasterizationStatus::Pending,
        }))
    }

    pub fn start_rasterizing_asset_if_necessary(&self, asset: &mut Asset) {
        if asset.rasterization_status.is_pending() {
            asset.rasterization_status = AssetRasterizationStatus::Waiting(
                self.job_server.borrow_mut().rasterize_asset(asset.description.clone()))
        }
    }

    pub fn start_rasterizing_assets_in_display_list_as_necessary(&self,
                                                                 display_list: &mut DisplayList) {
        for item in display_list.items.iter_mut() {
            match *item {
                DisplayItem::SolidColor(_) => {}
                DisplayItem::Text(ref mut text_display_item) => {
                    self.start_rasterizing_asset_if_necessary(&mut *text_display_item.asset
                                                                                     .borrow_mut())
                }
            }
        }
    }
}

