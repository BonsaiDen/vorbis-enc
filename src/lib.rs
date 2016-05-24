// Copyright (c) 2016 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Crates ---------------------------------------------------------------------
extern crate vorbis_sys;
extern crate vorbisfile_sys;
extern crate vorbisenc_sys;
extern crate ogg_sys;
extern crate rand;
extern crate libc;


// Exports --------------------------------------------------------------------
mod encoder;
pub use encoder::OggVorbisEncoder;

