use crate::bindings::*;
use crate::{TaosCode, TaosError};

use log::trace;
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

pub trait TaosErrorOr: Sized {
    fn taos_error_or(self) -> Result<Self, TaosError>;
}

macro_rules! impl_taos_error_or {
    ($ty:ty ) => {
        impl TaosErrorOr for $ty {
            fn taos_error_or(self) -> Result<Self, TaosError> {
                unsafe {
                    let errno = taos_errno(self as _);
                    trace!("error code: {:#0x}", errno & 0x0000ffff);
                    let code: TaosCode = (errno & 0x0000ffff).into();
                    if !code.success() {
                        let err = CStr::from_ptr(taos_errstr(self as _) as *const c_char)
                            .to_string_lossy()
                            .into_owned();
                        // here, it could also be &'static str, but we use string instead.
                        return Err(TaosError {
                            code,
                            err: Cow::from(err),
                        });
                    } else {
                        Ok(self)
                    }
                }
            }
        }
    };
}
impl_taos_error_or!(*mut c_void);
impl_taos_error_or!(*const c_void);
