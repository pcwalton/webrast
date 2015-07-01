/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use euclid::Size2D;

use assets::AssetManager;

pub struct Context {
    /// The asset manager.
    pub asset_manager: AssetManager,
    /// The size of the render target in pixels.
    pub render_target_size: Size2D<i32>,
}

