use crate::bindings::*;
use crate::*;

use bstr::{BStr, BString};
use chrono::{DateTime, NaiveDateTime};
use paste::paste;

use std::os::raw::{c_int, c_void};
use std::ptr;
use std::{time::SystemTime};

pub type BindParam = TAOS_BIND;

pub trait IntoBindParam {
    fn into_bind_param(self) -> BindParam;
}

impl BindParam {
    pub fn new(buffer_type: TaosDataType) -> Self {
        let buffer: *mut c_void = ptr::null_mut();
        let length: *mut usize = ptr::null_mut();
        let is_null: *mut c_int = ptr::null_mut();
        let error: *mut c_int = ptr::null_mut();
        TAOS_BIND {
            buffer_type: buffer_type as _,
            buffer,
            buffer_length: 0,
            length,
            is_null,
            is_unsigned: 0,
            error,
            allocated: 1,
            u: TAOS_BIND__bindgen_ty_1 { ts: 0 },
        }
    }
    pub fn null() -> Self {
        let mut null = Self::new(TaosDataType::Null);
        let v = Box::new(1i8);
        null.is_null = Box::into_raw(v) as _;
        null
    }
    pub(crate) unsafe fn free(&mut self) {
        if !self.buffer.is_null() {
            Box::from_raw(self.buffer);
        }
        if !self.length.is_null() {
            Box::from_raw(self.length);
        }
        if !self.is_null.is_null() {
            Box::from_raw(self.is_null);
        }
        if !self.error.is_null() {
            Box::from_raw(self.error);
        }
    }
}

macro_rules! _impl_primitive_into_bind_param {
    ($ty:ty, $target:ident, $v:expr) => {
        impl IntoBindParam for $ty {
            fn into_bind_param(self) -> BindParam {
                let mut param = BindParam::new(TaosDataType::$target);
                param.buffer_length = std::mem::size_of::<$ty>();
                let v = Box::new(self);
                param.buffer = Box::into_raw(v) as _;
                let l = Box::new(param.buffer_length);
                param.length = Box::into_raw(l) as _;
                param
            }
        }
        impl IntoBindParam for &$ty {
            fn into_bind_param(self) -> BindParam {
                (*self).into_bind_param()
            }
        }
        impl IntoBindParam for &&$ty {
            fn into_bind_param(self) -> BindParam {
                (*self).into_bind_param()
            }
        }
        paste! {
        #[test]
        fn [<test_ $ty:snake>]() {
            let v: $ty = $v;
            let p = v.into_bind_param();
            let v2 = unsafe { *(p.buffer as *const $ty) };
            assert!(v == v2);
        }
        }
    };
}
macro_rules! _impl_ref_into_bind_param {
    ($ty:ty) => {
        impl IntoBindParam for $ty {
            fn into_bind_param(self) -> BindParam {
                (&self).into_bind_param()
            }
        }
        impl IntoBindParam for &&$ty {
            fn into_bind_param(self) -> BindParam {
                (*self).into_bind_param()
            }
        }
    };
}

impl<T: IntoBindParam> IntoBindParam for Option<T> {
    fn into_bind_param(self) -> BindParam {
        match self {
            None => BindParam::null(),
            Some(v) => v.into_bind_param(),
        }
    }
}

impl<T: IntoBindParam, E: std::error::Error> IntoBindParam for Result<T, E> {
    fn into_bind_param(self) -> BindParam {
        match self {
            Err(_) => BindParam::null(),
            Ok(v) => v.into_bind_param(),
        }
    }
}

