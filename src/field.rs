use chrono::NaiveDateTime;

use itertools::Itertools;
use num_enum::FromPrimitive;
use serde::Deserialize;
use std::{
    fmt,
    fmt::Display,
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
        if self.timestamp > 0 {
            SystemTime::UNIX_EPOCH.checked_add(duration).unwrap()
        } else if self.timestamp == 0 {
            SystemTime::UNIX_EPOCH
        } else {
            SystemTime::UNIX_EPOCH.checked_sub(duration).unwrap()
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
    pub fn to_string(&self) -> String {
        let format = match self.precision {
            TimestampPrecision::Nano => "%Y-%m-%d %H:%M:%S%.9f",
            TimestampPrecision::Micro => "%Y-%m-%d %H:%M:%S%.6f",
            TimestampPrecision::Milli => "%Y-%m-%d %H:%M:%S%.3f",
            _ => unreachable!("not a valid precision"),
        };
        self.to_naive_datetime().format(format).to_string()
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub type_: TaosDataType,
    pub bytes: i16,
}
#[derive(Debug)]
pub struct TaosQueryData {
    pub column_meta: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Field>>,
}

#[derive(Debug)]
pub struct TaosDescribe(TaosQueryData);

impl TaosDescribe {
    pub fn names(&self) -> Vec<String> {
        self.0
            .rows
            .iter()
            .map(|row| {
                row.first()
                    .expect("first column must exists in describe")
                    .to_string()
            })
            .collect_vec()
    }
}

impl From<TaosQueryData> for TaosDescribe {
    fn from(rhs: TaosQueryData) -> Self {
        Self(rhs)
    }
}
impl TaosQueryData {
    /// Total rows count of query result
    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum TaosDataType {
    Null = 0,
    Bool,      // 1
    TinyInt,   // 2
    SmallInt,  // 3
    Int,       // 4
    BigInt,    // 5
    Float,     // 6
    Double,    // 7
    Binary,    // 8
    Timestamp, // 9
    NChar,     // 10
    UTinyInt,  // 11
    USmallInt, // 12
    UInt,      // 13
    UBigInt,   // 14
    #[num_enum(default)]
    NonZero = 255,
}

#[derive(Debug)]
pub enum Field {
    Null,        // 0
    Bool(bool),  // 1
    TinyInt(i8), // 2
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Binary(String),
    Timestamp(Timestamp),
    NChar(String),
    UTinyInt(u8),
    USmallInt(u16),
    UInt(u32),
    UBigInt(u64), // 14
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Field::Null => write!(f, "NULL"),
            Field::Bool(v) => write!(f, "{}", v),
            Field::TinyInt(v) => write!(f, "{}", v),
            Field::SmallInt(v) => write!(f, "{}", v),
            Field::Int(v) => write!(f, "{}", v),
            Field::BigInt(v) => write!(f, "{}", v),
            Field::Float(v) => write!(f, "{}", v),
            Field::Double(v) => write!(f, "{}", v),
            Field::Binary(v) | Field::NChar(v) => write!(f, "{}", v),
            Field::Timestamp(v) => write!(f, "{}", v),
            Field::UTinyInt(v) => write!(f, "{}", v),
            Field::USmallInt(v) => write!(f, "{}", v),
            Field::UInt(v) => write!(f, "{}", v),
            Field::UBigInt(v) => write!(f, "{}", v),
        }
    }
}

impl Field {
    pub fn as_bool(&self) -> Option<&bool> {
        match self {
            Field::Bool(v) => Some(v),
            _ => None
        }
    }
    pub fn as_tiny_int(&self) -> Option<&i8> {
        match self {
            Field::TinyInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_small_int(&self) -> Option<&i16> {
        match self {
            Field::SmallInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_int(&self) -> Option<&i32> {
        match self {
            Field::Int(v) => Some(v),
            _ => None
        }
    }
    pub fn as_big_int(&self) -> Option<&i64> {
        match self {
            Field::BigInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_float(&self) -> Option<&f32> {
        match self {
            Field::Float(v) => Some(v),
            _ => None
        }
    }
    pub fn as_double(&self) -> Option<&f64> {
        match self {
            Field::Double(v) => Some(v),
            _ => None
        }
    }
    pub fn as_binary(&self) -> Option<&str> {
        match self {
            Field::Binary(v)=> Some(v),
            _ => None
        }
    }
    pub fn as_nchar(&self) -> Option<&str> {
        match self {
            Field::NChar(v)=> Some(v),
            _ => None
        }
    }

    /// BINARY or NCHAR typed string reference
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Field::Binary(v) | Field::NChar(v)=> Some(v),
            _ => None
        }
    }
    pub fn as_timestamp(&self) -> Option<&Timestamp> {
        match self {
            Field::Timestamp(v) => Some(v),
            _ => None
        }
    }
    pub fn as_raw_timestamp(&self) -> Option<i64> {
        match self {
            Field::Timestamp(v) => Some(v.as_raw_timestamp()),
            _ => None
        }
    }
    pub fn as_unsigned_tiny_int(&self) -> Option<&u8> {
        match self {
            Field::UTinyInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_unsigned_samll_int(&self) -> Option<&u16> {
        match self {
            Field::USmallInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_unsigned_int(&self) -> Option<&u32> {
        match self {
            Field::UInt(v) => Some(v),
            _ => None
        }
    }
    pub fn as_unsigned_big_int(&self) -> Option<&u64> {
        match self {
            Field::UBigInt(v) => Some(v),
            _ => None
        }
    }
}