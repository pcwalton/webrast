/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![feature(main)]

extern crate geom as euclid;
extern crate gleam;

#[macro_use]
extern crate log;

pub mod batch;
pub mod context;
pub mod demo;
pub mod display_list;
pub mod draw;
