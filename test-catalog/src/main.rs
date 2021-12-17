use linked_hash_map::LinkedHashMap as HashMap;

use clap::{ArgEnum, Parser};
use itertools::Itertools;
use test_catalog::{Case, Cataloger, Result};

#[derive(Debug, Clone, Copy, ArgEnum)]
enum Format {
    ///
    Markdown,
    Tsv,
    Csv,
    Json,
}
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Manifest directory, default to use project root manifest.
    #[clap(short, long)]
    manifest: Option<String>,

    /// Number of times to greet
    #[clap(short, long, arg_enum, default_value = "markdown")]
    format: Format,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let cataloger = match args.manifest {
        None => Cataloger::open_root()?,
        Some(path) => Cataloger::open(path)?,
    };
    let cases = cataloger.latest_cases()?;
    let names = Case::field_names();

    match args.format {
        Format::Markdown => {
            println!("{}", names.join("|"));
            println!(
                "{}",
                (0..names.len()).into_iter().map(|_| "---").join(" | ")
            );
            for case in cases {
                println!("{}", case.into_printable_fields().join("|"));
            }
        }
        Format::Tsv => {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(b'\t')
                .from_writer(std::io::stdout());
            wtr.write_record(&names)?;
            for case in cases {
                wtr.write_record(case.into_printable_fields())?;
            }
        }
        Format::Csv => {
            let mut wtr = csv::WriterBuilder::new().from_writer(std::io::stdout());
            wtr.write_record(&names)?;
            for case in cases {
                wtr.write_record(case.into_printable_fields())?;
            }
        }
        Format::Json => {
            let names = Case::field_names();
            let json = serde_json::to_string(
                &cases
                    .into_iter()
                    .map(|case| {
                        names
                            .iter()
                            .zip(case.into_printable_fields())
                            .collect::<HashMap<_, _>>()
                    })
                    .collect_vec(),
            )?;
            println!("{}", json);
        }
    }
    Ok(())
}
