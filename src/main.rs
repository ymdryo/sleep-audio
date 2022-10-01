use std::io::{stdout, Write};

use anyhow::Result;
use portaudio as pa;
use rand::{prelude::StdRng, Rng, SeedableRng};
use rand_distr::StandardNormal;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

fn main() -> Result<()> {
   let pa = pa::PortAudio::new()?;

   let mut settings =
      pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
   // we won't output out of range samples so don't bother clipping them.
   settings.flags = pa::stream_flags::CLIP_OFF;

   let mut left = 0.;
   let mut right = 0.;

   let mut rng = StdRng::from_entropy();

   let mut sample = move |x: &mut f32| {
      *x = *x * (1. - 128. / SAMPLE_RATE as f32) + rng.sample::<f32, _>(StandardNormal) * 0.025;
      *x = x.min(1.).max(-1.);
   };

   // This routine will be called by the PortAudio engine when audio is needed. It may called at
   // interrupt level on some machines so don't do anything that could mess up the system like
   // dynamic resource allocation or IO.
   let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
      let mut idx = 0;
      let mut max: f32 = 0.;
      for _ in 0..frames {
         max = max.max(left).max(right);

         sample(&mut left);
         sample(&mut right);

         buffer[idx] = left;
         buffer[idx + 1] = right;
         idx += 2;
      }
      if max.abs() == 1. && false {
         print!(".");
         stdout().flush().unwrap();
      }
      pa::Continue
   };

   let mut stream = pa.open_non_blocking_stream(settings, callback)?;

   stream.start()?;

   // pa.sleep(NUM_SECONDS * 1_000);
   loop {
      std::thread::yield_now();
   }

   stream.stop()?;
   stream.close()?;

   Ok(())
}
