use std::path::Path;
use std::sync::{Arc, Mutex, Once};

use lazy_static::lazy_static;
use rusqlite::{params, Connection};

pub struct CaseIdentity<'a> {
    file: &'a str,
    name: &'a str,
}

impl<'a> CaseIdentity<'a> {
    pub fn new(file: &'a str, name: &'a str) -> Self {
        Self { file, name }
    }
}
pub struct Cataloger {
    version: String,
    connection: Connection,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Cataloger {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Cataloger> {
        Connection::open(path)
            .map(|connection| {
                let cataloger = Cataloger {
                    version: std::env::var("CARGO_PKG_VERSION").unwrap().into(),
                    connection,
                };
                cataloger.assert_schema().unwrap();
                cataloger
            })
            .map_err(|err| err.into())
    }

    fn assert_schema(&self) -> Result<()> {
        self.connection.execute_batch(
            "
        BEGIN;
        CREATE TABLE IF NOT EXISTS catalog(
            version, file, line_start, line_end, name, description, since, compatible_version,
            authors, created_at, last_commit_id, last_committer, last_committed_at, elapsed);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_version_file_name ON catalog(
            version, file, name);
        COMMIT;",
        )?;
        Ok(())
    }

    pub fn post_test<'a>(
        &self,
        case: &CaseIdentity<'a>,
        duration: &std::time::Duration,
    ) -> Result<()> {
        self.connection.execute(
            "update catalog set elapsed = ? where version = ? and file = ? and name = ?",
            params![
                duration.as_nanos() as u64,
                self.version,
                case.file,
                case.name
            ],
        )?;
        Ok(())
    }
}

lazy_static! {
    static ref CATALOGER: Arc<Mutex<Option<Cataloger>>> = Arc::new(Mutex::new(None));
}
static INIT: Once = Once::new();
pub fn init() {
    INIT.call_once(|| {
        let project_root = project_root::get_project_root().expect("can not find project root");
        // let path = std::env::var("CARGO_MANIFEST_DIR").unwrap().to_string();
        let path = project_root
            .join("target")
            .join("test-catalog");
        std::fs::create_dir_all(&path).expect("cannot create dir for tests catalog");
        let file = path.join("catalog.db");
        let catalogger = Cataloger::open(file).expect("cannot open");
        let mut guard = CATALOGER.lock().unwrap();
        *guard = Some(catalogger);
    });
}

pub fn catalogue(
    file: &str,
    name: &str,
    line_start: usize,
    line_end: usize,
    description: &str,
    since: &str,
    compatible_version: &str,
) {
    // let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_root = project_root::get_project_root().expect("can not find project root");
    let repo = git2::Repository::open(project_root).unwrap();
    let mut blame_options = git2::BlameOptions::default();
    blame_options.min_line(line_start).max_line(line_end).track_copies_any_commit_copies(true);
    let version = std::env::var("CARGO_PKG_VERSION").unwrap().to_string();
    let blame = repo
        .blame_file(Path::new(dbg!(file)), Some(&mut blame_options))
        .expect("blame");
    if let (Some(created_at), authors, last_commit_id, last_committer, Some(last_committed_at)) =
        blame
            .iter()
            .map(|chunk| {
                let last_commit_id = chunk.final_commit_id().to_string();
                let last_committer = chunk.final_signature().to_string();
                // let orig_commit_id = chunk.orig_commit_id().to_string();
                let orig_committer = chunk.orig_signature().to_string();
                let created_at = chunk.orig_signature().when().seconds();
                let last_committed_at = chunk.final_signature().when().seconds();
                println!("- {}", created_at);
                (
                    created_at,
                    orig_committer,
                    last_commit_id,
                    last_committer,
                    last_committed_at,
                )
            })
            .fold(
                (None, vec![], String::new(), "".to_string(), None),
                |mut acc, item| {
                    if acc.0.is_none() {
                        acc.0 = Some(item.0);
                    } else if acc.0.unwrap() > item.0 {
                        acc.0 = Some(item.0);
                    }
                    if !acc.1.contains(&item.1) {
                        acc.1.push(item.1);
                    }
                    if acc.4.is_none() || acc.0.unwrap() < item.4 {
                        acc.4 = Some(item.4);
                        acc.2 = item.2;
                        acc.3 = item.3;
                    }
                    acc
                },
            )
    {
        let guard = CATALOGER.lock().unwrap();
        let cater = guard.as_ref().unwrap();

        cater.connection.execute("
        INSERT INTO catalog (
            version, file, line_start, line_end, name, description, since, compatible_version, authors, created_at, last_commit_id, last_committer, last_committed_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(version, file, name) DO UPDATE SET
            line_start = excluded.line_start,
            line_end = excluded.line_end,
            description = excluded.description,
            authors = excluded.authors,
            created_at = excluded.created_at,
            since = excluded.since,
            last_commit_id = excluded.last_commit_id,
            last_committer = excluded.last_committer,
            last_committed_at = excluded.last_committed_at",
            params![version, file, line_start, line_end, name, description, since, compatible_version, authors.join(","),
                created_at, last_commit_id, last_committer, last_committed_at]).unwrap();
    }
}

pub fn pre<'a>(_case: &CaseIdentity<'a>) {
    init();
}

pub fn post<'a>(case: &CaseIdentity<'a>, duration: &std::time::Duration) {
    let guard = CATALOGER.lock().unwrap();
    let cater = guard.as_ref().unwrap();
    cater.post_test(case, duration).unwrap();
}
