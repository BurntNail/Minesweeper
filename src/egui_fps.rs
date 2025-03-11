use std::time::{Duration, Instant};

pub struct FpsCounter<const N: usize> {
    samples: [Duration; N],
    next_index: usize,
    are_all_valid: bool,
    last_start: Option<Instant>,
}

impl<const N: usize> FpsCounter<N> {
    pub fn new () -> Self {
        Self {
            samples: [Duration::new(0, 0); N],
            next_index: 0,
            are_all_valid: false,
            last_start: None,
        }
    }

    pub fn start_timer (&mut self) {
        self.last_start = Some(Instant::now());
    }

    pub fn stop_timer (&mut self) {
        if let Some(start) = self.last_start.take() {
            self.samples[self.next_index] = start.elapsed();

            if self.next_index == N - 1 {
                self.are_all_valid = true;
                self.next_index = 0;
            } else {
                self.next_index += 1;
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_average (&self) -> Duration {
        if self.next_index == 0 && !self.are_all_valid {
            return Duration::new(0, 0);
        }

        let mut sum_seconds = 0;
        let mut sum_nanos = 0;

        let end = if self.are_all_valid {
            N - 1
        } else {
            self.next_index
        };

        for dur in &self.samples[0..end] {
            const NANOS_PER_SECOND: u32 = 1_000_000_000;

            sum_seconds += dur.as_secs();
            sum_nanos += dur.subsec_nanos();

            if sum_nanos > NANOS_PER_SECOND {
                //nanos per whole second
                let delta = sum_nanos / NANOS_PER_SECOND;
                sum_nanos -= delta * NANOS_PER_SECOND;
                sum_seconds += delta as u64;
            }
        }


        Duration::new(sum_seconds / (end as u64), sum_nanos / (end as u32))
    }

    #[allow(dead_code)]
    pub fn get_max (&self) -> Duration {
        self.samples.iter().max().copied().unwrap_or_default()
    }
}