impl IntoBindParam for bool {
    fn into_bind_param(self) -> BindParam {
        let v: i8 = if self { 1 } else { 0 };
        let mut param = BindParam::new(TaosDataType::Bool);
        param.buffer_length = std::mem::size_of::<i8>();
        let v = Box::new(v);
        param.buffer = Box::into_raw(v) as _;
        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
impl IntoBindParam for &bool {
    fn into_bind_param(self) -> BindParam {
        (*self).into_bind_param()
    }
}
impl IntoBindParam for &&bool {
    fn into_bind_param(self) -> BindParam {
        (*self).into_bind_param()
    }
}

_impl_primitive_into_bind_param!(i8, TinyInt, 0);
_impl_primitive_into_bind_param!(i16, SmallInt, 0);
_impl_primitive_into_bind_param!(i32, Int, 0);
_impl_primitive_into_bind_param!(i64, BigInt, 0);
_impl_primitive_into_bind_param!(u8, UTinyInt, 0);
_impl_primitive_into_bind_param!(u16, USmallInt, 0);
_impl_primitive_into_bind_param!(u32, UInt, 0);
_impl_primitive_into_bind_param!(u64, UBigInt, 0);
_impl_primitive_into_bind_param!(f32, Float, 0.);
_impl_primitive_into_bind_param!(f64, Double, 0.);

impl IntoBindParam for &BStr {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::Binary);
        param.buffer_length = self.len();

        let cstr = self.to_c_string();
        param.buffer = cstr.as_ptr() as _;
        std::mem::forget(cstr);

        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}

impl IntoBindParam for &BString {
    fn into_bind_param(self) -> BindParam {
        let bstr: &BStr = self.as_ref();
        bstr.into_bind_param()
    }
}
_impl_ref_into_bind_param!(BString);

impl IntoBindParam for &str {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::NChar);
        param.buffer_length = self.len();

        let cstr = self.to_c_string();
        param.buffer = cstr.as_ptr() as _;
        std::mem::forget(cstr);

        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
impl IntoBindParam for &String {
    fn into_bind_param(self) -> BindParam {
        self.as_str().into_bind_param()
    }
}
_impl_ref_into_bind_param!(String);

impl IntoBindParam for &SystemTime {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::Timestamp);
        param.buffer_length = std::mem::size_of::<i64>();
        let duration = self
            .duration_since(std::time::UNIX_EPOCH)
            .expect("systemtime before unix epoch is not invalid");
        // FIXME(@huolinhe): an global flag for precision should be setted.
        let v = Box::new(duration.as_millis());
        param.buffer = Box::into_raw(v) as _;
        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
_impl_ref_into_bind_param!(SystemTime);

impl IntoBindParam for &NaiveDateTime {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::Timestamp);
        param.buffer_length = std::mem::size_of::<i64>();
        let timestamp = self.timestamp_millis();
        // FIXME(@huolinhe): an global flag for precision should be setted.
        let v = Box::new(timestamp);
        param.buffer = Box::into_raw(v) as _;
        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
_impl_ref_into_bind_param!(NaiveDateTime);

impl<Tz: chrono::TimeZone> IntoBindParam for &DateTime<Tz> {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::Timestamp);
        param.buffer_length = std::mem::size_of::<i64>();
        let timestamp = self.timestamp_millis();
        // FIXME(@huolinhe): an global flag for precision should be setted.
        let v = Box::new(timestamp);
        param.buffer = Box::into_raw(v) as _;
        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
impl<Tz: chrono::TimeZone> IntoBindParam for DateTime<Tz> {
    fn into_bind_param(self) -> BindParam {
        (&self).into_bind_param()
    }
}
impl IntoBindParam for &Timestamp {
    fn into_bind_param(self) -> BindParam {
        let mut param = BindParam::new(TaosDataType::Timestamp);
        param.buffer_length = std::mem::size_of::<i64>();
        let v = Box::new(self.timestamp);
        param.buffer = Box::into_raw(v) as _;
        let l = Box::new(param.buffer_length);
        param.length = Box::into_raw(l) as _;
        param
    }
}
_impl_ref_into_bind_param!(Timestamp);

impl IntoBindParam for &Field {
    fn into_bind_param(self) -> BindParam {
        match self {
            Field::Null => BindParam::null(),
            Field::Timestamp(v) => v.into_bind_param(),
            Field::Bool(v) => v.into_bind_param(),
            Field::TinyInt(v) => v.into_bind_param(),
            Field::SmallInt(v) => v.into_bind_param(),
            Field::Int(v) => v.into_bind_param(),
            Field::BigInt(v) => v.into_bind_param(),
            Field::UTinyInt(v) => v.into_bind_param(),
            Field::USmallInt(v) => v.into_bind_param(),
            Field::UInt(v) => v.into_bind_param(),
            Field::UBigInt(v) => v.into_bind_param(),
            Field::Float(v) => v.into_bind_param(),
            Field::Double(v) => v.into_bind_param(),
            Field::Binary(v) => v.into_bind_param(),
            Field::NChar(v) => v.into_bind_param(),
        }
    }
}
_impl_ref_into_bind_param!(Field);
