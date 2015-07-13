/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![feature(main)]

extern crate euclid;
extern crate freetype;
extern crate gleam;
extern crate num_cpus;

#[macro_use]
extern crate log;

pub mod assets;
pub mod atlas;
pub mod batch;
pub mod context;
pub mod demo;
pub mod distance_field;
pub mod display_list;
pub mod draw;
pub mod job_server;

mod blur;

