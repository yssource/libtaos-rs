use chrono::NaiveDateTime;
use num_enum::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

use std::{
    fmt,
    fmt::Display,
    str::FromStr,
    time::{self, SystemTime},
};

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, Eq, PartialEq, FromPrimitive)]
#[repr(i32)]
pub enum TimestampPrecision {
    Milli = 0,
    Micro = 1,
    Nano = 2,
    #[num_enum(default)]
    Unknown = -1,
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Timestamp {
    pub(crate) timestamp: i64,
    pub(crate) precision: TimestampPrecision,
}

impl Timestamp {
    pub fn new(timestamp: i64, precision: impl Into<TimestampPrecision>) -> Self {
        Self {
            timestamp,
            precision: precision.into(),
        }
    }
    pub fn now() -> Self {
        let timestamp = chrono::Local::now().timestamp_millis();
        Self {
            timestamp,
            precision: TimestampPrecision::Milli,
        }
    }
    pub fn as_raw_timestamp(&self) -> i64 {
        self.timestamp
    }
    pub fn to_std_time(&self) -> SystemTime {
        let duration = match self.precision {
            TimestampPrecision::Nano => time::Duration::from_nanos(self.timestamp.abs() as _),
            TimestampPrecision::Micro => time::Duration::from_micros(self.timestamp.abs() as _),
            TimestampPrecision::Milli => time::Duration::from_millis(self.timestamp.abs() as _),
            _ => unreachable!("not a valid precision"),
        };
        match self.timestamp {
            ts if ts > 0 => SystemTime::UNIX_EPOCH.checked_add(duration).unwrap(),
            0 => SystemTime::UNIX_EPOCH,
            _ => SystemTime::UNIX_EPOCH.checked_sub(duration).unwrap()
        }
    }

    pub fn to_naive_datetime(&self) -> NaiveDateTime {
        let duration = match self.precision {
            TimestampPrecision::Nano => chrono::Duration::nanoseconds(self.timestamp),
            TimestampPrecision::Micro => chrono::Duration::microseconds(self.timestamp),
            TimestampPrecision::Milli => chrono::Duration::milliseconds(self.timestamp),
            _ => unreachable!("not a valid precision"),
        };
        NaiveDateTime::from_timestamp(0, 0)
            .checked_add_signed(duration)
            .unwrap()
    }
    // pub fn to_string(&self) -> String {
    //     let format = match self.precision {
    //         TimestampPrecision::Nano => "%Y-%m-%d %H:%M:%S%.9f",
    //         TimestampPrecision::Micro => "%Y-%m-%d %H:%M:%S%.6f",
    //         TimestampPrecision::Milli => "%Y-%m-%d %H:%M:%S%.3f",
    //         _ => unreachable!("not a valid precision"),
    //     };
    //     self.to_naive_datetime().format(format).to_string()
    // }
}

impl FromStr for Timestamp {
    type Err = chrono::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ts = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")?;
        Ok(Timestamp {
            timestamp: ts.timestamp_millis(),
            precision: TimestampPrecision::Milli,
        })
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let format = match self.precision {
            TimestampPrecision::Nano => "%Y-%m-%d %H:%M:%S%.9f",
            TimestampPrecision::Micro => "%Y-%m-%d %H:%M:%S%.6f",
            TimestampPrecision::Milli => "%Y-%m-%d %H:%M:%S%.3f",
            _ => unreachable!("not a valid precision"),
        };
        write!(f, "{}", self.to_naive_datetime().format(format).to_string())
    }
}
