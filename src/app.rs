use crate::actions::Action;
use crate::actions::Actions;
use crate::inputs::key::Key;
use crate::io::IoEvent;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

use tui::widgets::ListState;

use log::{debug, error, warn};
use yandex_rust_music::{Client, Player, Track};

#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        duration: Duration,
        total_duration: Duration,
    },
}

impl AppState {
    pub fn initialized(total_duration: &Duration) -> Self {
        let duration = Duration::from_secs(0);
        Self::Initialized {
            duration,
            total_duration: *total_duration,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn update_duration(&mut self, current_duration: Duration) {
        if let Self::Initialized { duration, .. } = self {
            *duration = current_duration;
        }
    }

    pub fn duration(&self) -> Option<&Duration> {
        if let Self::Initialized { duration, .. } = self {
            Some(duration)
        } else {
            None
        }
    }

    pub fn total_duration(&self) -> Option<&Duration> {
        if let Self::Initialized { total_duration, .. } = self {
            Some(total_duration)
        } else {
            None
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init
    }
}

#[derive(Default, Clone)]
pub struct DisplayedTracks {
    pub tracks: Vec<Track>,
    pub state: ListState,
}

impl DisplayedTracks {
    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.tracks = tracks;
        self.state = ListState::default();
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.tracks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tracks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    io_tx: tokio::sync::mpsc::Sender<IoEvent>,
    actions: Actions,
    is_loading: bool,
    state: AppState,
    client: Client,
    player: Player,
    current_playlist: Vec<Track>,
    pub displayed_tracks: DisplayedTracks,
    cur_track_idx: usize,
}

impl App {
    fn create_client() -> Client {
        let mut token_file = File::open("token").unwrap();
        let mut token = String::new();
        let _ = token_file.read_to_string(&mut token);
        let _ = token.pop();

        Client::new(&token)
    }

    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let actions = vec![Action::Quit].into();
        let is_loading = false;
        let state = AppState::default();

        let client = Self::create_client();
        let player = Player::default();
        Self {
            io_tx,
            actions,
            is_loading,
            state,
            client,
            player,
            current_playlist: Vec::<Track>::new(),
            displayed_tracks: DisplayedTracks::default(),
            cur_track_idx: 0,
        }
    }

    pub fn initialized(&mut self) {
        self.actions = vec![
            Action::Quit,
            Action::PlaySound,
            Action::PauseSound,
            Action::SelectNextTrack,
            Action::SelectPreviousTrack,
        ]
        .into();
        self.current_playlist = self.client.playlist_of_the_day();
        debug!("Added playlist of the day...");
        self.displayed_tracks
            .set_tracks(self.current_playlist.clone());
        self.displayed_tracks.next();
        let sel_track_idx = self.displayed_tracks.state.selected().unwrap();
        self.cur_track_idx = sel_track_idx;

        let track_ref = &self.displayed_tracks.tracks[sel_track_idx];
        let track_path = track_ref.download();
        self.player.append(&track_path);
        let total_duration = track_ref.total_duration().unwrap();
        self.state = AppState::initialized(&total_duration);
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        if let Ok(status) = self.player.status() {
            self.state.update_duration(status.elapsed());
        }
        AppReturn::Continue
    }

    pub async fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;
        if let Err(e) = self.io_tx.send(action).await {
            self.is_loading = false;
            error!("Error from dispatch {}", e);
        }
    }

    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        if let Some(action) = self.actions.find(key) {
            debug!("Run action [{:?}]", action);
            match action {
                Action::Quit => AppReturn::Exit,
                Action::PlaySound => {
                    let sel_track_idx = self.displayed_tracks.state.selected().unwrap();
                    if self.cur_track_idx != sel_track_idx {
                        self.cur_track_idx = sel_track_idx;
                        self.player.stop();
                        let track_ref = &self.displayed_tracks.tracks[sel_track_idx];
                        let track_path = track_ref.download();
                        self.player.append(&track_path);
                        let total_duration = track_ref.total_duration().unwrap();
                        self.state = AppState::initialized(&total_duration);
                    }
                    let _status = self.player.play();
                    AppReturn::Continue
                }
                Action::PauseSound => {
                    let _status = self.player.pause();
                    AppReturn::Continue
                }
                Action::SelectNextTrack => {
                    self.displayed_tracks.next();
                    AppReturn::Continue
                }
                Action::SelectPreviousTrack => {
                    self.displayed_tracks.previous();
                    AppReturn::Continue
                }
            }
        } else {
            warn!("No action associated to {}", key);
            AppReturn::Continue
        }
    }

    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }

    pub fn current_playlist(&self) -> &Vec<Track> {
        &self.current_playlist
    }
}
