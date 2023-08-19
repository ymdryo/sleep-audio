use std::io::{stdout, Write};

use anyhow::Result;
use portaudio as pa;
use rand::{prelude::StdRng, Rng, SeedableRng};
use rand_distr::StandardNormal;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = (SAMPLE_RATE * 0.5) as u32;

// const a: (f32, f32, f32, f32) = (1., -2.494956002, 2.017265875, -0.522189400);
// const b: (f32, f32, f32, f32) = (0.049922035, -0.095993537, 0.050612699, -0.004408786);
// const a: (f32, f32, f32, f32) = (-128. / SAMPLE_RATE as f32 + 1., 0., 0., 0.);
// const b: (f32, f32, f32, f32) = (0.025, 0., 0., 0.);

type IirType = f32;

struct IIR<const N: usize, const M: usize> {
   y_buf: [IirType; N],
   x_buf: [IirType; M],
}

struct IirCoeff<const N: usize, const M: usize> {
   a0: IirType,
   a: [IirType; N],
   b: [IirType; M],
}

impl<const N: usize, const M: usize> IIR<N, M> {
   fn new() -> Self {
      Self {
         y_buf: [0.; N],
         x_buf: [0.; M],
      }
   }

   fn next_sample(&self, coeff: &IirCoeff<N, M>) -> IirType {
      (coeff
         .b
         .iter()
         .zip(self.x_buf)
         .map(|(s, t)| s * t)
         .sum::<IirType>()
         - coeff
            .a
            .iter()
            .zip(self.y_buf)
            .map(|(s, t)| s * t)
            .sum::<IirType>())
         / coeff.a0
   }

   fn update(&mut self, coeff: &IirCoeff<N, M>, x: IirType) -> IirType {
      push_iir_buf(&mut self.x_buf, x);
      let y = self.next_sample(coeff);
      push_iir_buf(&mut self.y_buf, y);
      y
   }

   fn current(&self) -> IirType {
      self.y_buf[0]
   }
}

fn push_iir_buf<T: Copy, const N: usize>(buf: &mut [T; N], x: T) {
   for k in (0..(N - 1)).rev() {
      buf[k + 1] = buf[k];
   }
   buf[0] = x;
}

const ID_FILTER: IirCoeff<1, 1> = IirCoeff {
   a0: 1.,
   a: [0.],
   b: [0.025],
};

const BROWN_FILTER: IirCoeff<1, 1> = IirCoeff {
   a0: 1.,
   a: [-(1. - 128. / SAMPLE_RATE as f32)],
   b: [0.025],
};

// source: https://ccrma.stanford.edu/~jos/sasp/Example_Synthesis_1_F_Noise.html
const PINK_FILTER: IirCoeff<3, 4> = IirCoeff {
   a0: 1.,
   a: [-2.494956002, 2.017265875, -0.522189400],
   b: [0.049922035, -0.095993537, 0.050612699, -0.004408786],
};

fn clip(x: f32) -> f32 {
   x.min(1.).max(-1.)
}

fn main() -> Result<()> {
   let pa = pa::PortAudio::new()?;

   let mut settings =
      pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
   // we won't output out of range samples so don't bother clipping them.
   settings.flags = pa::stream_flags::CLIP_OFF;

   let mut left = IIR::new();
   let mut right = IIR::new();

   let mut rng = StdRng::from_entropy();

   let mut normal = move || rng.sample::<f32, _>(StandardNormal);

   // This routine will be called by the PortAudio engine when audio is needed. It may called at
   // interrupt level on some machines so don't do anything that could mess up the system like
   // dynamic resource allocation or IO.
   let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
      let mut idx = 0;
      let mut max: f32 = 0.;
      for _ in 0..frames {
         max = clip(max.max(left.current()).max(right.current()));

         left.update(&PINK_FILTER, normal());
         right.update(&PINK_FILTER, normal());

         // println!("{} {}", left.current(), right.current());

         buffer[idx] = clip(left.current());
         buffer[idx + 1] = clip(right.current());
         idx += 2;
      }
      if max.abs() == 1. {
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
