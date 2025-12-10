//! `lore_lib` library. This contains the application loading code for lore.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unreachable_code)]

/*
 * Enforcing documentation is less for the sake of the public API (since none of this is going on
 * crates.io) but rather just to make it abundantly clear what's going on and why. I don't want to
 * come back to this one day with absolutely no clue as to what's going on.
 *
 * Making the below modules pub enforces the missing_docs lint on them, which I'm doing to make
 * sure I don't forget to document whatever I end up putting in those.
 */

pub mod config;
pub mod fuzzer;

use config::{AppConfig, CONFIG_PATH, DEFAULT_STORE_PATH};
use fuzzer::{Fuzzer, FuzzerState};
use normpath::PathExt;
use std::fs;

use tauri::{App, Manager, State, Wry};

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

/// Run the application.
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
        app.manage(Fuzzer::new(&DEFAULT_STORE_PATH));
        return Ok(());
    } else if !DEFAULT_STORE_PATH.exists() || !DEFAULT_STORE_PATH.is_dir() {
        fs::create_dir_all(&*DEFAULT_STORE_PATH)?;
    }

    let cfg_str = fs::read_to_string(CONFIG_PATH.as_path())?;
    let cfg = serde_json::from_str::<AppConfig>(&cfg_str)?;
    eprintln!("config loaded\n{:?}", cfg);

    app.manage(Fuzzer::new(cfg.store_dir()));
    eprintln!(
        "fuzzer initialized with directory: '{}'",
        cfg.store_dir().display()
    );

    Ok(())
}
