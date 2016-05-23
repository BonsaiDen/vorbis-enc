// Copyright (c) 2016 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std;
use std::ptr;
use std::mem;
use std::fs::File;
use std::io::Write;
use std::io::Error;
use std::ffi::CString;


// External Dependencies ------------------------------------------------------
use rand;
use rand::Rng;


// Vorbis Dependencies --------------------------------------------------------
use vorbis_sys::{
    vorbis_info,
    vorbis_comment,
    vorbis_dsp_state,
    vorbis_block ,

    vorbis_info_init,
    vorbis_info_clear,
    vorbis_comment_init,
    vorbis_comment_add_tag,
    vorbis_comment_clear,
    vorbis_analysis_init,
    vorbis_analysis,
    vorbis_bitrate_addblock,
    vorbis_bitrate_flushpacket,
    vorbis_analysis_headerout,
    vorbis_analysis_blockout,
    vorbis_analysis_wrote,
    vorbis_analysis_buffer,
    vorbis_dsp_clear,
    vorbis_block_init,
    vorbis_block_clear
};

use vorbisenc_sys::{
    vorbis_encode_init,
    vorbis_encode_init_vbr
};


// Ogg Dependencies -----------------------------------------------------------
use ogg_sys::{
    ogg_stream_state,
    ogg_page,
    ogg_packet,

    ogg_stream_init,
    ogg_stream_clear,
    ogg_stream_packetin,
    ogg_stream_flush,
    ogg_stream_pageout,
    ogg_page_eos
};


// Internal Types -------------------------------------------------------------
#[derive(PartialEq)]
enum EncoderState {
    Created,
    Initialized,
    Closed
}


// Simple Ogg Vorbis Encoder Implementation -----------------------------------

/// Implementation of a file based ogg-vorbis audio encoder.
pub struct OggVorbisEncoder {
    file: File,
    ogg: Box<OggState>,
    vorbis: Box<VorbisState>,
    state: EncoderState,
    file_size: usize
}

impl OggVorbisEncoder {

    /// Creates a audio stream with the specified output file.
    pub fn new(filename: &str) -> Result<OggVorbisEncoder, Error> {
        match File::create(filename) {
            Ok(file) => Ok(OggVorbisEncoder {
                file: file,
                ogg: Box::new(OggState::new()),
                vorbis: Box::new(VorbisState::new()),
                state: EncoderState::Created,
                file_size: 0
            }),
            Err(e) => Err(e)
        }
    }

    /// Initializes the audio stream for encoding with a pre-defined bitrate
    /// configuration.
    pub fn initialize(
        &mut self,
        channels: usize,
        sample_rate: u32,
        nominal_bitrate: u32,
        min_bitrate: Option<u32>,
        max_bitrate: Option<u32>

    ) -> Result<(), String> {
        match self.state {
            EncoderState::Created => {

                self.vorbis.init(
                    channels,
                    sample_rate as i64,
                    max_bitrate.map_or(-1, |b| b as i64),
                    nominal_bitrate as i64,
                    min_bitrate.map_or(-1, |b| b as i64)
                );
                self.ogg.init(&mut self.vorbis);
                self.ogg.write_flush(&mut self.file);

                self.state = EncoderState::Initialized;

                Ok(())

            },
            EncoderState::Initialized => {
                Err("Audio stream already initialized.".to_string())
            },
            EncoderState::Closed => {
                Err("Audio stream already closed.".to_string())
            }
        }
    }

    /// Initializes the audio stream with variable bitrate encoding.
    pub fn initialize_with_vbr(
        &mut self,
        channels: usize,
        sample_rate: u32,
        quality: f32

    ) -> Result<(), String> {
        match self.state {
            EncoderState::Created => {

                self.vorbis.init_vbr(channels, sample_rate as i64, quality);
                self.ogg.init(&mut self.vorbis);
                self.file_size += self.ogg.write_flush(&mut self.file);

                self.state = EncoderState::Initialized;

                Ok(())

            },
            EncoderState::Initialized => {
                Err("Audio stream already initialized.".to_string())
            },
            EncoderState::Closed => {
                Err("Audio stream already closed.".to_string())
            }
        }
    }

