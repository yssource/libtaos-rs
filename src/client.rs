use crate::bindings::*;
use crate::*;

use lazy_static::lazy_static;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use crate::field::*;

/// Make sure to cleanup taos client workspace before exit.
struct CleanUpPhantomData(bool);
impl Drop for CleanUpPhantomData {
    fn drop(&mut self) {
        eprintln!("clean up");
        // unsafe {
        //     taos_cleanup();
        // }
    }
}

lazy_static! {
    static ref TAOS_INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

trait TaosErrorOr: Sized {
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

trait NullOr: Sized {
    fn null_or(self) -> Option<Self>;
}

macro_rules! impl_null_or {
    ($ty:ty ) => {
        impl NullOr for $ty {
            fn null_or(self) -> Option<Self> {
                if self.is_null() {
                    None
                } else {
                    Some(self)
                }
            }
        }
    };
}

impl_null_or!(*const c_void);
impl_null_or!(*mut c_void);
impl_null_or!(*mut *mut c_void);
impl_null_or!(*const *mut c_void);
#[test]
fn test_value_or() {
    let mut a = 0;
    let b = &mut a as *mut i32 as *mut c_void;
    b.null_or();
}

#[derive(Debug)]
pub struct Taos {
    conn: *mut TAOS,
}

unsafe impl Send for Taos {}
unsafe impl Sync for Taos {}

pub trait ToCString {
    fn to_c_string(&self) -> CString;
}

impl ToCString for str {
    fn to_c_string(&self) -> CString {
        CString::new(self).expect("CString::new should not fail here")
    }
}
impl ToCString for &str {
    fn to_c_string(&self) -> CString {
        CString::new(*self).expect("CString::new should not fail here")
    }
}
impl ToCString for &String {
    fn to_c_string(&self) -> CString {
        CString::new(self.as_str()).expect("CString::new should not fail here")
    }
}
impl Taos {
    pub fn new(
        ip: impl ToCString,
        user: impl ToCString,
        pass: impl ToCString,
        db: impl ToCString,
        port: u16,
    ) -> Result<Self, Error> {
        let ip = ip.to_c_string();
        let user = user.to_c_string();
        let pass = pass.to_c_string();
        let db = db.to_c_string();

        // Call taos_init at first connection.\
        {
            let mut n = TAOS_INIT_LOCK.lock().unwrap();
            if *n == 0 {
                unsafe { taos_init() };
            }
            *n += 1;
        }
        unsafe {
            let conn = taos_connect(
                ip.as_ptr(),
                user.as_ptr(),
                pass.as_ptr(),
                db.as_ptr(),
                port as u16,
            )
            .null_or();
            match conn {
                None => {
                    let null: *const c_void = std::ptr::null();
                    let _ = null.taos_error_or();
                    unreachable!()
                }
                Some(conn) => Ok(Taos { conn }),
            }
        }
    }

    pub async fn create_table(&self, table: &str, options: Option<&str>) -> Result<(), Error> {
        self.query(&format!("create table {} {}", table, options.unwrap_or("")))
            .await
            .map(|_| ())
    }

    pub async fn create_table_if_not_exists(
        &self,
        table: &str,
        options: Option<&str>,
    ) -> Result<(), Error> {
        self.query(&format!(
            "create table if not exists {} {}",
            table,
            options.unwrap_or("")
        ))
        .await
        .map(|_| ())
    }

    pub async fn create_database(&self, database: &str) -> Result<(), Error> {
        self.query(&format!("create database {}", database))
            .await
            .map(|_| ())
    }

    pub async fn create_database_if_exists(&self, database: &str) -> Result<(), Error> {
        self.query(&format!("create database if not exists {}", database))
            .await
            .map(|_| ())
    }

    pub async fn use_database(&self, database: &str) -> Result<(), Error> {
        self.query(&format!("use {}", database)).await.map(|_| ())
    }

    pub async fn describe(&self, table: &str) -> Result<TaosDescribe, Error> {
        self.query(&format!("describe {}", table))
            .await
            .map(TaosDescribe::from)
    }

    pub async fn exec(&self, sql: &str) -> Result<(), Error> {
        self.raw_query(sql).map(|_| ())
    }
    pub fn raw_query(&self, s: &str) -> Result<CTaosResult, Error> {
        let cstr = CString::new(s).expect("CString::new should not fail here");
        let res = CTaosResult::new(unsafe { taos_query(self.conn, cstr.as_ptr()) })?;
        Ok(res)
    }
    pub async fn query(&self, s: &str) -> Result<TaosQueryData, Error> {
        let res = self.raw_query(s)?;
        Ok(res.fetch_fields())
    }

    pub fn as_raw(&self) -> *mut TAOS {
        self.conn
    }
}

impl Drop for Taos {
    fn drop(&mut self) {
        let mut n = TAOS_INIT_LOCK.lock().unwrap();
        // reduce connection count and call clean_up after the last connection closed.
        unsafe {
            taos_close(self.conn);
        }
        *n -= 1;
        if *n == 0 {
            trace!("taos client workspace cleanup");
            unsafe { taos_cleanup() };
            trace!("taos client workspace cleaned");
        }
    }
}

#[derive(Debug)]
pub struct CTaosResult {
    res: *mut bindings::TAOS_RES,
}

impl CTaosResult {
    pub fn as_raw_mut_ptr(&mut self) -> *mut bindings::TAOS_RES {
        self.res
    }

    pub fn error_code(&self) -> TaosCode {
        (unsafe { taos_errno(self.res as _) } & 0x0000ffff).into()
    }

    pub fn error_string(&self) -> String {
        let err = unsafe { taos_errstr(self.res as _) };
        unsafe {
            CStr::from_ptr(err as *const c_char)
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn new(res: *mut TAOS_RES) -> Result<Self, TaosError> {
        let res = Self { res };
        let code = res.error_code();

        if !code.success() {
            let err = res.error_string();
            Err(TaosError {
                code,
                err: Cow::from(err),
            })
        } else {
            Ok(res)
        }
    }

    pub fn fetch_fields(&self) -> TaosQueryData {
        let fields = unsafe { taos_fetch_fields(self.res) };

        let fcount = unsafe { taos_field_count(self.res) };
        let mut rows = Vec::new();
        let fields = (0..fcount)
            .into_iter()
            .map(|i| {
                let field = &unsafe { *fields.offset(i as _) };
                let name = unsafe { CStr::from_ptr(&field.name as _) }
                    .to_string_lossy()
                    .into_owned();
                ColumnMeta {
                    name,
                    type_: field.type_.into(),
                    bytes: field.bytes,
                }
            })
            .collect_vec();

        while let Some(taos_row) = unsafe { taos_fetch_row(self.res) }.null_or() {
            let row = unsafe { std::slice::from_raw_parts(taos_row, fcount as usize) }
                .into_iter()
                .zip(fields.iter())
                .map(|(ptr, meta)| unsafe {
                    dbg!(meta);
                    dbg!(ptr);
                    //let ptr = taos_row.offset(i as _);
                    match meta.type_ {
                        TaosDataType::Null => Field::Null,
                        TaosDataType::Bool => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Bool(*(*ptr as *mut i8) != 0)
                            }
                        }
                        TaosDataType::TinyInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::TinyInt(*(*ptr as *mut i8))
                            }
                        }
                        TaosDataType::SmallInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::SmallInt(*(*ptr as *mut i16))
                            }
                        }
                        TaosDataType::Int => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Int(*(*ptr as *mut i32))
                            }
                        }
                        TaosDataType::BigInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::BigInt(*(*ptr as *mut i64))
                            }
                        }
                        TaosDataType::UTinyInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::UTinyInt(*(*ptr as *mut u8))
                            }
                        }
                        TaosDataType::USmallInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::USmallInt(*(*ptr as *mut u16))
                            }
                        }
                        TaosDataType::UInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::UInt(*(*ptr as *mut u32))
                            }
                        }
                        TaosDataType::UBigInt => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::UBigInt(*(*ptr as *mut u64))
                            }
                        }
                        TaosDataType::Timestamp => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Timestamp(*(*ptr as *mut i64))
                            }
                        }
                        TaosDataType::Float => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Float(*(*ptr as *mut f32))
                            }
                        }
                        TaosDataType::Double => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Double(*(*ptr as *mut f64))
                            }
                        }
                        TaosDataType::Binary => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::Binary(
                                    CStr::from_ptr((*ptr) as *const c_char)
                                        .to_string_lossy()
                                        .into_owned(),
                                )
                            }
                        }
                        TaosDataType::NChar => {
                            if ptr.is_null() {
                                Field::Null
                            } else {
                                Field::NChar(
                                    CStr::from_ptr((*ptr) as *const c_char)
                                        .to_string_lossy()
                                        .into_owned(),
                                )
                            }
                        }
                        _ => {
                            unreachable!("unexpected data type, please contract the author to fix!")
                        }
                    }
                })
                .collect_vec();
            rows.push(row);
        }
        TaosQueryData {
            column_meta: fields,
            rows,
        }
    }
}
impl Drop for CTaosResult {
    fn drop(&mut self) {
        unsafe {
            taos_free_result(self.res);
        }
    }
}
