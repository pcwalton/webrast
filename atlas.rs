/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Texture atlas management.
//!
//! Currently, the bin packing algorithm is the simple one described here:
//!
//!     http://www.blackpawn.com/texts/lightmaps/default.html

use assets::Asset;

use euclid::{Point2D, Rect, Size2D};
use gleam::gl::{self, GLint, GLuint};
use std::cell::RefCell;
use std::cmp;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering};

pub const WIDTH: GLuint = 1024;
pub const HEIGHT: GLuint = 1024;

pub struct Atlas {
    pub texture: GLuint,
    root_bin: Bin,
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
            root_bin: Bin::new(&Rect::new(Point2D::new(0, 0), Size2D::new(WIDTH, HEIGHT))),
        }
    }

    pub fn require_asset(&mut self, asset: &mut Asset, priority: Priority) {
        if asset.is_in_atlas() {
            return
        }

        let handle = {
            let rasterization = asset.get_rasterization();
            let location = self.allocate(priority, &rasterization.size);
            self.upload(&location, &rasterization.data[..])
        };

        asset.set_atlas_handle(handle);
    }

    fn allocate(&mut self, _: Priority, size: &Size2D<u32>) -> AtlasLocation {
        // TODO(pcwalton): Evict old objects.
        let point = self.root_bin.insert(size).expect("Atlas out of space!");
        println!("placing object at {:?}", point);
        AtlasLocation {
            rect: Rect::new(point, *size),
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
            static ATLAS_INDEX: AtomicUsize = ATOMIC_USIZE_INIT;

            let value = ATLAS_INDEX.fetch_add(1, Ordering::SeqCst);
            write_tga(&format!("atlas{}.tga", value), buffer, &location.rect.size);
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

struct Bin {
    children: Option<[Box<Bin>; 2]>,
    rect: Rect<u32>,
    full: bool,
}

impl Bin {
    fn new(rect: &Rect<u32>) -> Bin {
        Bin {
            children: None,
            rect: *rect,
            full: false,
        }
    }

    fn insert(&mut self, size: &Size2D<u32>) -> Option<Point2D<u32>> {
        if let Some(ref mut children) = self.children {
            let (left, right) = children.split_at_mut(1);
            let (left, right) = (&mut left[0], &mut right[0]);
            return left.insert(size).or_else(|| right.insert(size))
        }

        if self.full {
            return None
        }

        match (self.rect.size.width.cmp(&size.width), self.rect.size.height.cmp(&size.height)) {
            (cmp::Ordering::Less, _) | (_, cmp::Ordering::Less) => return None,
            (cmp::Ordering::Equal, cmp::Ordering::Equal) => {
                self.full = true;
                return Some(self.rect.origin)
            }
            _ => {}
        }

        let left_child = Box::new(Bin::new(&Rect::new(self.rect.origin, *size)));

        let extra_width = self.rect.size.width - size.width;
        let extra_height = self.rect.size.height - size.height;
        let right_child = if extra_width > extra_height {
            Box::new(Bin::new(&Rect::new(Point2D::new(self.rect.origin.x + size.width,
                                                      self.rect.origin.y),
                                         Size2D::new(self.rect.size.width - size.width,
                                                     self.rect.size.height))))
        } else {
            Box::new(Bin::new(&Rect::new(Point2D::new(self.rect.origin.x,
                                                      self.rect.origin.y + size.height),
                                         Size2D::new(self.rect.size.width,
                                                     self.rect.size.height - size.height))))
        };

        self.children = Some([ left_child, right_child ]);
        self.children.as_mut().unwrap()[0].insert(size)
    }
}

pub fn write_tga(name: &str, buffer: &[u8], size: &Size2D<u32>) {
    let mut file = File::create(name).unwrap();
    let mut header = [ 0; 18 ];
    header[2] = 2;
    header[12] = size.width as u8;
    header[13] = (size.width >> 8) as u8;
    header[14] = size.height as u8;
    header[15] = (size.height >> 8) as u8;
    header[16] = 24;
    file.write(&header).unwrap();
    for y in 0..(size.height as usize) {
        let y = (size.height as usize) - y - 1;
        for x in 0..(size.width as usize) {
            let a = buffer[4 * (y * (size.width as usize) + x) + 3];
            file.write(&[ a, a, a ]).unwrap();
        }
    }
}

