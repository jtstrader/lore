use std::sync::Arc;

use nucleo::{Config, Nucleo};

// TODO: Will likely need to return a preview of the image data. Should the frontend handle this?
#[tauri::command]
fn search(key: &str, is_append: bool) -> Vec<String> {
    const CANDIDATES: &[&str] = &[
        "foo/bar", "foo bar", "potato", "sandwich", "flu bar", "foo bot", "fubar", "yaks",
    ];

    let mut nucleo = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);

    CANDIDATES.into_iter().for_each(|&x| {
        nucleo.injector().push(x, |data, cols| {
            cols[0] = (*data).into();
        });
    });

    nucleo.pattern.reparse(
        0,
        key,
        nucleo::pattern::CaseMatching::Ignore,
        nucleo::pattern::Normalization::Never,
        is_append,
    );

    nucleo.tick(10);

    nucleo
        .snapshot()
        .matched_items(..)
        .into_iter()
        .map(|item| *item.data)
        .map(String::from)
        .collect::<Vec<_>>()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
