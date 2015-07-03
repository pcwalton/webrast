/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use atlas;
use distance_field::BUFFER;

use euclid::Size2D;
use std::f32::consts::PI;
use std::f32;

pub fn approximate_gaussian_blur_with_distance_field(distance_field: &[u8],
                                                     distance_scaling_factor: f32,
                                                     size: &Size2D<u32>,
                                                     sigma: f32)
                                                     -> Vec<u8> {
    atlas::write_tga("distance-field.tga", distance_field, size);

    let blur_radius = f32::ceil(sigma * 3.0) as i32;
    let mut convolution = Vec::with_capacity(blur_radius as usize * 2 + 1);
    let two_sigma_squared = 2.0 * sigma * sigma;
    let a = 1.0 / f32::sqrt(PI * two_sigma_squared);
    for x in (-blur_radius)..(blur_radius + 1) {
        let x = x as f32;
        convolution.push(a * f32::exp(-x * x / two_sigma_squared))
    }

    // Precompute ∫₋ₐⁱf(x)dx, where a is the blur radius and f(x) is the Gaussian function for all
    // i in [-a..a].
    //
    // This results in the correct values for a blur in one direction as long as the blur is less
    // than the thickness of the narrowest path in the vector. We use this to approximate a
    // Gaussian blur for the whole image.
    let mut precomputed_values = Vec::with_capacity(convolution.len());
    for i in 0..convolution.len() {
        let mut sum = 0.0;
        for j in 0..i {
            sum += convolution[j]
        }
        precomputed_values.push(sum)
    }

    let mut result = Vec::with_capacity((size.width * size.height * 4) as usize);
    for y in 0..size.height {
        for x in 0..size.width {
            let mut distance = distance_field[((y * size.width + x) * 4 + 3) as usize] as f32;
            distance = (distance - (BUFFER as f32)) / distance_scaling_factor;

            let color = if distance < -(blur_radius as f32) {
                0
            } else if distance > blur_radius as f32 {
                255
            } else {
                let distance = f32::round(distance) as i32;
                let index = distance + ((precomputed_values.len() as i32) - 1) / 2;
                f32::round(precomputed_values[index as usize] as f32 * 255.0) as u8
            };
            /*println!("{}", color);
            let color = if distance < 0.0 {
                0
            } else {
                255
            };*/

            result.extend([ color, color, color, color ].iter())
        }
    }
    result
}