    /// Writes the `samples` into the audio stream.
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), String> {
        match self.state {
            EncoderState::Created => {
                Err("Audio stream not initialized.".to_string())
            },
            EncoderState::Initialized => {
                self.vorbis.write_samples(samples);
                self.file_size += self.ogg.write(&mut self.file, &mut self.vorbis);
                Ok(())
            },
            EncoderState::Closed => {
                Err("Audio stream already closed.".to_string())
            }
        }
    }

    /// Closes the audio stream.
    pub fn close(&mut self) -> Result<(), String> {
        match self.state {
            EncoderState::Created => {
                Err("Audio stream not initialized.".to_string())
            },
            EncoderState::Initialized => {
                self.vorbis.close();
                self.file_size += self.ogg.write(&mut self.file, &mut self.vorbis);
                self.state == EncoderState::Closed;
                Ok(())
            },
            EncoderState::Closed => {
                Err("Audio stream already closed.".to_string())
            }
        }
    }

    /// Returns the number of bytes written into the output file.
    pub fn len(&self) -> usize {
        self.file_size
    }

}

impl Drop for OggVorbisEncoder {
    fn drop(&mut self) {
        self.ogg.destroy();
        self.vorbis.destroy();
    }
}


// Internal Vorbis Encoding State ---------------------------------------------
#[repr(C)]
struct VorbisState {
    vi: vorbis_info,
    vc: vorbis_comment,
    vd: vorbis_dsp_state,
    vb: vorbis_block,
    channels: usize
}

impl VorbisState {

    fn new() -> VorbisState {
        VorbisState {
            vi: unsafe { mem::zeroed() },
            vc: unsafe { mem::zeroed() },
            vd: unsafe { mem::zeroed() },
            vb: unsafe { mem::zeroed() },
            channels: 0
        }
    }

    fn init(&mut self, channels: usize, sample_rate: i64, max_bitrate: i64, nominal_bitrate: i64, min_bitrate: i64) {
        self.pre_init(channels);
        unsafe {
            vorbis_encode_init(&mut self.vi, channels as i64, sample_rate, max_bitrate, nominal_bitrate, min_bitrate);
        }
        self.post_init();
    }

    fn init_vbr(&mut self, channels: usize, sample_rate: i64, quality: f32) {
        self.pre_init(channels);
        unsafe {
            vorbis_encode_init_vbr(&mut self.vi, channels as i64, sample_rate, quality);
        }
        self.post_init();
    }

    fn write_samples(&mut self, samples: &[i16]) {

        let len = samples.len();
        let channel_buffers = unsafe {
            std::slice::from_raw_parts(
                vorbis_analysis_buffer(&mut self.vd, len as i32),
                self.channels
            )
        };

        if self.channels == 1 {

            let mono_ptr: *mut f32 = channel_buffers[0];
            let mono: &mut [f32] = unsafe {
                std::slice::from_raw_parts_mut(mono_ptr, len)
            };

            for i in 0..len {
                mono[i] = samples[i] as f32 / 32768.0;
            }

        } else if self.channels == 2 {

            let left_ptr: *mut f32 = channel_buffers[0];
            let left: &mut [f32] = unsafe {
                std::slice::from_raw_parts_mut(left_ptr, len)
            };

            let right_ptr: *mut f32 = channel_buffers[1];
            let right: &mut [f32] = unsafe {
                std::slice::from_raw_parts_mut(right_ptr, len)
            };

            for i in 0..len / 2 {
                left[i] = samples[i * 2] as f32 / 32768.0;
                right[i] = samples[i * 2 + 1] as f32 / 32768.0;
            }

        }

        unsafe {
            vorbis_analysis_wrote(&mut self.vd, (len / self.channels) as i32);
        }

    }

    fn close(&mut self) {
        unsafe {
            vorbis_analysis_wrote(&mut self.vd, 0);
        }
    }

    fn pre_init(&mut self, channels: usize) {
        self.channels = channels;
        unsafe {
            vorbis_info_init(&mut self.vi);
            vorbis_comment_init(&mut self.vc);
            vorbis_comment_add_tag(
                &mut self.vc,
                CString::new("ENCODER").unwrap().as_ptr(),
                CString::new("vorbis_enc.rs").unwrap().as_ptr()
            );
        }
    }

