use crate::bindings::*;
use crate::*;

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::*;
use crate::field::*;

#[cfg(feature = "cleanup")]
lazy_static::lazy_static! {
    static ref TAOS_INIT_LOCK: std::sync::Mutex<u32> = std::sync::Mutex::new(0);
}

#[derive(Debug)]
pub struct Taos {
    conn: *mut TAOS,
}

unsafe impl Send for Taos {}
unsafe impl Sync for Taos {}

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

        #[cfg(feature = "cleanup")]
        // Call taos_init at first connection.
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
            .as_mut();
            match conn {
                None => Err(Error::ConnectionInvalid),
                Some(conn) => Ok(Taos { conn: conn as _ }),
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

    pub async fn exec(&self, sql: impl ToCString) -> Result<(), Error> {
        self.raw_query(sql).map(|_| ())
    }
    pub fn raw_query(&self, s: impl ToCString) -> Result<CTaosResult, Error> {
        let cstr = s.to_c_string();
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
        // reduce connection count and call clean_up after the last connection closed.
        unsafe {
            taos_close(self.conn);
        }
        #[cfg(feature = "cleanup")]
        {
            let mut n = TAOS_INIT_LOCK.lock().unwrap();
            *n -= 1;
            if *n == 0 {
                trace!("taos client workspace cleanup");
                unsafe { taos_cleanup() };
                trace!("taos client workspace cleaned");
            }
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

        while let Some(taos_row) = unsafe { taos_fetch_row(self.res).as_ref() } {
            let row = unsafe { std::slice::from_raw_parts(taos_row, fcount as usize) }
                .iter()
                .zip(fields.iter())
                .map(|(ptr, meta)| unsafe {
                    if ptr.is_null() {
                        return Field::Null;
                    }
                    //let ptr = taos_row.offset(i as _);
                    match meta.type_ {
                        TaosDataType::Null => Field::Null,
                        TaosDataType::Bool => Field::Bool(*(*ptr as *mut i8) != 0),
                        TaosDataType::TinyInt => Field::TinyInt(*(*ptr as *mut i8)),
                        TaosDataType::SmallInt => Field::SmallInt(*(*ptr as *mut i16)),
                        TaosDataType::Int => Field::Int(*(*ptr as *mut i32)),
                        TaosDataType::BigInt => Field::BigInt(*(*ptr as *mut i64)),
                        TaosDataType::UTinyInt => Field::UTinyInt(*(*ptr as *mut u8)),
                        TaosDataType::USmallInt => Field::USmallInt(*(*ptr as *mut u16)),
                        TaosDataType::UInt => Field::UInt(*(*ptr as *mut u32)),
                        TaosDataType::UBigInt => Field::UBigInt(*(*ptr as *mut u64)),
                        TaosDataType::Timestamp => Field::Timestamp(Timestamp::new(
                            *(*ptr as *mut i64),
                            taos_result_precision(self.res),
                        )),
                        TaosDataType::Float => Field::Float(*(*ptr as *mut f32)),
                        TaosDataType::Double => Field::Double(*(*ptr as *mut f64)),
                        TaosDataType::Binary => Field::Binary(
                            CStr::from_ptr((*ptr) as *const c_char)
                                .to_string_lossy()
                                .into_owned()
                                .into(),
                        ),
                        TaosDataType::NChar => Field::NChar(
                            CStr::from_ptr((*ptr) as *const c_char)
                                .to_string_lossy()
                                .into_owned(),
                        ),
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
