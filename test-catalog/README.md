# Test Catalog

It's a simple tool to collect test cases information and export as a catalog.

First, install `test-catalog` binary with `cargo install test-catalog`.

To use the test catalog specification, please add the dependencies to Cargo manifest file (Cargo.toml):

```toml
[dev-dependencies]
proc-test-catalog = "0.1.0"
test-catalog = "0.1.0"
```

Note that the `proc-test-catalog` crate needs nightly channel to compile.

After then, additional to legacy test cases, you should use `#[proc_test_catalog::test_catalogue]` attribute after `#[test]`, and then add the case description in the document line.

```rust
#[test]
#[proc_test_catalog::test_catalogue(since = "0.1.0")]
/// This is preferred case description.
fn it_works() {
    // some test codes.
    println!("ok");
}
```

Full supported attributes list for `test_catalogue` proc macro are listed beblow:

- **since**: test case is added since specific version.
- **compatible_version**: compatible TDengine versions expression, like `^2.3.0 && ~2.5.0` or ">2.0".
- **description**: override description with this attribute, other wise it will use in-doc description as default.

And then, apply `cargo test`, and export the test catalog with `test-catalog` tool.

Full information list about a test case is:

- **version**: current app version
- **file**: test case located file path relative to the main manifest file.
- **line_start**: line start for the test case in `file`.
- **line_end**: line end for the test case in `file`.
- **name**: test case function name.
- **description**: describe the current test case with distinguish information.
- **since**: the first version of the test case added.
- **compatible_version**: a property for TDengine only, compatible TDengine versions expression.
- **authors**: authors of the test case
- **created_at**: the creation time (seconds from epoch) of the test case.
- **last_commit_id**: the last commit id of the test case.
- **last_committer**: the last commit signature author of the test case.
- **last_committed_at**: the last modiffication time for the test case.
- **elapsed**: the test case time cost in nano seconds.

The `test-catalog` tool could export the test catalog to many file formats, including markdown, confluence wiki markup, tab-separated text(tsv), csv, or json, markdown by default.

For example, export the test catalog to CSV:

```bash
test-catalog -f csv
```
