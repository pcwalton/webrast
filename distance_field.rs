/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A very naïve O((width * height)²) implementation of distance field generation.
//!
//! TODO(pcwalton): Replace with a better algorithm.

use std::cmp;
use std::f32;

const FIELD_CUTOFF: u8 = 128;

pub fn build(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity((width * height * 4) as usize);
    for y0 in 0..height {
        for x0 in 0..width {
            let inside_glyph = data[(y0 * width + x0) as usize] != 0;
            let mut distance = 0;
            for y1 in 0..height {
                for x1 in 0..width {
                    if (x0, y0) == (x1, y1) {
                        continue
                    }
                    let test_point_inside_glyph = data[(y1 * width + x1) as usize] != 0;
                    if test_point_inside_glyph == inside_glyph {
                        continue
                    }
                    let (x0, y0, x1, y1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
                    let (y_delta, x_delta) = (y1 - y0, x1 - x0);
                    distance = cmp::max(distance,
                                        f32::sqrt(y_delta * y_delta + x_delta * x_delta) as u8);
                }
            }
            let value = if inside_glyph {
                FIELD_CUTOFF + distance
            } else {
                FIELD_CUTOFF - distance
            };
            result.extend([ 255, 255, 255, value ].iter());
        }
    }
    result
}

