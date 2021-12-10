mod init;

use libtaos::*;

#[tokio::test]
async fn double_quote() -> Result<(), Error> {
    init::init();
    let taos = init::taos().unwrap();
    let _ = taos
        .exec("create database if not exists test_rust_double_quote_tag")
        .await
        .unwrap();
    let _ = taos.use_database("test_rust_double_quote_tag").await?;
    let _ = taos.exec("drop stable if exists stb1").await?;
    let _ = taos
        .exec("create stable if not exists stb1 (ts timestamp, t double) tags (tag1 binary(100))")
        .await?;
    let _ = taos
        .exec("create table tb1 using stb1 tags(\"abc\\\"def\")")
        .await?;
    let _ = taos.exec("insert into tb1 values(now, 1.0)").await;
    let res = taos.query("select * from stb1").await?;
    dbg!(&res);
    Ok(())
}
