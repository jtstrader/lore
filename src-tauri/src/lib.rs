use normpath::PathExt;
use nucleo::{Config, Nucleo};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{
    io,
    path::Path,
    sync::{Arc, RwLock},
};
use tauri::{App, Manager, State, Wry};

const NUM_COLUMNS: usize = 1;
static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let sys_cfg_dir =
        dirs::config_dir().expect("Supported operating systems are Linux, macOS, and Windows");
    sys_cfg_dir.join("lore/config.json")
});

static DEFAULT_STORE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let sys_cfg_dir =
        dirs::config_dir().expect("Supported operating systems are Linux, macOS, and Windows");
    sys_cfg_dir.join("lore/store")
});

#[derive(Serialize, Deserialize)]
struct AppConfig {
    store_dir: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            store_dir: DEFAULT_STORE_PATH.to_str().map(ToString::to_string),
        }
    }
}

/// The current state of the fuzzer.
#[derive(Serialize)]
enum FuzzerState {
    /// The fuzzer has been created but is unable to perform searches.
    Uninitialized,

    /// A thread panicked while the fuzzer was in use, it is now unusable.
    Poisoned,

    /// The fuzzer has been created and can search.
    Initialized,
}

struct Fuzzer {
    matcher: RwLock<Option<Nucleo<String>>>,
    path: PathBuf,
}

// TODO: Dangerous because no data has been injected, so the matcher will never match anything.
// Good for now but need to find the most idiomatic way to initialize the data _without_ compromising
// startup (which naturally will not have a set directory yet).
impl Default for Fuzzer {
    fn default() -> Self {
        Self {
            matcher: RwLock::new(None),
            path: PathBuf::new(),
        }
    }
}

impl Fuzzer {
    fn new(root_dir: &Path) -> Result<Self, io::Error> {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, NUM_COLUMNS as u32);

        walkdir::WalkDir::new(root_dir)
            .min_depth(1) // prevent matching on working dir
            .into_iter()
            .filter_map(|e| e.ok())
            .for_each(|entry| {
                let entry = entry
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

        Ok(Self {
            matcher: RwLock::new(Some(matcher)),
            path: root_dir.to_path_buf(),
        })
    }

    fn search(&self, key: &str, is_append: bool) -> Vec<String> {
        let mut matcher_lock = self
            .matcher
            .write()
            .expect("search cannot panic so this lock cannot be poisoned");
        let matcher = matcher_lock
            .as_mut()
            .expect("matcher should be initialized before search is called");

        match key.len() {
            0 => eprintln!("no key provided"),
            1.. => eprintln!("performing search for key: {key}, is_append: {is_append}")
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
            .into_iter()
            .map(|item| item.data)
            .cloned()
            .collect::<Vec<_>>()
    }

    fn get_state(&self) -> FuzzerState {
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

// TODO: Will likely need to return a preview of the image data. Should the frontend handle this?
#[tauri::command]
fn search(key: &str, is_append: bool, fuzzer: State<Fuzzer>) -> Vec<String> {
    fuzzer.search(key, is_append)
}

#[tauri::command]
fn get_state(fuzzer: State<Fuzzer>) -> FuzzerState {
    fuzzer.get_state()
}

#[tauri::command]
fn get_fuzzer_working_dir(fuzzer: State<Fuzzer>) -> String {
    fuzzer
        .path
        .normalize()
        .expect("path must be valid for the fuzzer")
        .into_path_buf()
        .display()
        .to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(init_app_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            search,
            get_state,
            get_fuzzer_working_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_app_state(app: &mut App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    /*
     * Steps to initialize:
     *   1. Check to see if the config exists. If not, init the app with a default fuzzer.
     *   2. If the config exists, grab the directory and attempt to init the fuzzer with
     *      the provided directory. If this fails, raise an error and init the app with
     *      a default fuzzer.
     */

    if !CONFIG_PATH.exists() || !CONFIG_PATH.is_file() {
        eprintln!("lore config did not exist");

        // Create default config.
        fs::create_dir_all(
            CONFIG_PATH
                .parent()
                .expect("config path guaranteed to have at least 1 parent"),
        )?;
        fs::File::create(CONFIG_PATH.as_path())?;
        let cfg_str = serde_json::to_string(&AppConfig::default())?;
        fs::write(CONFIG_PATH.as_path(), cfg_str.as_bytes())?;

        // Create default store.
        fs::create_dir_all(&*DEFAULT_STORE_PATH)?;

        eprintln!("lore config successfully created");

        // Init app state with default.
        app.manage(
            Fuzzer::new(&*DEFAULT_STORE_PATH)
                .expect("store was just successfully created meaning we have perms"),
        );
        return Ok(());
    } else if !DEFAULT_STORE_PATH.exists() || !DEFAULT_STORE_PATH.is_dir() {
        fs::create_dir_all(&*DEFAULT_STORE_PATH)?;
    }

    let cfg_str = fs::read_to_string(CONFIG_PATH.as_path())?;
    let cfg = serde_json::from_str::<AppConfig>(&cfg_str)?;

    let Some(lore_dir) = cfg.store_dir else {
        eprintln!("CRITICAL: lore config exists but has no set directory");
        eprintln!("  > this means someone has messed with the file somehow...");
        app.manage(Fuzzer::new(&*DEFAULT_STORE_PATH).expect("hope and pray, chief"));
        return Ok(());
    };

    eprintln!("lore config successfully deserialized");

    match Fuzzer::new(Path::new(&lore_dir)) {
        Ok(fuzzer) => {
            eprintln!("fuzzer initialized with directory: '{}'", &lore_dir);
            app.manage(fuzzer);
        }
        Err(e) => {
            eprintln!("fuzzer failed to load due to error {}", e);
            app.manage(Fuzzer::default());
        }
    };

    Ok(())
}
