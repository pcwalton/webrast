/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use assets::{AssetContext, AssetDescription, AssetRasterization};

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

enum Job {
    RasterizeAsset(AssetDescription, Sender<AssetRasterization>),
    Exit,
}

pub struct JobServer {
    workers: Vec<Sender<Job>>,
    next_worker: u32,
}

impl JobServer {
    pub fn new(worker_count: u32) -> JobServer {
        let mut senders = Vec::new();
        for _ in 0..worker_count {
            let (sender, receiver) = mpsc::channel();
            senders.push(sender);
            thread::spawn(move || worker_main(receiver));
        }
        JobServer {
            workers: senders,
            next_worker: 0,
        }
    }

    pub fn rasterize_asset(&mut self, asset_description: AssetDescription)
                           -> Receiver<AssetRasterization> {
        let (sender, receiver) = mpsc::channel();
        self.workers[self.next_worker as usize].send(Job::RasterizeAsset(asset_description,
                                                                         sender)).unwrap();
        self.next_worker = (self.next_worker + 1) % (self.workers.len() as u32);
        receiver
    }
}

fn worker_main(receiver: Receiver<Job>) {
    let mut asset_context = AssetContext::new();
    loop {
        match receiver.recv().unwrap() {
            Job::Exit => return,
            Job::RasterizeAsset(asset, sender) => {
                sender.send(asset.rasterize(&mut asset_context)).unwrap()
            }
        }
    }
}

