use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use derive_builder::Builder;
use itertools::Itertools;
use log::*;
use thiserror::Error;

#[cfg(not(feature = "rest"))]
pub mod bindings;

mod error;
pub mod field;
#[cfg(feature = "rest")]
mod rest;
#[cfg(feature = "rest")]
pub use rest::*;

#[cfg(not(feature = "rest"))]
mod client;
#[cfg(not(feature = "rest"))]
pub use client::*;

pub use error::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("taos error: {0}")]
    RawTaosError(#[from] TaosError),
    #[cfg(feature = "rest")]
    #[error("rest error: {0}")]
    RestApiError(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub struct TaosError {
    pub code: TaosCode,
    pub err: Cow<'static, str>,
}

impl Display for TaosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.err)
    }
}

#[derive(Builder, Debug)]
#[builder(setter(into))]
pub struct TaosCfg {
    ip: String,
    user: String,
    pass: String,
    #[builder(setter(strip_option))]
    db: Option<String>,
    port: u16,
}

impl TaosCfg {
    #[cfg(feature = "rest")]
    pub fn connect(&self) -> Result<Taos, Error> {
        Ok(Taos::new(
            format!("http://{}:{}/rest/sql", self.ip, self.port + 11),
            self.user.clone(),
            self.pass.clone(),
        ))
    }

    #[cfg(not(feature = "rest"))]
    pub fn connect(&self) -> Result<Taos, Error> {
        let default_db = "log".to_string();
        Taos::new(
            &self.ip,
            &self.user,
            &self.pass,
            self.db.as_ref().unwrap_or(&default_db),
            self.port,
        )
    }
}

#[cfg(feature = "r2d2")]
pub type TaosPool = r2d2::Pool<TaosCfg>;

#[cfg(feature = "r2d2")]
impl r2d2::ManageConnection for TaosCfg {
    type Connection = Taos;
    type Error = Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.connect()
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        true
    }
}
