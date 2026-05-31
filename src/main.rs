use log::debug;
use niri_ipc::{socket::Socket, Event, Request, Response, Window};

use crate::config::Config;

mod cache;
mod config;
mod serializable;

use std::{
    time::{Duration, Instant},
};

fn main() {
    env_logger::init();

    let config = config::load_or_create_config();
    let mut history = cache::load_cache();
    let mut state = State::new();

    let niri_socket_env = std::env::var("NIRI_SOCKET");

    let mut socket = if let Ok(niri_socket) = niri_socket_env {
        Socket::connect_to(niri_socket).unwrap()
    } else {
        Socket::connect().unwrap()
    };

    if let Ok(Ok(Response::Windows(windows))) = socket.send(Request::Windows) {
        state.windows = windows;
    }

    let reply = socket.send(Request::EventStream).unwrap();
    if !matches!(reply, Ok(Response::Handled)) {
        return;
    }

    let mut read_event = socket.read_events();

    {
        let serializable_state = serializable::SerializableState::from_parts(
            &state,
            &config.general.icon_size,
            &config.general.icon_theme,
            &config.general.seperate_workspaces,
            &config.general.sorting_mode,
            &mut history,
            &config.general.check_cache_validity,
        );

        let json = serde_json::to_string(&serializable_state).unwrap();
        println!("{}", json);
    }

    let mut last_render = Instant::now();
    const MIN_FRAME_TIME: Duration = Duration::from_millis(16);

    loop {
        match read_event() {
            Ok(event) => {
                state.update_with_event(event, &config);
            }
            Err(_) => {
                break;
            }
        }

        if last_render.elapsed() >= MIN_FRAME_TIME {
            let serializable_state = serializable::SerializableState::from_parts(
                &state,
                &config.general.icon_size,
                &config.general.icon_theme,
                &config.general.seperate_workspaces,
                &config.general.sorting_mode,
                &mut history,
                &config.general.check_cache_validity,
            );

            let json = serde_json::to_string(&serializable_state).unwrap();
            println!("{}", json);

            last_render = Instant::now();
        }
    }
}

#[derive(Debug, Default)]
struct State {
    windows: Vec<Window>,
}

impl State {
    fn new() -> Self {
        Self::default()
    }

    fn update_with_event(&mut self, e: Event, config: &Config) {
        match e {
            Event::WindowsChanged { windows } => {
                self.windows = windows;
            }

            Event::WindowOpenedOrChanged { window } => {
                if let Some(app_id) = window.app_id.as_ref() {
                    if config.general.blacklist.contains(app_id) {
                        return;
                    }
                }

                if window.is_focused {
                    for w in self.windows.iter_mut() {
                        w.is_focused = false;
                    }
                }

                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window.id) {
                    *w = window;
                } else {
                    self.windows.push(window);
                }
            }

            Event::WindowClosed { id } => {
                debug!("removing window (closed) {}", id);
                self.windows.retain(|w| w.id != id);
            }

            Event::WindowFocusChanged { id } => {
                for w in self.windows.iter_mut() {
                    w.is_focused = false;
                }

                if let Some(id) = id {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                        w.is_focused = true;
                    }
                }
            }

            _ => {}
        }
    }
}
