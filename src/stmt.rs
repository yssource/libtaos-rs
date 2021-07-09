use crate::bindings::*;
use crate::*;

use std::ffi::CStr;
use std::os::raw::c_void;

mod bind;
pub use bind::{BindParam, IntoBindParam};

pub trait IntoParams {
    fn into_params(self) -> Vec<BindParam>;
}

impl<F: IntoBindParam, T: IntoIterator<Item = F>> IntoParams for T {
    fn into_params(self) -> Vec<BindParam> {
        self.into_iter().map(|v| v.into_bind_param()).collect()
    }
}

pub struct Stmt {
    stmt: *mut c_void,
}

impl Stmt {
    fn err_or(&self, res: i32) -> Result<(), TaosError> {
        if res != 0 {
            let code: TaosCode = (res & 0x0000ffff).into();
            let err = unsafe { taos_stmt_errstr(self.stmt) };
            if !err.is_null() {
                let err = unsafe { CStr::from_ptr(err as _) }
                    .to_string_lossy()
                    .to_owned();
                trace!("stmt error: {:?}", err);
                return Err(TaosError { code, err });
            }
            return Err(TaosError {
                code,
                err: "unknown".into(),
            });
        }
        Ok(())
    }
    /// NOT a public method
    fn prepare(&mut self, sql: impl ToCString) -> Result<(), TaosError> {
        let res = unsafe { taos_stmt_prepare(self.stmt, sql.to_c_string().as_ptr(), 0) };
        self.err_or(res)
    }
    pub fn execute(&self) -> Result<(), TaosError> {
        unsafe {
            let res = taos_stmt_execute(self.stmt);
            self.err_or(res)
        }
    }

    /// To bind one row with params
    pub fn bind(&mut self, params: impl IntoParams) -> Result<(), TaosError> {
        let params = params.into_params();
        //assert_eq!(self.num_params(), params.len());
        unsafe {
            let res = taos_stmt_bind_param(self.stmt, params.as_ptr() as _);
            self.err_or(res)?;
            let res = taos_stmt_add_batch(self.stmt);
            self.err_or(res)?;
        }
        for mut param in params {
            unsafe { param.free() };
        }
        Ok(())
    }

    pub fn bind_batch_at_col<T>(&mut self, _params: T) -> Result<(), TaosError>
    where
        T: IntoIterator,
        T::Item: IntoBindParam,
    {
        Ok(())
    }

    pub fn num_params(&self) -> usize {
        unsafe {
            let mut num = 0;
            taos_stmt_num_params(self.stmt, &mut num as _);
            num as _
        }
    }
    pub fn set_tbname_tags(
        &mut self,
        tbname: impl ToCString,
        tags: impl IntoParams,
    ) -> Result<(), TaosError> {
        let tags = tags.into_params();
        unsafe {
            let res = taos_stmt_set_tbname_tags(
                self.stmt,
                tbname.to_c_string().as_ptr(),
                tags.as_ptr() as _,
            );
            self.err_or(res)
        }
    }
    pub fn set_tbname(&mut self, tbname: impl ToCString) -> Result<(), TaosError> {
        unsafe {
            let res = taos_stmt_set_tbname(self.stmt, tbname.to_c_string().as_ptr());
            self.err_or(res)
        }
    }
    pub fn is_insert(&self) -> bool {
        unsafe {
            let mut res = 0;
            taos_stmt_is_insert(self.stmt, &mut res as _);
            res != 0
        }
    }
    fn close(&mut self) {
        unsafe {
            taos_stmt_close(self.stmt);
        }
    }
}

impl Drop for Stmt {
    fn drop(&mut self) {
        self.close();
    }
}

