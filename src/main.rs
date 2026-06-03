use log::debug;
use niri_ipc::{socket::Socket, Event, Request, Response, Window};

use crate::config::Config;

use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;

mod serializable;
mod config;
mod cache;

fn main() {
    env_logger::init();

    let config = config::load_or_create_config();
    let mut history = cache::load_cache();
    let mut state = State::new();

    let _ = std::fs::remove_file(config.general.socket_path.clone());

    let listener = UnixListener::bind(config.general.socket_path.clone())
        .expect("failed to create unix socket");

    let clients: Arc<Mutex<Vec<UnixStream>>> =
        Arc::new(Mutex::new(Vec::new()));

    {
        let clients = clients.clone();

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        debug!("new client connected");
                        clients.lock().unwrap().push(stream);
                    }
                    Err(err) => {
                        eprintln!("socket accept error: {err}");
                    }
                }
            }
        });
    }

    let niri_socket_env = std::env::var("NIRI_SOCKET");

    let mut socket = if let Ok(niri_socket) = niri_socket_env {
        Socket::connect_to(niri_socket).unwrap()
    } else {
        Socket::connect().unwrap()
    };

    let reply = socket.send(Request::EventStream).unwrap();

    if matches!(reply, Ok(Response::Handled)) {
        let mut read_event = socket.read_events();

        while let Ok(event) = read_event() {
            state.update_with_event(event, &config);

            let serializable_state =
                serializable::SerializableState::from_parts(
                    &state,
                    &config.general.icon_size,
                    &config.general.icon_theme,
                    &config.general.seperate_workspaces,
                    &config.general.sorting_mode,
                    &mut history,
                    &config.general.check_cache_validity,
                );

            let json = serde_json::to_string(&serializable_state).unwrap();

            let mut clients = clients.lock().unwrap();

            clients.retain_mut(|client| {
                writeln!(client, "{json}").is_ok()
            });
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

    /// https://yalter.github.io/niri/niri_ipc/enum.Event.html
    fn update_with_event(&mut self, e: Event, config: &Config) {
        match e {
            Event::WindowsChanged { windows } => {
                self.windows = windows;
            }

            Event::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    for window in self.windows.iter_mut() {
                        window.is_focused = false;
                    }
                }

                if let Some(app_id) = window.app_id.as_ref() {
                    if config.general.blacklist.contains(app_id) {
                        return;
                    }
                }

                if let Some(w) =
                    self.windows.iter_mut().find(|w| w.id == window.id)
                {
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
                for window in self.windows.iter_mut() {
                    window.is_focused = false;
                }

                if let Some(id) = id {
                    if let Some(window) =
                        self.windows.iter_mut().find(|w| w.id == id)
                    {
                        window.is_focused = true;
                    }
                }
            }

            _ => {}
        }
    }
}