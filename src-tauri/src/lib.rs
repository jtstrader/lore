use std::{
    io,
    path::Path,
    sync::{Arc, RwLock},
};

use nucleo::{Config, Nucleo};
use tauri::State;

const NUM_COLUMNS: usize = 1;

struct Fuzzer {
    matcher: RwLock<Option<Nucleo<&'static str>>>,
}

// TODO: Dangerous because no data has been injected, so the matcher will never match anything.
// Good for now but need to find the most idiomatic way to intitialize the data _without_ compromising
// startup (which naturally will not have a set directory yet).
impl Default for Fuzzer {
    fn default() -> Self {
        Self {
            matcher: RwLock::new(None),
        }
    }
}

impl Fuzzer {
    fn new(_root_dir: &Path) -> Result<Self, io::Error> {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, NUM_COLUMNS as u32);

        // TODO: Read from disk rather than dummy data.
        const CANDIDATES: &[&str] = &[
            "foo/bar", "foo bar", "potato", "sandwich", "flu bar", "foo bot", "fubar", "yaks",
        ];

        CANDIDATES.into_iter().for_each(|&x| {
            matcher.injector().push(x, |data, cols| {
                cols[0] = (*data).into();
            });
        });

        eprintln!("matcher initialized");

        Ok(Self {
            matcher: RwLock::new(Some(matcher)),
        })
    }

    fn search(&self, key: &str, is_append: bool) -> Vec<String> {
        eprintln!("performing search for key: {key}, is_append: {is_append}");

        let mut matcher_lock = self
            .matcher
            .write()
            .expect("search cannot panic so this lock cannot be poisoned");
        let matcher = matcher_lock
            .as_mut()
            .expect("matcher should be initialized before search is called");

        matcher.pattern.reparse(
            0,
            key,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Never,
            is_append,
        );

        matcher.tick(10);
        matcher
            .snapshot()
            .matched_items(..)
            .into_iter()
            .map(|item| *item.data)
            .map(String::from)
            .collect::<Vec<_>>()
    }
}

// TODO: Will likely need to return a preview of the image data. Should the frontend handle this?
#[tauri::command]
fn search(key: &str, is_append: bool, fuzzer: State<Fuzzer>) -> Vec<String> {
    fuzzer.search(key, is_append)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Fuzzer::new(Path::new("")).unwrap())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
