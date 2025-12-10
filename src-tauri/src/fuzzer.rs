//! `lore_lib` fuzzer.

use nucleo::{Config, Nucleo};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// The number of columns that will be allocated for the fuzzer.
const NUM_COLUMNS: usize = 1;

/// The current state of the fuzzer.
#[derive(Serialize)]
pub enum FuzzerState {
    /// The fuzzer has been created but is unable to perform searches.
    Uninitialized,

    /// A thread panicked while the fuzzer was in use, it is now unusable.
    Poisoned,

    /// The fuzzer has been created and can search.
    Initialized,
}

/// A fuzzy-finding matcher to search for files in a store.
pub struct Fuzzer {
    matcher: RwLock<Option<Nucleo<String>>>,

    /// The working directory of the matcher. Guaranteed to exist as fuzzer creation
    /// fails if the directory does not exist and is not accessible.
    pub path: PathBuf,
}

// TODO: Dangerous because no data has been injected, so the matcher will never match anything.
//       Good for now but need to find the most idiomatic way to initialize the data _without_ compromising
//       startup (which naturally will not have a set directory yet).
impl Default for Fuzzer {
    fn default() -> Self {
        Self {
            matcher: RwLock::new(None),
            path: PathBuf::new(),
        }
    }
}

impl Fuzzer {
    /// Builds a [`Fuzzer`].
    pub fn new(root_dir: &Path) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, NUM_COLUMNS as u32);

        walkdir::WalkDir::new(root_dir)
            .min_depth(1) // prevent matching on working dir
            .into_iter()
            .flat_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .for_each(|e| {
                let entry = e
                    .path()
                    .strip_prefix(root_dir)
                    .expect("entry is a child item of root_dir")
                    .display()
                    .to_string();
                eprintln!("adding entry: {}", entry);

                matcher.injector().push(entry, |data, cols| {
                    cols[0] = data.clone().into();
                });
            });

        eprintln!("matcher initialized");

        Self {
            matcher: RwLock::new(Some(matcher)),
            path: root_dir.to_path_buf(),
        }
    }

    /// Perform an approximate string match search for the key.
    ///
    /// # Appending
    ///
    /// There are some minor performance optimization gains we can get by notifying the matcher
    /// if the new key is the previous key with some new text appended on it.
    ///
    /// > TODO: Save key of prior search and track is_append completely in the backend.
    pub fn search(&self, key: &str, is_append: bool) -> Vec<String> {
        let mut matcher_lock = self
            .matcher
            .write()
            .expect("search cannot panic so this lock cannot be poisoned");
        let matcher = matcher_lock
            .as_mut()
            .expect("matcher should be initialized before search is called");

        match key.len() {
            0 => eprintln!("no key provided"),
            1.. => eprintln!("performing search for key: {key}, is_append: {is_append}"),
        };

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
            .map(|item| item.data)
            .cloned()
            .collect::<Vec<_>>()
    }

    /// Get a [`FuzzerState`]. The fuzzer is only ready for searching if it is [`FuzzerState::Initialized`].
    pub fn get_state(&self) -> FuzzerState {
        let lock = match self.matcher.read() {
            Ok(matcher) => matcher,
            Err(_) => return FuzzerState::Poisoned,
        };

        match lock.is_some() {
            true => FuzzerState::Initialized,
            false => FuzzerState::Uninitialized,
        }
    }
}