impl Taos {
    /// Create stmt with sql
    pub fn stmt(&self, sql: impl ToCString) -> Result<Stmt, TaosError> {
        let sql = sql.to_c_string();
        unsafe {
            let stmt = taos_stmt_init(self.as_raw());
            // let res = taos_stmt_prepare(stmt, sql.as_ptr(), 0);
            let mut stmt = Stmt { stmt };
            stmt.prepare(sql)?;
            Ok(stmt)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test::taos;
    use crate::*;

    async fn stmt_test(db: &str, ty: &str, value: Field) -> Result<(), Error> {
        let taos = taos()?;
        println!("test {} using {}", ty, db);
        taos.exec(format!("drop database if exists {}", db)).await?;
        taos.exec(format!("create database if not exists {} keep 36500", db))
            .await?;
        taos.exec(format!("use {}", db)).await?;
        taos.exec(format!(
            "create table if not exists tb0 (ts timestamp, n {})",
            ty
        ))
        .await?;
        let mut stmt = taos.stmt("insert into ? values(?,?)")?;
        stmt.set_tbname("tb0")?;
        assert!(stmt.is_insert());
        assert_eq!(stmt.num_params(), 2);
        let ts = Field::Timestamp(Timestamp::now());
        stmt.bind(vec![ts, value.clone()].iter())?;
        let _ = stmt.execute()?;
        let res = taos.query("select n from tb0").await?;
        assert_eq!(value, res.rows[0][0]);
        taos.exec(format!("drop database {}", db)).await?;
        Ok(())
    }
    #[tokio::test]
    async fn bool_null() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "bool", Field::Null).await
    }
    #[tokio::test]
    async fn int_null() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "int", Field::Null).await
    }
    #[tokio::test]
    async fn float_null() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "float", Field::Null).await
    }
    #[tokio::test]
    async fn bool() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "bool", Field::Bool(true)).await
    }
    #[tokio::test]
    async fn tinyint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "tinyint", Field::TinyInt(-0x7f)).await
    }
    #[tokio::test]
    async fn smallint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "smallint", Field::SmallInt(0x7fff)).await
    }
    #[tokio::test]
    async fn int() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "int", Field::Int(0x7fffffff)).await
    }
    #[tokio::test]
    async fn bigint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "bigint", Field::BigInt(0x7fffffff_ffffffff)).await
    }
    #[tokio::test]
    async fn utinyint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "tinyint unsigned", Field::UTinyInt(0)).await
    }
    #[tokio::test]
    async fn usmallint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "smallint unsigned", Field::USmallInt(1)).await
    }
    #[tokio::test]
    async fn uint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "int unsigned", Field::UInt(1)).await
    }
    #[tokio::test]
    async fn ubigint() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "bigint unsigned", Field::UBigInt(1)).await
    }
    #[tokio::test]
    async fn float() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "float", Field::Float(1.0)).await
    }
    #[tokio::test]
    async fn double() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        stmt_test(&db, "double", Field::Double(1.0)).await
    }
    #[tokio::test]
    async fn binary() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        let v = Field::Binary("0123456789".into());
        stmt_test(&db, "binary(10)", v).await
    }
    #[tokio::test]
    async fn nchar() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        let v = Field::NChar("一二三四五六七八九十".into());
        stmt_test(&db, "nchar(10)", v).await
    }

    #[tokio::test]
    async fn all_types() -> Result<(), Error> {
        let taos = taos()?;
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        println!("{}", db);
        taos.exec(format!("drop database if exists {}", db)).await?;
        taos.exec(format!("create database if not exists {} keep 36500", db))
            .await?;
        taos.exec(format!("use {}", db)).await?;
        taos.exec(
            "create table if not exists tb0 (ts timestamp,
             c1 tinyint, c2 smallint, c3 int, c4 bigint,
             c5 tinyint unsigned, c6 smallint unsigned, c7 int unsigned, c8 bigint unsigned,
             c9 float, c10 double, c11 binary(10), c12 nchar(10))",
        )
        .await?;
        let mut stmt = taos.stmt("insert into ? values(?,?,?,?,?,?,?,?,?,?,?,?,?)")?;
        stmt.set_tbname("tb0")?;
        assert!(stmt.is_insert());

        assert_eq!(stmt.num_params(), 13);
        let ts = Field::Timestamp(Timestamp::now());
        let c1 = Field::TinyInt(1);
        let c2 = Field::SmallInt(2);
        let c3 = Field::Int(3);
        let c4 = Field::BigInt(4);
        let c5 = Field::UTinyInt(5);
        let c6 = Field::USmallInt(6);
        let c7 = Field::UInt(7);
        let c8 = Field::UBigInt(8);
        let c9 = Field::Float(9.0);
        let c10 = Field::Double(9.0);
        let c11 = Field::Binary("binary".into());
        let c12 = Field::NChar("nchar".into());
        stmt.bind(vec![ts, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12].iter())?;
        let _ = stmt.execute()?;
        let res = taos.query("select count(*) as count from tb0").await?;
        println!("{:?}", res);
        taos.exec(format!("drop database {}", db)).await?;
        Ok(())
    }
    #[tokio::test]
    async fn test_stmt_tags() -> Result<(), Error> {
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        println!("{}", db);
        let taos = taos()?;
        taos.exec(format!("drop database if exists {}", db)).await?;
        taos.exec(format!("create database if not exists {} keep 36500", db))
            .await?;
        taos.exec(format!("use {}", db)).await?;
        taos.exec("create table if not exists stb (ts timestamp, n int) tags(b int)")
            .await?;

        let mut stmt = taos.stmt("insert into ? using stb tags(?) values(now,?)")?;

        stmt.set_tbname_tags("tb0", [0])?;
        stmt.bind(&[0])?;
        assert!(stmt.is_insert());
        assert_eq!(stmt.num_params(), 1);

        stmt.set_tbname_tags("tb1", &[1])?;
        stmt.bind(&[&1])?;

        stmt.set_tbname_tags("tb2", &[2])?;
        stmt.bind([&2])?;

        stmt.set_tbname_tags("tb3", &[3])?;
        stmt.bind([3i32])?;

        stmt.set_tbname_tags("tb4", &[4])?;
        stmt.bind([Field::Int(4)])?;

        stmt.set_tbname_tags("tb5", &[5])?;
        stmt.bind([&Field::Int(5)])?;

        stmt.set_tbname_tags("tb6", &[6])?;
        stmt.bind(&[Field::Int(6)])?;

        stmt.set_tbname_tags("tb7", &[7])?;
        stmt.bind(&[&Field::Int(5)])?;

        stmt.set_tbname_tags("tb8", &[8])?;
        stmt.bind(vec![Field::Int(8)])?;

        stmt.set_tbname_tags("tb9", [9])?;
        stmt.bind(vec![&Field::Int(9)])?;

        let _ = stmt.execute()?;
        let res = taos.query("select count(*) as count from stb").await?;
        assert_eq!(res.rows[0][0], Field::BigInt(10));
        taos.exec(format!("drop database {}", db)).await?;
        Ok(())
    }
    #[tokio::test]
    async fn test_stmt_jtqx() -> Result<(), Error> {
        std::env::set_var("RUST_LOG", "libtaos=trace");
        env_logger::init();
        let db = stdext::function_name!()
            .replace("::{{closure}}", "")
            .replace("::", "_");
        println!("{}", db);
        let taos = taos()?;
        taos.exec(format!("drop database if exists {}", db)).await?;
        taos.exec(format!("create database if not exists {} keep 36500", db))
            .await?;
        taos.exec(format!("use {}", db)).await?;
        taos.exec("create table if not exists stb (ts timestamp, n int) tags(b int)")
            .await?;

        let mut stmt = taos.stmt("insert into ? using parquet (City,Cnty,Country,Province,Station_Id_d,Station_Name,Town)  tags(?,?,?,?,?,?,?)  (Admin_Code_CHN,Alti,Aur,CLO_Cov_Avg,DATA_ID,Datetime,Day,Dew,DrSnow,DrSnow_OTime,DuWhr,EICE,EICED_NS,EICED_WE,EICET_NS,EICET_WE,EICEW_NS,EICEW_WE,EVP_Big,FRS_1st_Bot,FRS_1st_Top,FRS_2nd_Bot,FRS_2nd_Top,FlDu,FlSa,Fog,Fog_OTime,Frost,GLAZE_OTime,GSS,GST_Avg,GST_Avg_10cm,GST_Avg_15cm,GST_Avg_160cm,GST_Avg_20cm,GST_Avg_320cm,GST_Avg_40cm,GST_Avg_5cm,GST_Avg_80cm,GST_Max,GST_Min,GaWIN,GaWIN_OTime,Glaze,HAIL_OTime,Hail,Haze,ICE,IcePri,LGST_Avg,LGST_Max,LGST_Min,Lat,Lit,Lon,Mist,Mon,PRE_Max_1h,PRE_Max_1h_OTime,PRE_OTime,PRE_Time_0808,PRE_Time_0820,PRE_Time_2008,PRE_Time_2020,PRS_Avg,PRS_Max,PRS_Min,PRS_Sea_Avg,PRS_Sensor_Alti,Q_Aur,Q_Dew,Q_DrSnow,Q_DrSnow_OTime,Q_DuWhr,Q_EICE,Q_EICED_NS,Q_EICED_WE,Q_EICET_NS,Q_EICET_WE,Q_EICEW_NS,Q_EICEW_WE,Q_EVP_Big,Q_FRS_1st_Bot,Q_FRS_1st_Top,Q_FRS_2nd_Bot,Q_FRS_2nd_Top,Q_FlDu,Q_FlSa,Q_Fog,Q_Fog_OTime,Q_Frost,Q_GLAZE_OTime,Q_GSS,Q_GST_Avg,Q_GST_Avg_10cm,Q_GST_Avg_15cm,Q_GST_Avg_160cm,Q_GST_Avg_20cm,Q_GST_Avg_320cm,Q_GST_Avg_40cm,Q_GST_Avg_5cm,Q_GST_Avg_80cm,Q_GST_Max,Q_GST_Min,Q_GaWIN,Q_GaWIN_OTime,Q_Glaze,Q_HAIL_OTime,Q_Hail,Q_Haze,Q_ICE,Q_IcePri,Q_LGST_Avg,Q_LGST_Max,Q_LGST_Min,Q_Lit,Q_Mist,Q_PRE_Max_1h,Q_PRE_Max_1h_OTime,Q_PRE_OTime,Q_PRE_Time_0808,Q_PRE_Time_0820,Q_PRE_Time_2008,Q_PRE_Time_2020,Q_PRS_Avg,Q_PRS_Max,Q_PRS_Min,Q_PRS_Sea_Avg,Q_RHU_Avg,Q_RHU_Min,Q_Rain,Q_SQUA_OTime,Q_SSH,Q_SaSt,Q_SaSt_OTime,Q_Smoke,Q_Snow,Q_SnowSt,Q_SnowSt_OTime,Q_Snow_Depth,Q_Snow_OTime,Q_Snow_PRS,Q_SoRi,Q_SoRi_OTime,Q_Squa,Q_TEM,Q_TEM_Avg,Q_TEM_Max,Q_TEM_Max_OTime,Q_TEM_Min,Q_TEM_Min_OTime,Q_THUND_OTime,Q_Thund,Q_Tord,Q_Tord_OTime,Q_VAP_Avg,Q_VIS_Avg_10mi_Hourly,Q_VIS_Min,Q_WEP_Record,Q_WEP_Sumary,Q_WIN_D,Q_WIN_D_2mi_Avg_C,Q_WIN_D_INST_Max,Q_WIN_D_S_Max,Q_WIN_S,Q_WIN_S_10mi_Avg,Q_WIN_S_2mi_Avg,Q_WIN_S_INST_Max_OTime,Q_WIN_S_Inst_Max,Q_WIN_S_Max,Q_WIN_S_Max_OTime,RHU_Avg,RHU_Min,Rain,SQUA_OTime,SSH,SaSt,SaSt_OTime,Smoke,Snow,SnowSt,SnowSt_OTime,Snow_Depth,Snow_OTime,Snow_PRS,SoRi,SoRi_OTime,Squa,Station_Id_C,Station_levl,TEM,TEM_Avg,TEM_Max,TEM_Max_OTime,TEM_Min,TEM_Min_OTime,THUND_OTime,Thund,Tord,Tord_OTime,VAP_Avg,VIS_Min,WEP_Sumary,WIN_D_INST_Max,WIN_D_S_Max,WIN_S_10mi_Avg,WIN_S_2mi_Avg,WIN_S_INST_Max_OTime,WIN_S_Inst_Max,WIN_S_Max,WIN_S_Max_OTime,Year) values(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")?;

        stmt.set_tbname_tags("tb0", [0])?;
        stmt.bind(&[0])?;
        assert!(stmt.is_insert());
        assert_eq!(stmt.num_params(), 1);

        stmt.set_tbname_tags("tb1", &[1])?;
        stmt.bind(&[&1])?;

        stmt.set_tbname_tags("tb2", &[2])?;
        stmt.bind([&2])?;

        stmt.set_tbname_tags("tb3", &[3])?;
        stmt.bind([3i32])?;

        stmt.set_tbname_tags("tb4", &[4])?;
        stmt.bind([Field::Int(4)])?;

        stmt.set_tbname_tags("tb5", &[5])?;
        stmt.bind([&Field::Int(5)])?;

        stmt.set_tbname_tags("tb6", &[6])?;
        stmt.bind(&[Field::Int(6)])?;

        stmt.set_tbname_tags("tb7", &[7])?;
        stmt.bind(&[&Field::Int(5)])?;

        stmt.set_tbname_tags("tb8", &[8])?;
        stmt.bind(vec![Field::Int(8)])?;

        stmt.set_tbname_tags("tb9", [9])?;
        stmt.bind(vec![&Field::Int(9)])?;

        let _ = stmt.execute()?;
        let res = taos.query("select count(*) as count from stb").await?;
        assert_eq!(res.rows[0][0], Field::BigInt(10));
        taos.exec(format!("drop database {}", db)).await?;
        Ok(())
    }
}
