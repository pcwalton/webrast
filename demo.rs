/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate glutin;

use batch::Batcher;
use context::Context;
use display_list::{Au, BaseDisplayItem, ClippingRegion, Color, DisplayItem, DisplayList};
use display_list::{SolidColorDisplayItem};
use draw::DrawContext;

use demo::glutin::{Api, GlRequest, WindowBuilder};
use euclid::point::Point2D;
use euclid::rect::Rect;
use euclid::size::Size2D;
use gleam::gl;
use log::{self, Log, LogLevelFilter, LogMetadata, LogRecord};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{}", record.args());
        }
    }
}

#[main]
pub fn main() {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Debug);
        Box::new(SimpleLogger)
    }).unwrap();

    let window = WindowBuilder::new().with_gl(GlRequest::Specific(Api::OpenGl, (2, 1)))
                                     .build()
                                     .unwrap();
    window.set_title("webrast demo");
    gl::load_with(|symbol| window.get_proc_address(symbol));
    unsafe {
        window.make_current();
    }

    let display_list = DisplayList {
        items: vec![
            DisplayItem::SolidColor(Box::new(SolidColorDisplayItem {
                base: BaseDisplayItem {
                    bounds: Rect::new(Point2D::new(Au::from_px(60), Au::from_px(60)),
                                      Size2D::new(Au::from_px(240), Au::from_px(240))),
                    clip: ClippingRegion {
                        main: Rect::new(Point2D::new(Au::from_px(60), Au::from_px(100)),
                                        Size2D::new(Au::from_px(240), Au::from_px(160))),
                    },
                },
                color: Color {
                    r: 128,
                    g: 0,
                    b: 128,
                    a: 255,
                },
            })),
            DisplayItem::SolidColor(Box::new(SolidColorDisplayItem {
                base: BaseDisplayItem {
                    bounds: Rect::new(Point2D::new(Au::from_px(150), Au::from_px(150)),
                                      Size2D::new(Au::from_px(240), Au::from_px(240))),
                    clip: ClippingRegion {
                        main: Rect::new(Point2D::new(Au::from_px(170), Au::from_px(180)),
                                        Size2D::new(Au::from_px(200), Au::from_px(160))),
                    },
                },
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            })),
        ],
    };

    let context = Context {
        render_target_size: Size2D::new(800, 600),
    };
    let mut draw_context = DrawContext::new();

    let mut batcher = Batcher::new();
    for item in display_list.items.into_iter() {
        batcher.add(&context, item)
    }
    let batches = batcher.finish();

    draw_context.init_gl_state();
    draw_context.clear();
    for batch in batches.into_iter() {
        draw_context.draw_batch(&batch)
    }

    window.swap_buffers();

    while !window.is_closed() {
        window.wait_events().next();
    }
}