    fn post_init(&mut self) {
        unsafe {
            vorbis_analysis_init(&mut self.vd, &mut self.vi);
            vorbis_block_init(&mut self.vd, &mut self.vb);
        }
    }

    fn destroy(&mut self) {
        unsafe {
            vorbis_block_clear(&mut self.vb);
            vorbis_dsp_clear(&mut self.vd);
            vorbis_comment_clear(&mut self.vc);
            vorbis_info_clear(&mut self.vi);
        }
    }

}


// Internal Ogg Container State -----------------------------------------------
#[repr(C)]
struct OggState {
    os: ogg_stream_state,
    og: ogg_page,
    op: ogg_packet
}

impl OggState {

    fn new() -> OggState {
        OggState {
            os: unsafe { mem::zeroed() },
            og: unsafe { mem::zeroed() },
            op: unsafe { mem::zeroed() }
        }
    }

    fn init(&mut self, vorbis: &mut VorbisState) {

        let serial_no = rand::thread_rng().next_u32();
        unsafe {
            ogg_stream_init(&mut self.os, serial_no as i32);
        }

        let mut header: ogg_packet = unsafe { mem::zeroed() };
        let mut header_comm: ogg_packet = unsafe { mem::zeroed() };
        let mut header_code: ogg_packet = unsafe { mem::zeroed() };

        unsafe {
            vorbis_analysis_headerout(
                &mut vorbis.vd,
                &mut vorbis.vc,
                &mut header,
                &mut header_comm,
                &mut header_code
            );
            ogg_stream_packetin(&mut self.os, &mut header);
            ogg_stream_packetin(&mut self.os, &mut header_comm);
            ogg_stream_packetin(&mut self.os, &mut header_code);
        }

    }

    fn write(&mut self, file: &mut File, vorbis: &mut VorbisState) -> usize {

        let null = ptr::null_mut();
        let mut bytes_written = 0;
        while unsafe { vorbis_analysis_blockout(&mut vorbis.vd, &mut vorbis.vb) } == 1 {

            unsafe {
                // Assume we want to use bitrate management
                vorbis_analysis(&mut vorbis.vb, null);
                vorbis_bitrate_addblock(&mut vorbis.vb);
            }

            while unsafe { vorbis_bitrate_flushpacket(&mut vorbis.vd, &mut self.op) } != 0 {

                unsafe {
                    ogg_stream_packetin(&mut self.os, &mut self.op);
                }

                bytes_written += self.write_page(file);

            }

        }

        bytes_written

    }

    fn write_page(&mut self, file: &mut File) -> usize {

        let mut bytes_written = 0;

        loop {

            let result = unsafe {
                ogg_stream_pageout(&mut self.os, &mut self.og)
            };

            if result == 0 {
                break;

            } else {
                let header: &[u8] = unsafe { std::slice::from_raw_parts(self.og.header, self.og.header_len as usize) };
                let body: &[u8] = unsafe { std::slice::from_raw_parts(self.og.body, self.og.body_len as usize) };
                file.write_all(header).ok();
                file.write_all(body).ok();
                bytes_written += self.og.header_len as usize;
                bytes_written += self.og.body_len as usize;

                if unsafe { ogg_page_eos(&mut self.og) } != 0 {
                    break;
                }

            }

        }

        bytes_written

    }

    fn write_flush(&mut self, file: &mut File) -> usize {

        let mut bytes_written = 0;
        loop {

            let result = unsafe {
                ogg_stream_flush(&mut self.os, &mut self.og)
            };

            if result == 0 {
                break;

            } else {
                let header: &[u8] = unsafe { std::slice::from_raw_parts(self.og.header, self.og.header_len as usize) };
                let body: &[u8] = unsafe { std::slice::from_raw_parts(self.og.body, self.og.body_len as usize) };
                file.write_all(header).ok();
                file.write_all(body).ok();
                bytes_written += self.og.header_len as usize;
                bytes_written += self.og.body_len as usize;
            }
        }

        bytes_written

    }

    fn destroy(&mut self) {
        unsafe {
            ogg_stream_clear(&mut self.os);
        }
    }

}

