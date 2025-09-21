use crate::{config::SortingMode, State};
use freedesktop_icons::lookup;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct SerializableState {
    workspaces: Vec<Workspace>,
}

#[derive(Serialize)]
struct Workspace {
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

impl SerializableState {
    pub fn from_parts(
        state: &State,
        icon_size: &u16,
        icon_theme: &String,
        seperate_workspaces: &bool,
        sorting_mode: &SortingMode,
    ) -> Self {
        let mut workspaces_map = std::collections::BTreeMap::<u64, Workspace>::new();

        for win in &state.windows {
            let icon_name = win.app_id.as_deref().unwrap_or("application-default-icon");
            let mut icon = lookup(icon_name)
                .with_cache()
                .with_size(*icon_size)
                .with_theme(&icon_theme)
                .find();

            let mut icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
            if icon_path.is_empty() {
                let lowercase_icon_name = icon_name.to_lowercase();
                icon = lookup(&lowercase_icon_name)
                    .with_size(*icon_size)
                    .with_cache()
                    .with_theme(&icon_theme)
                    .find();

                icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
            }

            if icon_path.is_empty() {
                icon = lookup("application-x-executable")
                    .with_size(*icon_size)
                    .with_cache()
                    .with_theme(&icon_theme)
                    .find();
                icon_path = icon.unwrap_or_default().to_string_lossy().to_string();
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

        SerializableState { workspaces }
    }
}
