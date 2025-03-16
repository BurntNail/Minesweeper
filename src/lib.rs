use chrono::{DateTime, Local, TimeDelta};

pub mod app;
pub mod board;
pub mod data;
pub mod ser;
pub mod time_sampler;

pub trait ChronoDateTimeExt {
    fn elapsed (self) -> TimeDelta;
}
impl ChronoDateTimeExt for DateTime<Local> {
    fn elapsed(self) -> TimeDelta {
        Local::now() - self
    }
}


pub trait ChronoTimeDeltaExt {
    fn as_secs_f64 (&self) -> f64;
    fn as_nice_time<const DECIMAL_PLACES: u32>(&self) -> String;
}
impl ChronoTimeDeltaExt for TimeDelta {
    fn as_secs_f64(&self) -> f64 {
        (self.num_seconds() as f64) + ((self.subsec_nanos() as f64) / 1_000_000_000.0)
    }
    fn as_nice_time<const DECIMAL_PLACES: u32>(&self) -> String {
        if DECIMAL_PLACES > 9 {
            panic!("Cannot have decimal places > 9 for nanoseconds");
        }

        let mins = self.num_seconds() / 60;
        let secs = self.num_seconds() % 60;

        let decimals = match DECIMAL_PLACES {
            0 | _ if self.subsec_nanos() == 0 => "".to_string(),
            n => format!(".{}", self.subsec_nanos() / 10_i32.pow(9 - n)),
        };

        format!("{mins}'{secs}{decimals}\"")
    }
}