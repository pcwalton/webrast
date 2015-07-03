/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use assets::Asset;
use atlas::{self, Priority};
use context::Context;
use display_list::{Au, BLACK, ClippingRegion, Color, DisplayItem, TRANSPARENT_BLACK};
use display_list::{TRANSPARENT_GREEN, TextDisplayItem, WHITE};
use distance_field;

use euclid::{Point2D, Point3D, Rect, Size2D};
use std::cell::RefCell;
use std::iter;

const NEAR_DEPTH_VALUE: f32 = -0.5;
const FAR_DEPTH_VALUE: f32 = 0.5;

const BUFFER: f32 = (distance_field::BUFFER as f32) / 255.0;
const GAMMA: f32 = 0.01;

pub struct Batch {
    pub vertices: Vec<Point3D<f32>>,
    pub colors: Vec<Color>,
    pub buffer_gamma: Vec<Point2D<f32>>,
    pub texture_coords: Vec<Point2D<f32>>,
    pub elements: Vec<u32>,
}

impl Batch {
    fn new() -> Batch {
        Batch {
            vertices: Vec::new(),
            colors: Vec::new(),
            buffer_gamma: Vec::new(),
            texture_coords: Vec::new(),
            elements: Vec::new(),
        }
    }

    fn add_vertices_for_rect(&mut self, context: &Context, rect: &Rect<Au>, z_value: f32) {
        let rect = rect.to_normalized_device_position(context);
        self.vertices.extend([
            Point3D::new(rect.origin.x, -rect.origin.y, z_value),
            Point3D::new(rect.max_x(), -rect.origin.y, z_value),
            Point3D::new(rect.origin.x, -rect.max_y(), z_value),
            Point3D::new(rect.max_x(), -rect.max_y(), z_value),
        ].iter());
    }

    fn add_solid_colors(&mut self, count: usize, color: &Color) {
        self.colors.extend(iter::repeat(*color).take(count))
    }

    fn add_buffer_gamma(&mut self, count: usize, buffer: f32, gamma: f32) {
        self.buffer_gamma.extend(iter::repeat(Point2D::new(buffer, gamma)).take(count))
    }

    fn add_dummy_buffer_gamma(&mut self, count: usize) {
        self.add_buffer_gamma(count, 0.0, 0.0)
    }

    fn add_texture_coords_for_rect(&mut self, texture_rect: &Rect<u32>) {
        let (atlas_width, atlas_height) = (atlas::WIDTH as f32, atlas::HEIGHT as f32);
        let texture_rect =
            Rect::new(Point2D::new((texture_rect.origin.x as f32) / atlas_width,
                                   (texture_rect.origin.y as f32) / atlas_height),
                      Size2D::new((texture_rect.size.width as f32) / atlas_width,
                                  (texture_rect.size.height as f32) / atlas_height));
            //Rect::new(Point2D::new(0.0, 0.0), Size2D::new(1.0, 1.0));
        self.texture_coords.extend([
            texture_rect.origin,
            texture_rect.top_right(),
            texture_rect.bottom_left(),
            texture_rect.bottom_right(),
        ].iter());
    }

    fn add_dummy_texture_coords(&mut self, count: usize) {
        self.texture_coords.extend(iter::repeat(Point2D::new(0.0, 0.0)).take(count))
    }

    fn add_elements_for_clockwise_wound_rect(&mut self) {
        let bottom_right = self.vertices.len() as u32 - 1;
        let bottom_left = bottom_right - 1;
        let top_right = bottom_left - 1;
        let top_left = top_right - 1;
        self.elements.extend([
            top_left,
            top_right,
            bottom_left,
            bottom_left,
            top_right,
            bottom_right,
        ].iter());
    }

    fn add_elements_for_counterclockwise_wound_rect(&mut self) {
        let bottom_right = self.vertices.len() as u32 - 1;
        let bottom_left = bottom_right - 1;
        let top_right = bottom_left - 1;
        let top_left = top_right - 1;
        self.elements.extend([
            top_left,
            bottom_left,
            top_right,
            top_right,
            bottom_left,
            bottom_right,
        ].iter());
    }

    // TODO(pcwalton): Only clear clips if we need to.
    // TODO(pcwalton): Clip by adjusting vertices and texture coordinates for simple clips.
    fn clear_clip(&mut self, context: &Context) {
        let rect = Rect::new(Point2D::new(Au::from_px(0), Au::from_px(0)),
                             context.render_target_size.to_au());
        self.add_vertices_for_rect(context, &rect, FAR_DEPTH_VALUE);
        self.add_solid_colors(4, &WHITE);
        self.add_dummy_buffer_gamma(4);
        self.add_dummy_texture_coords(4);
        self.add_elements_for_clockwise_wound_rect();
    }

    // TODO(pcwalton): Only add clips if we need to.
    // TODO(pcwalton): Clip by adjusting vertices and texture coordinates for simple clips.
    fn add_clip(&mut self, context: &Context, clipping_region: &ClippingRegion) {
        self.add_vertices_for_rect(context, &clipping_region.main, NEAR_DEPTH_VALUE);
        self.add_solid_colors(4, &TRANSPARENT_GREEN);
        self.add_dummy_buffer_gamma(4);
        self.add_dummy_texture_coords(4);
        self.add_elements_for_clockwise_wound_rect();
    }

    fn add_solid_color_rect(&mut self, context: &Context, rect: &Rect<Au>, color: &Color) {
        self.add_vertices_for_rect(context, rect, NEAR_DEPTH_VALUE);
        self.add_solid_colors(4, color);
        self.add_dummy_buffer_gamma(4);
        self.add_dummy_texture_coords(4);
        self.add_elements_for_counterclockwise_wound_rect();
    }

