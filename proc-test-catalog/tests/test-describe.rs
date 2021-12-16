use proc_test_catalog::test_catalogue;

#[test]
#[test_catalogue(
    since = "0.1.0",
    compatible_version = "^2.3",
    description = "in-doc description is preferred, bu you can override with this attribute"
)]
/// Simple test, only use first line as test case description.
///
/// Long description
fn simple_test() {
    println!("ok");
}
