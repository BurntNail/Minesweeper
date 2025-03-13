use std::ops::Add;
use std::ops::Div;
use std::time::{Duration, Instant};

pub trait Sampler: Default {
    type Output;
    fn start(&mut self);
    fn stop(&mut self) -> Option<Self::Output>;
}

pub struct SampleHolder<const N: usize, S: Sampler> {
    sampler: S,
    samples: [S::Output; N],
    next_index: usize,
    are_all_valid: bool,
}

impl<const N: usize, S: Sampler> SampleHolder<N, S>
where S::Output: Default + Copy
{
    //TODO: make a version that doesn't require copy?
    pub fn new() -> Self {
        Self {
            sampler: S::default(),
            samples: [S::Output::default(); N],
            next_index: 0,
            are_all_valid: false,
        }
    }

    pub fn start(&mut self) {
        self.sampler.start();
    }

    pub fn stop(&mut self) {
        if let Some(sample) = self.sampler.stop() {
            self.samples[self.next_index] = sample;

            if self.next_index == N - 1 {
                self.are_all_valid = true;
                self.next_index = 0;
            } else {
                self.next_index += 1;
            }
        }
    }
}

impl<const N: usize, S: Sampler> SampleHolder<N, S>
where S::Output: Add<Output = S::Output> + Div<u32, Output = S::Output> + Default + Copy {
    #[allow(dead_code)]
    pub fn get_average(&self) -> Option<S::Output> {
        if self.next_index == 0 && !self.are_all_valid {
            return None;
        }

        let mut sum = S::Output::default();

        let end = if self.are_all_valid {
            N - 1
        } else {
            self.next_index
        };

        for sample in &self.samples[0..end] {
            sum = sum + *sample;
        }

        Some(sum / (end as u32))
    }
}

impl<const N: usize, S: Sampler> SampleHolder<N, S>
where S::Output: Ord {
    #[allow(dead_code)]
    pub fn get_max (&self) -> Option<&S::Output> {
        self.samples.iter().max()
    }
}

#[derive(Default)]
pub struct InstantSampler(Option<Instant>);

impl Sampler for InstantSampler {
    type Output = Duration;

    fn start(&mut self) {
        self.0 = Some(Instant::now());
    }

    fn stop(&mut self) -> Option<Self::Output> {
        self.0.take().map(|i| i.elapsed())
    }
}