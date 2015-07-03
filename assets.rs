/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use atlas::{Atlas, AtlasHandle};
use blur;
use display_list::{DisplayItem, DisplayList};
use distance_field::{self, GLYPH_DISTANCE_SCALING_FACTOR};
use job_server::JobServer;

use euclid::Size2D;
use freetype::{Face, Library};
use freetype::face::RENDER;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

const DISTANCE_FIELD_SIZE: i32 = 96;
const FONT_SIZE_FOR_RASTERIZATION: i32 = 1024;
const DISTANCE_FIELD_RATIO: f32 =
    (DISTANCE_FIELD_SIZE as f32) / (FONT_SIZE_FOR_RASTERIZATION as f32);
const GLYPH_BUFFER_SIZE_RATIO: f32 = 0.5;

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
    derived_from: Option<Rc<RefCell<Asset>>>,
    pub rasterization_status: AssetRasterizationStatus,
}

impl Asset {
    pub fn is_pending_or_waiting_for_dependency(&self) -> bool {
        match self.rasterization_status {
            AssetRasterizationStatus::Pending | AssetRasterizationStatus::WaitingForDependency => {
                true
            }
            _ => false,
        }
    }

    pub fn is_in_atlas(&self) -> bool {
        match self.rasterization_status {
            AssetRasterizationStatus::InAtlas(..) => true,
            _ => false,
        }
    }

    pub fn get_rasterization(&mut self) -> &mut AssetRasterization {
        let rasterized_asset = match self.rasterization_status {
            AssetRasterizationStatus::Pending => {
                panic!("Can't get a pending asset; begin rasterizing it first!")
            }
            AssetRasterizationStatus::WaitingForDependency => {
                panic!("Can't get an asset waiting for its dependency; rasterize its dependency \
                        first!");
            }
            AssetRasterizationStatus::InMemory(ref mut rasterized_asset) |
            AssetRasterizationStatus::InAtlas(ref mut rasterized_asset, _) => {
                return rasterized_asset
            }
            AssetRasterizationStatus::Waiting(ref mut receiver) => receiver.recv().unwrap(),
        };
        self.rasterization_status = AssetRasterizationStatus::InMemory(rasterized_asset);
        match self.rasterization_status {
            AssetRasterizationStatus::InMemory(ref mut rasterized_asset) => rasterized_asset,
            _ => unreachable!()
        }
    }

    pub fn set_atlas_handle(&mut self, handle: Rc<RefCell<AtlasHandle>>) {
        let status = mem::replace(&mut self.rasterization_status,
                                  AssetRasterizationStatus::Pending);
        let rasterized_asset = match status {
            AssetRasterizationStatus::Pending |
            AssetRasterizationStatus::Waiting(_) |
            AssetRasterizationStatus::WaitingForDependency => {
                panic!("Can't set an asset handle for an asset that's pending or waiting!")
            }
            AssetRasterizationStatus::InMemory(rasterized_asset) |
            AssetRasterizationStatus::InAtlas(rasterized_asset, _) => rasterized_asset,
        };
        self.rasterization_status = AssetRasterizationStatus::InAtlas(rasterized_asset, handle)
    }

    pub fn get_atlas_handle(&self) -> Rc<RefCell<AtlasHandle>> {
        if let AssetRasterizationStatus::InAtlas(_, ref asset_handle) = self.rasterization_status {
            return (*asset_handle).clone()
        }
        panic!("No asset handle available for this asset!")
    }
}

#[derive(Clone)]
pub enum AssetDescription {
    Glyph(Glyph),
    BlurredGlyph(BlurredGlyph),
}

