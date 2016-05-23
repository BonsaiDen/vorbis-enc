extern crate rand;
extern crate vorbis_enc;

use std::iter;
use rand::Rng;
use vorbis_enc::OggVorbisEncoder;

fn main() {

    let mut rng = rand::thread_rng();

    let mut encoder = OggVorbisEncoder::new("stereo.ogg").unwrap();
    encoder.initialize_with_vbr(2, 48000, 0.2).ok();

    let mut samples: Vec<i16> = iter::repeat(0).take((32559) as usize).collect();
    for i in 0..samples.len() / 2 {
        samples[i * 2] = ((rng.next_f32() - 0.5) * u16::max_value() as f32) as i16;
        samples[i * 2 + 1] = 0;
    }

    let mut packets = 0;
    while packets < 64 {
        encoder.write_samples(&samples).ok();
        packets += 1;
    }

    encoder.close().ok();
    println!("{} bytes of noise written (only left channel).", encoder.len());

}

