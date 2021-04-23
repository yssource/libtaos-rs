use itertools::Itertools;
use num_enum::FromPrimitive;
use serde::Deserialize;
use std::fmt;

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
    Timestamp(i64),
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