impl AssetDescription {
    pub fn rasterize(&self, context: &mut AssetContext, dependency: Option<&AssetRasterization>)
                     -> AssetRasterization {
        match *self {
            AssetDescription::Glyph(ref glyph) => glyph.rasterize(context),
            AssetDescription::BlurredGlyph(ref blurred_glyph) => {
                blurred_glyph.rasterize(context,
                                        dependency.expect("Blurred glyphs need a glyph to blur!"))
            }
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
        let glyph_width = bitmap.width() as u32;
        let glyph_height = (FONT_SIZE_FOR_RASTERIZATION - glyph.bitmap_top()) as u32;
        let buffer = bitmap.buffer();
        let glyph_size = Size2D::new(glyph_width, glyph_height);
        let glyph_size_in_field =
            Size2D::new(((glyph_width as f32) * DISTANCE_FIELD_RATIO) as u32,
                        ((glyph_height as f32) * DISTANCE_FIELD_RATIO) as u32);
        let extra_buffer_size =
            Size2D::new((glyph_size_in_field.width as f32 * GLYPH_BUFFER_SIZE_RATIO) as u32,
                        (glyph_size_in_field.height as f32 * GLYPH_BUFFER_SIZE_RATIO) as u32);
        let distance_field_size =
            Size2D::new(glyph_size_in_field.width + extra_buffer_size.width,
                        glyph_size_in_field.height + extra_buffer_size.height);
        let distance_field = distance_field::build_distance_field_for_glyph(buffer,
                                                                            &glyph_size,
                                                                            &glyph_size_in_field,
                                                                            &distance_field_size);

        AssetRasterization {
            data: distance_field,
            size: distance_field_size,
        }
    }
}

#[derive(Clone)]
pub struct BlurredGlyph {
    pub sigma: f32,
}

impl BlurredGlyph {
    pub fn new(sigma: f32) -> BlurredGlyph {
        BlurredGlyph {
            sigma: sigma,
        }
    }

    pub fn rasterize(&self, context: &mut AssetContext, dependency: &AssetRasterization)
                     -> AssetRasterization {
        let data =
            blur::approximate_gaussian_blur_with_distance_field(&dependency.data[..],
                                                                GLYPH_DISTANCE_SCALING_FACTOR,
                                                                &dependency.size,
                                                                self.sigma);
        AssetRasterization {
            data: data,
            size: dependency.size,
        }
    }
}

#[derive(Clone)]
pub struct AssetRasterization {
    pub data: Vec<u8>,
    pub size: Size2D<u32>,
}

pub enum AssetRasterizationStatus {
    Pending,
    WaitingForDependency,
    Waiting(Receiver<AssetRasterization>),
    InMemory(AssetRasterization),
    InAtlas(AssetRasterization, Rc<RefCell<AtlasHandle>>),
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

    pub fn create_asset(&self,
                        description: AssetDescription,
                        derived_from: Option<Rc<RefCell<Asset>>>)
                        -> Rc<RefCell<Asset>> {
        // TODO(pcwalton): Maintain a map of assets so we don't rasterize things multiple times.
        Rc::new(RefCell::new(Asset {
            description: description,
            rasterization_status: AssetRasterizationStatus::Pending,
            derived_from: derived_from,
        }))
    }

    pub fn start_rasterizing_asset_if_necessary(&self, asset: &mut Asset) {
        if !asset.is_pending_or_waiting_for_dependency() {
            return
        }

        let derived_from = match asset.derived_from {
            Some(ref derived_from) => derived_from,
            None => {
                asset.rasterization_status = AssetRasterizationStatus::Waiting(
                    self.job_server.borrow_mut().rasterize_asset(asset.description.clone(), None));
                return
            }
        };

        match derived_from.borrow().rasterization_status {
            AssetRasterizationStatus::Pending => {
                panic!("Can't rasterize an asset that's derived from a pending one; start
                        rasterizing the asset it's derived from first!")
            }
            AssetRasterizationStatus::Waiting(_) => {
                asset.rasterization_status = AssetRasterizationStatus::WaitingForDependency
            }
            AssetRasterizationStatus::WaitingForDependency => {}
            AssetRasterizationStatus::InMemory(ref rasterization) |
            AssetRasterizationStatus::InAtlas(ref rasterization, _) => {
                asset.rasterization_status = AssetRasterizationStatus::Waiting(
                    self.job_server
                        .borrow_mut()
                        .rasterize_asset(asset.description.clone(),
                                         Some((*rasterization).clone())));
            }
        }
    }

    pub fn start_rasterizing_assets_in_display_list_as_necessary(&self,
                                                                 display_list: &mut DisplayList) {
        for item in display_list.items.iter_mut() {
            match *item {
                DisplayItem::SolidColor(_) => {}
                DisplayItem::Text(ref mut text_display_item) => {
                    self.start_rasterizing_asset_if_necessary(
                        &mut *text_display_item.glyph_asset.borrow_mut());
                    if let Some(ref blurred_glyph_asset) = text_display_item.blurred_glyph_asset {
                        self.start_rasterizing_asset_if_necessary(
                            &mut *blurred_glyph_asset.borrow_mut())
                    }
                }
            }
        }
    }
}

