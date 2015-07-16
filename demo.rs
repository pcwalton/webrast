/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate clock_ticks;
extern crate glutin;

use assets::{ArcAsset, ArcMode, AssetDescription, AssetManager, BlurredGlyph, Glyph};
use atlas::Atlas;
use batch::Batcher;
use context::Context;
use display_list::{Au, BaseDisplayItem, BorderDisplayItem, ClippingRegion, Color, DisplayItem};
use display_list::{DisplayList, SolidColorDisplayItem, TextDisplayItem};
use draw::DrawContext;
use job_server::JobServer;

use demo::glutin::{Api, GlRequest, WindowBuilder};
use euclid::point::Point2D;
use euclid::rect::Rect;
use euclid::size::Size2D;
use gleam::gl;
use log::{self, Log, LogLevelFilter, LogMetadata, LogRecord};
use num_cpus;
use std::cell::RefCell;
use std::rc::Rc;

static FONT_PATH: &'static str = "/Users/pcwalton/Library/Fonts/Montserrat-Regular.ttf";

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

struct Timer<'a> {
    description: &'a str,
    start_time: u64,
}

impl<'a> Timer<'a> {
    fn new(description: &str) -> Timer {
        Timer {
            description: description,
            start_time: clock_ticks::precise_time_ns(),
        }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        let elapsed = clock_ticks::precise_time_ns() - self.start_time;
        println!("{}: {}ms", self.description, (elapsed as f64) / 1000000.0)
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

    let atlas = Rc::new(RefCell::new(Atlas::new()));
    let job_server = Rc::new(RefCell::new(JobServer::new(num_cpus::get() as u32)));
    let asset_manager = AssetManager::new(job_server, atlas.clone());

    /*let glyph_asset =
        asset_manager.create_asset(AssetDescription::Glyph(Glyph::new(FONT_PATH.to_string(),
                                                                      'S')),
                                   None);*/
    let mut display_list = DisplayList {
        items: vec![
            /*DisplayItem::SolidColor(Box::new(SolidColorDisplayItem {
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
            })),*/
            /*
            DisplayItem::Text(Box::new(TextDisplayItem {
                base: BaseDisplayItem {
                    bounds: Rect::new(Point2D::new(Au::from_px(0), Au::from_px(0)),
                                      Size2D::new(Au::from_px(54), Au::from_px(72))),
                    clip: ClippingRegion {
                        main: Rect::new(Point2D::new(Au::from_px(200), Au::from_px(200)),
                                        Size2D::new(Au::from_px(100), Au::from_px(100))),
                    },
                },
                glyph_asset: glyph_asset.clone(),
                blurred_glyph_asset:
                    Some(asset_manager.create_asset(AssetDescription::BlurredGlyph(
                                BlurredGlyph::new(10.0)), Some(glyph_asset))),
            })),*/
            /*DisplayItem::SolidColor(Box::new(SolidColorDisplayItem {
                base: BaseDisplayItem {
                    bounds: Rect::new(Point2D::new(Au::from_px(0), Au::from_px(0)),
                                      Size2D::new(Au::from_px(1), Au::from_px(1))),
                    clip: ClippingRegion {
                        main: Rect::new(Point2D::new(Au::from_px(200), Au::from_px(200)),
                                        Size2D::new(Au::from_px(100), Au::from_px(100))),
                    },
                },
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            })),*/
            DisplayItem::Border(Box::new(BorderDisplayItem {
                base: BaseDisplayItem {
                    bounds: Rect::new(Point2D::new(Au::from_px(0), Au::from_px(0)),
                                      Size2D::new(Au::from_px(100), Au::from_px(100))),
                    clip: ClippingRegion {
                        main: Rect::new(Point2D::new(Au::from_px(200), Au::from_px(200)),
                                        Size2D::new(Au::from_px(100), Au::from_px(100))),
                    },
                },
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: Au::from_px(150),
                radius: Au::from_px(50),
                arc_asset: asset_manager.create_asset(AssetDescription::Arc(ArcAsset {
                    mode: ArcMode::FilledArc,
                }), None),
                inverted_arc_asset: asset_manager.create_asset(AssetDescription::Arc(ArcAsset {
                    mode: ArcMode::InvertedFilledArc,
                }), None),
            })),
        ],
    };

    let mut context = Context {
        asset_manager: asset_manager,
        render_target_size: Size2D::new(800, 600),
    };
    context.asset_manager.start_rasterizing_assets_in_display_list_as_necessary(&mut display_list);

    let mut draw_context;
    let batches;
    {
        let _timer = Timer::new("building batches");
        draw_context = DrawContext::new(atlas);
        let mut batcher = Batcher::new();
        for mut item in display_list.items.into_iter() {
            batcher.add(&mut context, &mut item)
        }
        batches = batcher.finish();
    }

    {
        let _timer = Timer::new("initializing GL state and clearing");
        draw_context.init_gl_state();
        draw_context.clear();
    }

    {
        let _timer = Timer::new("drawing batches");
        for batch in batches.into_iter() {
            draw_context.draw_batch(&batch)
        }
    }

    {
        let _timer = Timer::new("finishing rasterization");
        draw_context.finish();
    }

    window.swap_buffers();

    loop {
        window.wait_events().next();
    }
}

