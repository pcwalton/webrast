/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Texture atlas management.

use assets::Asset;

use euclid::{Point2D, Rect, Size2D};
use gleam::gl::{self, GLint, GLuint};
use std::cell::RefCell;
use std::rc::Rc;

pub const WIDTH: GLuint = 1024;
pub const HEIGHT: GLuint = 1024;

pub struct Atlas {
    texture: GLuint,
}

impl Atlas {
    pub fn new() -> Atlas {
        let texture = gl::gen_textures(1)[0];
        gl::bind_texture(gl::TEXTURE_2D, texture);
        gl::tex_image_2d(gl::TEXTURE_2D,
                         0,
                         gl::RGBA as GLint,
                         WIDTH as GLint,
                         HEIGHT as GLint,
                         0,
                         gl::RGBA,
                         gl::UNSIGNED_BYTE,
                         None);
        Atlas {
            texture: texture,
        }
    }

    pub fn require_asset(&mut self, asset: &mut Asset, priority: Priority) {
        if asset.rasterization_status.is_in_atlas() {
            return
        }

        let handle = {
            let rasterization = asset.rasterization_status.get_rasterization();
            let location = self.allocate(priority, &rasterization.size);
            self.upload(&location, &rasterization.data[..])
        };

        asset.rasterization_status.set_atlas_handle(handle);
    }

    fn allocate(&mut self, _: Priority, size: &Size2D<u32>) -> AtlasLocation {
        AtlasLocation {
            rect: Rect::new(Point2D::new(0, 0), *size),
        }
    }

    fn upload(&mut self, location: &AtlasLocation, buffer: &[u8]) -> Rc<RefCell<AtlasHandle>> {
        println!("buffer size={}, location={:?}", buffer.len(), location.rect);
        assert!(buffer.len() >=
                (location.rect.size.width * location.rect.size.height * 4) as usize);
        gl::bind_texture(gl::TEXTURE_2D, self.texture);
        gl::tex_sub_image_2d(gl::TEXTURE_2D,
                             0,
                             location.rect.origin.x as GLint,
                             location.rect.origin.y as GLint,
                             location.rect.size.width as GLint,
                             location.rect.size.height as GLint,
                             gl::RGBA,
                             gl::UNSIGNED_BYTE,
                             buffer);
        Rc::new(RefCell::new(AtlasHandle {
            location: *location,
        }))
    }
}

/// A reference to an object in the atlas.
pub struct AtlasHandle {
    pub location: AtlasLocation,
}

#[derive(Copy, Clone)]
pub struct AtlasLocation {
    pub rect: Rect<u32>,
}

/// Priority of assets in the atlas, from lowest to highest.
#[derive(Copy, Clone, PartialEq)]
pub enum Priority {
    /// An item in the retained display list needs this asset.
    Retained = 0,
}

