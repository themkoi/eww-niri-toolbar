use std::{
    collections::HashMap,
    fs,
    path::{Path},
};

use crate::cache::*;
use crate::{config::SortingMode, State};

use freedesktop_desktop_entry::{default_paths, get_languages_from_env, DesktopEntry, Iter};
use freedesktop_icons::lookup;

use serde::Serialize;
use once_cell::sync::Lazy;
use rayon::prelude::*;

#[derive(Serialize)]
pub(crate) struct SerializableState {
    pub(crate) workspaces: Vec<Workspace>,
}

#[derive(Serialize)]
pub struct Workspace {
    id: u64,
    windows: Vec<Window>,
}

#[derive(Serialize)]
struct Window {
    id: u64,
    app_id: String,
    title: String,
    icon_path: String,
    is_focused: bool,
}

static DESKTOP_ICON_INDEX: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let locales = get_languages_from_env();
    let mut map = HashMap::new();

    for path in Iter::new(default_paths()) {
        if let Ok(entry) = DesktopEntry::from_path(path.clone(), Some(&locales)) {
            if let Some(icon) = entry.icon() {
                if let Some(stem) = path.file_stem() {
                    map.insert(stem.to_string_lossy().to_lowercase(), icon.to_string());
                }

                if let Some(wm) = entry.startup_wm_class() {
                    map.insert(wm.to_lowercase(), icon.to_string());
                }
            }
        }
    }

    map
});

pub fn get_icon_desktop_fallback(
    app_id: &str,
    icon_theme: &str,
    icon_size: u16,
) -> Option<String> {
    let icon_name = DESKTOP_ICON_INDEX.get(&app_id.to_lowercase())?;

    lookup(icon_name)
        .with_theme(icon_theme)
        .with_size(icon_size)
        .with_cache()
        .find()
        .map(|p| p.to_string_lossy().into_owned())
}

impl SerializableState {
    pub fn from_parts(
        state: &State,
        icon_size: &u16,
        icon_theme: &String,
        separate_workspaces: &bool,
        sorting_mode: &SortingMode,
        icon_cache: &mut CacheMap,
        check_cache_validity: &bool,
    ) -> Self {
        let mut cache_changed = false;

        /* per-run dedup cache */
        let mut resolved: HashMap<String, String> = HashMap::new();

        /* cache dir (ONLY ONCE) */
        let mut cache_folder = get_cache_folder();
        cache_folder.push("icons");
        fs::create_dir_all(&cache_folder).ok();

        let unique_apps: Vec<String> = state
            .windows
            .iter()
            .map(|w| w.app_id.clone().unwrap_or_else(|| "application-default-icon".into()))
            .collect();

        let results: Vec<(String, String)> = unique_apps
            .into_par_iter()
            .map(|app_id| {
                let key = app_id.clone();
                let mut icon_path = String::new();
                let mut run_lookup = true;

                if let Some(cache) = icon_cache.get(&key) {
                    icon_path = cache.icon_path.clone();

                    if *check_cache_validity && Path::new(&icon_path).exists() {
                        run_lookup = false;
                    }
                }

                if run_lookup {
                    let mut icon = lookup(&key)
                        .with_cache()
                        .with_size(*icon_size)
                        .with_theme(icon_theme)
                        .find();

                    icon_path = icon
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();

                    if icon_path.is_empty() {
                        let lower = key.to_lowercase();

                        icon = lookup(&lower)
                            .with_cache()
                            .with_size(*icon_size)
                            .with_theme(icon_theme)
                            .find();

                        icon_path = icon.unwrap_or_default().to_string_lossy().into_owned();
                    }

                    if icon_path.is_empty() {
                        icon_path = get_icon_desktop_fallback(&key, icon_theme, *icon_size)
                            .unwrap_or_default();
                    }

                    if icon_path.is_empty() {
                        icon = lookup("application-x-executable")
                            .with_cache()
                            .with_size(*icon_size)
                            .with_theme(icon_theme)
                            .find();

                        icon_path = icon.unwrap_or_default().to_string_lossy().into_owned();
                    }
                }

                (key, icon_path)
            })
            .collect();

        for (key, path) in &results {
            if !path.is_empty() {
                set_path(icon_cache, key, path);
                cache_changed = true;
            }
            resolved.insert(key.clone(), path.clone());
        }


        let mut workspaces_map = std::collections::BTreeMap::<u64, Workspace>::new();

        for win in &state.windows {
            let key = win
                .app_id
                .clone()
                .unwrap_or_else(|| "application-default-icon".into());

            let icon_path = resolved.get(&key).cloned().unwrap_or_default();

            let window = Window {
                id: win.id,
                app_id: key.clone(),
                title: win.title.clone().unwrap_or_default(),
                icon_path,
                is_focused: win.is_focused,
            };

            let ws_id = if *separate_workspaces {
                win.workspace_id.unwrap_or(0)
            } else {
                0
            };

            workspaces_map
                .entry(ws_id)
                .or_insert_with(|| Workspace {
                    id: ws_id,
                    windows: Vec::new(),
                })
                .windows
                .push(window);
        }


        let mut workspaces: Vec<Workspace> = workspaces_map.into_values().collect();
        workspaces.sort_by_key(|ws| ws.id);

        for ws in &mut workspaces {
            match sorting_mode {
                SortingMode::Default => {}
                SortingMode::AZ => ws.windows.sort_by(|a, b| a.app_id.cmp(&b.app_id)),
                SortingMode::Id => ws.windows.sort_by_key(|w| w.id),
            }
        }

        if cache_changed {
            save_cache(icon_cache);
        }

        SerializableState { workspaces }
    }
}