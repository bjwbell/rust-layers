// Copyright 2013 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Miscellaneous utilities.
/// A request from the compositor to the renderer for tiles that need to be (re)displayed.
use geom::rect::Rect;

#[deriving(Clone)]
pub struct BufferRequest {
    // The rect in pixels that will be drawn to the screen
    screen_rect: Rect<uint>,

    // The rect in page coordinates that this tile represents
    page_rect: Rect<f32>,
}

pub fn BufferRequest(screen_rect: Rect<uint>, page_rect: Rect<f32>) -> BufferRequest {
    BufferRequest {
        screen_rect: screen_rect,
        page_rect: page_rect,
    }
}


pub fn convert_rgb32_to_rgb24(buffer: &[u8]) -> Vec<u8> {
    let mut i = 0;
    Vec::from_fn(buffer.len() * 3 / 4, |j| {
        match j % 3 {
            0 => {
                buffer[i + 2]
            }
            1 => {
                buffer[i + 1]
            }
            2 => {
                let val = buffer[i];
                i += 4;
                val
            }
            _ => {
                fail!()
            }
        }
    })
}