    fn add_text(&mut self,
                context: &mut Context,
                bounds: &Rect<Au>,
                glyph_asset: &RefCell<Asset>,
                blurred_glyph_asset: Option<&mut Asset>) {
        context.asset_manager.atlas.borrow_mut().require_asset(&mut *glyph_asset.borrow_mut(),
                                                               Priority::Retained);
        match blurred_glyph_asset {
            None => {
                let atlas_handle = glyph_asset.borrow().get_atlas_handle();

                self.add_vertices_for_rect(context, bounds, NEAR_DEPTH_VALUE);
                self.add_solid_colors(4, &TRANSPARENT_BLACK);
                self.add_buffer_gamma(4, BUFFER, GAMMA);
                self.add_texture_coords_for_rect(&atlas_handle.borrow().location.rect);
                self.add_elements_for_counterclockwise_wound_rect();
            }
            Some(blurred_glyph_asset) => {
                // TODO(pcwalton): We should have a service that automatically starts rasterizing
                // dependencies so we don't have to block on it here!
                context.asset_manager.start_rasterizing_asset_if_necessary(blurred_glyph_asset);
                context.asset_manager.atlas.borrow_mut().require_asset(blurred_glyph_asset,
                                                                       Priority::Retained);
                let atlas_handle = blurred_glyph_asset.get_atlas_handle();

                self.add_vertices_for_rect(context, bounds, NEAR_DEPTH_VALUE);
                self.add_solid_colors(4, &TRANSPARENT_BLACK);
                self.add_dummy_buffer_gamma(4);
                self.add_texture_coords_for_rect(&atlas_handle.borrow().location.rect);
                self.add_elements_for_counterclockwise_wound_rect();
            }
        }
    }
}

pub struct Batcher {
    pending_batch: Batch,
}

impl Batcher {
    pub fn new() -> Batcher {
        Batcher {
            pending_batch: Batch::new(),
        }
    }

    pub fn add(&mut self, context: &mut Context, display_item: &mut DisplayItem) {
        /*self.pending_batch.clear_clip(context);
        self.pending_batch.add_clip(context, &display_item.base().clip);*/

        match *display_item {
            DisplayItem::SolidColor(ref mut solid_color_display_item) => {
                self.pending_batch.add_solid_color_rect(context,
                                                        &solid_color_display_item.base.bounds,
                                                        &solid_color_display_item.color);
            }
            DisplayItem::Text(ref mut text_display_item) => {
                let text_display_item = &mut **text_display_item;
                match *text_display_item {
                    TextDisplayItem {
                        base: ref mut base,
                        glyph_asset: ref mut glyph_asset,
                        blurred_glyph_asset: None,
                        ..
                    } => {
                        self.pending_batch.add_text(context, &base.bounds, &*glyph_asset, None);
                    }
                    TextDisplayItem {
                        base: ref mut base,
                        glyph_asset: ref mut glyph_asset,
                        blurred_glyph_asset: Some(ref mut blurred_glyph_asset),
                        ..
                    } => {
                        self.pending_batch.add_text(context,
                                                    &base.bounds,
                                                    &*glyph_asset,
                                                    Some(&mut blurred_glyph_asset.borrow_mut()));
                    }
                }
            }
        }
    }

    pub fn finish(self) -> Vec<Batch> {
        vec![self.pending_batch]
    }
}

trait ToNormalizedDevicePosition {
    type To;

    fn to_normalized_device_position(&self, context: &Context) -> Self::To;
}

impl ToNormalizedDevicePosition for Rect<Au> {
    type To = Rect<f32>;

    fn to_normalized_device_position(&self, context: &Context) -> Rect<f32> {
        Rect::new(self.origin.to_normalized_device_position(context),
                  self.size.to_normalized_device_position(context))
    }
}

impl ToNormalizedDevicePosition for Point2D<Au> {
    type To = Point2D<f32>;

    fn to_normalized_device_position(&self, context: &Context) -> Point2D<f32> {
        Point2D::new(((self.x.to_px() as f32) / (context.render_target_size.width as f32) - 0.5) *
                     2.0,
                     ((self.y.to_px() as f32) / (context.render_target_size.height as f32) - 0.5) *
                     2.0)
    }
}

impl ToNormalizedDevicePosition for Size2D<Au> {
    type To = Size2D<f32>;

    fn to_normalized_device_position(&self, context: &Context) -> Size2D<f32> {
        Size2D::new((self.width.to_px() as f32) / (context.render_target_size.width as f32) * 2.0,
                    (self.height.to_px() as f32) / (context.render_target_size.height as f32) *
                     2.0)
    }
}

trait ToAu {
    type To;

    fn to_au(&self) -> Self::To;
}

impl ToAu for Rect<i32> {
    type To = Rect<Au>;

    fn to_au(&self) -> Rect<Au> {
        Rect::new(self.origin.to_au(), self.size.to_au())
    }
}

impl ToAu for Point2D<i32> {
    type To = Point2D<Au>;

    fn to_au(&self) -> Point2D<Au> {
        Point2D::new(self.x.to_au(), self.y.to_au())
    }
}

impl ToAu for Size2D<i32> {
    type To = Size2D<Au>;

    fn to_au(&self) -> Size2D<Au> {
        Size2D::new(self.width.to_au(), self.height.to_au())
    }
}

impl ToAu for i32 {
    type To = Au;

    fn to_au(&self) -> Au {
        Au::from_px(*self)
    }
}

