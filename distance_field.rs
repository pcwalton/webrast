/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A very naïve O((width * height)²) implementation of distance field generation.
//!
//! TODO(pcwalton): Replace with a better algorithm.

use euclid::{Point2D, Size2D};
use std::cmp;
use std::f32;

pub const BUFFER: u8 = 192;

pub const GLYPH_DISTANCE_SCALING_FACTOR: f32 = 10.0;
const ARC_DISTANCE_SCALING_FACTOR: f32 = 256.0;

pub fn build_distance_field_for_glyph(data: &[u8],
                                      glyph_size: &Size2D<u32>,
                                      glyph_size_in_field: &Size2D<u32>,
                                      field_size: &Size2D<u32>)
                                      -> Vec<u8> {
    let mut result = Vec::with_capacity((field_size.width * field_size.height * 4) as usize);
    let offset_from_field_to_glyph =
        Point2D::new(((field_size.width - glyph_size_in_field.width) / 2),
                     ((field_size.height - glyph_size_in_field.height) / 2));
    let ratio = (glyph_size.width as f32) / (glyph_size_in_field.width as f32);
    for y0 in 0..field_size.height {
        for x0 in 0..field_size.width {
            let glyph_point_inside_field =
                Point2D::new((x0 as i32) - (offset_from_field_to_glyph.x as i32),
                             (y0 as i32) - (offset_from_field_to_glyph.y as i32));
            let glyph_point = Point2D::new(((glyph_point_inside_field.x as f32) * ratio) as i32,
                                           ((glyph_point_inside_field.y as f32) * ratio) as i32);
            let inside_glyph = glyph_point.x > 0 && glyph_point.y > 0 &&
                glyph_point.x < glyph_size.width as i32 &&
                glyph_point.y < glyph_size.height as i32 &&
                data[(glyph_point.y * (glyph_size.width as i32) + glyph_point.x) as usize] != 0;
            let mut distance = 127.0;
            for y1 in 0..glyph_size.height {
                for x1 in 0..glyph_size.width {
                    if glyph_point == Point2D::new(x1 as i32, y1 as i32) {
                        continue
                    }
                    let test_point_inside_glyph = data[(y1 * glyph_size.width + x1) as usize] != 0;
                    if test_point_inside_glyph == inside_glyph {
                        continue
                    }
                    let (x0, y0) = (glyph_point.x as f32, glyph_point.y as f32);
                    let (x1, y1) = (x1 as f32, y1 as f32);
                    let (y_delta, x_delta) = (y1 - y0, x1 - x0);
                    let this_distance = f32::sqrt(y_delta * y_delta + x_delta * x_delta);
                    if this_distance < distance {
                        distance = this_distance
                    }
                }
            }
            let mut value = if inside_glyph {
                (BUFFER as i64 + (((distance * GLYPH_DISTANCE_SCALING_FACTOR) -
                                   GLYPH_DISTANCE_SCALING_FACTOR) as i64))
            } else {
                (BUFFER as i64 - ((distance * GLYPH_DISTANCE_SCALING_FACTOR) as i64))
            };
            if value < 0 {
                value = 0
            } else if value > 255 {
                value = 255
            }
            let value = value as u8;
            result.extend([ 255, 255, 255, value ].iter());
        }
    }
    result
}

pub fn build_distance_field_for_filled_arc(size: u32, radius: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity((size * size * 4) as usize);
    let radius = radius as f32;
    for y in 0..size {
        for x in 0..size {
            let delta = Point2D::new(size - x, size - y);
            let distance_to_center = f32::sqrt((delta.y * delta.y + delta.x * delta.x) as f32);
            let distance = distance_to_center - radius;
            let mut scaled_distance =
                (1.0 - distance / ARC_DISTANCE_SCALING_FACTOR) * (BUFFER as f32);
            if scaled_distance < 0.0 {
                scaled_distance = 0.0
            } else if scaled_distance > 255.0 {
                scaled_distance = 255.0
            }
            let value = scaled_distance as u8;
            result.extend([ 255, 255, 255, value ].iter());
        }
    }
    result
}

