use niri_ipc::{socket::Socket, Event, Reply, Request, Response, Window, Workspace};
use std::io;

use crate::config::SortingMode;

mod serializable;

mod niri_socket;

mod config;

fn main() {
    let config = config::load_or_create_config().unwrap();
    let mut state = State::new();
    let niri_socket_env = std::env::var("NIRI_SOCKET");
    let mut socket = if let Ok(niri_socket) = niri_socket_env {
        Socket::connect_to(niri_socket).unwrap()
    } else {
        Socket::connect().unwrap()
    };
    let reply = socket.send(Request::EventStream).unwrap();
    if matches!(reply, Ok(Response::Handled)) {
        let response = socket.send(Request::Windows);
        let mut read_event = socket.read_events(); // ownership moves here
        while let Ok(event) = read_event() {
            state.update_with_event(event);
            let serializable_state = serializable::SerializableState::from_parts(&state,&config.general.icon_size,&config.general.icon_theme,&config.general.seperate_workspaces,&config.general.sorting_mode);
            let json = serde_json::to_string(&serializable_state).unwrap();
            println!("{}", json);
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
    fn update_with_event(&mut self, e: Event) {
        match e {
            Event::WorkspacesChanged { .. } => {}
            Event::WorkspaceActivated { .. } => {}
            Event::WorkspaceActiveWindowChanged { .. } => {}
            Event::WindowsChanged { windows } => self.windows = windows,
            Event::WindowOpenedOrChanged { .. } => {
                let niri_socket = niri_socket::NiriSocket::new();

                let mut niri_socket = match niri_socket {
                    Some(socket) => socket,
                    None => {
                        eprintln!("Failed to connect with Niri instance");
                        return;
                    }
                };
                let windows = niri_socket.list_windows();
                self.windows = windows;
            }
            Event::WindowClosed { .. } => {
                let niri_socket = niri_socket::NiriSocket::new();

                let mut niri_socket = match niri_socket {
                    Some(socket) => socket,
                    None => {
                        eprintln!("Failed to connect with Niri instance");
                        return;
                    }
                };
                let windows = niri_socket.list_windows();
                self.windows = windows;
            }
            Event::WindowFocusChanged { .. } => {
                let niri_socket = niri_socket::NiriSocket::new();

                let mut niri_socket = match niri_socket {
                    Some(socket) => socket,
                    None => {
                        eprintln!("Failed to connect with Niri instance");
                        return;
                    }
                };
                let windows = niri_socket.list_windows();
                self.windows = windows;
            }
            Event::WindowLayoutsChanged { .. } => {}
            Event::KeyboardLayoutsChanged { .. } => { /* Do nothing */ }
            Event::KeyboardLayoutSwitched { .. } => { /* Do nothing */ }
            Event::WorkspaceUrgencyChanged { .. } => { /* Do nothing */ }
            Event::WindowUrgencyChanged { .. } => { /* Do nothing */ }
            Event::OverviewOpenedOrClosed { .. } => { /* Do nothing */ }
            Event::ConfigLoaded { .. } => { /* Do nothing */ }
        }
    }
}
