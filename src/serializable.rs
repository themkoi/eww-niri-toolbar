use std::path::Path;

use crate::cache::*;
use crate::{config::SortingMode, State};
use freedesktop_desktop_entry::{default_paths, get_languages_from_env, DesktopEntry, Iter};
use freedesktop_icons::lookup;
use icon::Icons;
use serde::Serialize;

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

pub fn get_icon_desktop_fallback(
    app_name: &str,
    icon_theme: &str,
    icon_size: u16,
) -> Option<String> {
    println!(" what");
    let locales = get_languages_from_env();
    let paths = Iter::new(default_paths());
    println!(" ste");

    for path in paths {
        if let Ok(entry) = DesktopEntry::from_path(path, Some(&locales)) {
            if let Some(name) = entry.name(&locales) {
                println!(" searching desktop");
                if name == app_name {
                    // Try to get the icon from the .desktop file
                    let icon_name = entry.icon().unwrap_or_default();
                    let mut icon_p = lookup(icon_name)
                        .with_theme(icon_theme)
                        .with_size(icon_size)
                        .with_cache()
                        .find();

                    if icon_p.is_none() {
                        let icons = Icons::new();
                        icons.find_standalone_icon(icon_name).map(|icon| {
                            icon_p = Some(icon.path().to_path_buf());
                        });
                    }

                    if let Some(icon_path) = icon_p {
                        return Some(icon_path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    None
}

impl SerializableState {
    pub fn from_parts(
        state: &State,
        icon_size: &u16,
        icon_theme: &String,
        seperate_workspaces: &bool,
        sorting_mode: &SortingMode,
        icon_cache: &mut CacheMap,
        check_cache_validity: &bool,
    ) -> Self {
        let mut workspaces_map = std::collections::BTreeMap::<u64, Workspace>::new();
        let mut cache_changed = false;
        for win in &state.windows {
            let icon_name = win.app_id.as_deref().unwrap_or("application-default-icon");
            let mut icon_path = String::new();
            let mut run_lookup = true;

            if let Some(cache_date) =
                icon_cache.get(win.app_id.as_deref().unwrap_or("application-default-icon"))
            {
                icon_path = cache_date.icon_path.clone();

                if *check_cache_validity && Path::new(&cache_date.icon_path).exists() {
                    run_lookup = false; // cache is valid, no need to run lookup
                }
            }

            if run_lookup {
                let mut icon = lookup(icon_name)
                    .with_cache()
                    .with_size(*icon_size)
                    .with_theme(&icon_theme)
                    .find();

                icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
                let lowercase_icon_name = icon_name.to_lowercase();

                if icon_path.is_empty() {
                    icon = lookup(&lowercase_icon_name)
                        .with_size(*icon_size)
                        .with_cache()
                        .with_theme(&icon_theme)
                        .find();

                    icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
                }

                if icon_path.is_empty() {
                    let icon_name = lowercase_icon_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("application-default-icon");

                    icon = lookup(icon_name)
                        .with_cache()
                        .with_size(*icon_size)
                        .with_theme(&icon_theme)
                        .find();

                    icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
                }

                if icon_path.is_empty() {
                    let icon_name = lowercase_icon_name
                        .split('*')
                        .next()
                        .unwrap_or("application-default-icon");

                    icon = lookup(icon_name)
                        .with_size(*icon_size)
                        .with_cache()
                        .with_theme(&icon_theme)
                        .find();

                    icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
                }

                if icon_path.is_empty() {
                    let icons = Icons::new();
                    icons.find_standalone_icon(icon_name).map(|icon| {
                        icon_path = icon.path().to_string_lossy().to_string();
                    });
                }

                if icon_path.is_empty() {
                    icon_path =
                        get_icon_desktop_fallback(icon_name, &*icon_theme, *icon_size).unwrap_or_default();
                }

                if icon_path.is_empty() {
                    icon = lookup("application-x-executable")
                        .with_size(*icon_size)
                        .with_cache()
                        .with_theme(&icon_theme)
                        .find();

                    icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
                }

                set_path(
                    icon_cache,
                    win.app_id.as_deref().unwrap_or("application-default-icon"),
                    &icon_path,
                );

                cache_changed = true;
            }

            let window = Window {
                id: win.id,
                app_id: win.app_id.clone().unwrap_or_else(|| "unknown".to_string()),
                title: win.title.clone().unwrap_or_default(),
                icon_path,
                is_focused: win.is_focused,
            };

            let ws_id = if *seperate_workspaces {
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
        workspaces.sort_by_key(|ws| ws.id); // always sort workspaces by id

        for ws in &mut workspaces {
            match sorting_mode {
                SortingMode::Default => {}
                SortingMode::AZ => ws.windows.sort_by(|a, b| a.app_id.cmp(&b.app_id)),
                SortingMode::Id => ws.windows.sort_by_key(|w| w.id),
            }
        }
        if cache_changed {
            save_cache(&icon_cache);
        }
        SerializableState { workspaces }
    }
}
