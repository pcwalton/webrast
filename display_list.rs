/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use assets::Asset;

use euclid::Rect;
use std::cell::RefCell;
use std::rc::Rc;

const AU_PER_PX: i32 = 60;

pub static BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

pub static TRANSPARENT_BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

pub static TRANSPARENT_GREEN: Color = Color {
    r: 0,
    g: 255,
    b: 0,
    a: 0,
};

pub static TRANSPARENT_MAGENTA: Color = Color {
    r: 255,
    g: 0,
    b: 255,
    a: 0,
};

pub static WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

#[derive(Clone)]
pub struct DisplayList {
    pub items: Vec<DisplayItem>,
}

#[derive(Clone)]
pub enum DisplayItem {
    SolidColor(Box<SolidColorDisplayItem>),
    Text(Box<TextDisplayItem>),
}

impl DisplayItem {
    pub fn base(&self) -> &BaseDisplayItem {
        match *self {
            DisplayItem::SolidColor(ref solid_color_display_item) => {
                &solid_color_display_item.base
            }
            DisplayItem::Text(ref text_display_item) => &text_display_item.base,
        }
    }
}

#[derive(Clone)]
pub struct SolidColorDisplayItem {
    pub base: BaseDisplayItem,
    pub color: Color,
}

#[derive(Clone)]
pub struct TextDisplayItem {
    pub base: BaseDisplayItem,
    pub glyph_asset: Rc<RefCell<Asset>>,
    pub blurred_glyph_asset: Option<Rc<RefCell<Asset>>>,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r: r,
            g: g,
            b: b,
            a: a,
        }
    }
}

#[derive(Copy, Clone)]
pub struct BaseDisplayItem {
    pub bounds: Rect<Au>,
    pub clip: ClippingRegion,
}

#[derive(Copy, Clone)]
pub struct ClippingRegion {
    pub main: Rect<Au>,
}

#[derive(Copy, Clone)]
pub struct Au(pub i32);

impl Au {
    #[inline]
    pub fn from_px(pixels: i32) -> Au {
        Au(pixels * AU_PER_PX)
    }

    #[inline]
    pub fn to_px(&self) -> i32 {
        self.0 / AU_PER_PX
    }
}

