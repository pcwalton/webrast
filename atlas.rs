/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Texture atlas management.

use assets::Asset;

use euclid::{Point2D, Rect, Size2D};
use gleam::gl::{self, GLint, GLuint};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

pub const WIDTH: GLuint = 1024;
pub const HEIGHT: GLuint = 1024;

pub struct Atlas {
    pub texture: GLuint,
}

impl Atlas {
    pub fn new() -> Atlas {
        let texture = gl::gen_textures(1)[0];
        gl::bind_texture(gl::TEXTURE_2D, texture);

        let mut buffer = Vec::new();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                buffer.extend([ 0, 0, 255, 255 ].iter());
            }
        }
        gl::tex_image_2d(gl::TEXTURE_2D,
                         0,
                         gl::RGBA as GLint,
                         WIDTH as GLint,
                         HEIGHT as GLint,
                         0,
                         gl::RGBA,
                         gl::UNSIGNED_BYTE,
                         Some(&buffer[..]));

        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
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

        {
            let mut file = File::create("atlas.tga").unwrap();
            let mut header = [ 0; 18 ];
            header[2] = 2;
            header[12] = location.rect.size.width as u8;
            header[13] = (location.rect.size.width >> 8) as u8;
            header[14] = location.rect.size.height as u8;
            header[15] = (location.rect.size.height >> 8) as u8;
            header[16] = 24;
            file.write(&header).unwrap();
            for y in 0..(location.rect.size.height as usize) {
                let y = (location.rect.size.height as usize) - y - 1;
                for x in 0..(location.rect.size.width as usize) {
                    let a = buffer[4 * (y * (location.rect.size.width as usize) + x) + 3];
                    file.write(&[ a, a, a ]).unwrap();
                }
            }
        }

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

