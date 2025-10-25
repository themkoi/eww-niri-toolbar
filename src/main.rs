use niri_ipc::{socket::Socket, Event, Request, Response, Window};

mod serializable;

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
        let mut read_event = socket.read_events(); // ownership moves here
        while let Ok(event) = read_event() {
            state.update_with_event(event);
            let serializable_state = serializable::SerializableState::from_parts(
                &state,
                &config.general.icon_size,
                &config.general.icon_theme,
                &config.general.seperate_workspaces,
                &config.general.sorting_mode,
            );
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
            Event::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    // All other windows become not focused
                    for window in self.windows.iter_mut() {
                        window.is_focused = false;
                    }
                }

                // Change or add window
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window.id) {
                    *w = window;
                } else {
                    self.windows.push(window);
                }
            }
            Event::WindowClosed { id } => {
                self.windows.retain(|w| w.id != id);
            }
            Event::WindowFocusChanged { id } => {
                for window in self.windows.iter_mut() {
                    window.is_focused = false;
                }

                if let Some(id) = id {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.is_focused = true;
                    }
                }
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
