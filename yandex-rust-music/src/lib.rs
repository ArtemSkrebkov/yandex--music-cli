use pyo3::prelude::*;
use rand::{thread_rng, Rng};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

use eyre::Result;

#[derive(Clone)]
pub struct Client {
    client_py: PyObject,
}

impl Client {
    pub fn new(token: &str) -> Self {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let yandex_music_py = PyModule::import(py, "yandex_music").unwrap();
        let client_class_py = yandex_music_py.getattr("Client").unwrap();
        let client_py = client_class_py.call1(((token),)).unwrap();
        client_py.call_method0("init").unwrap();

        Self {
            client_py: client_py.into(),
        }
    }

    pub fn get_random_track(&self) -> Track {
        let playlist = self.playlist_of_the_day();
        let random_track_num = thread_rng().gen_range(0..playlist.len());

        playlist[random_track_num].clone()
    }

    pub fn playlist_of_the_day(&self) -> Vec<Track> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let clone_client_py = self.client_py.clone_ref(py);
        let ref_client_py = clone_client_py.as_ref(py);

        let feed_py = ref_client_py.call_method0("feed").unwrap();
        let generated_playlists_py = feed_py.getattr("generated_playlists").unwrap();
        // TODO: hard-code playlist of the day - update to extract playlist of the day
        let playlist_of_the_day_py = generated_playlists_py.get_item(2).unwrap();
        let playlist_py = playlist_of_the_day_py.getattr("data").unwrap();
        let track_count_py = playlist_py.getattr("track_count").unwrap();
        let track_count = track_count_py.extract::<usize>().unwrap();

        let mut tracks = Vec::new();
        for i in 0..track_count {
            let tracks_short_py = playlist_py.getattr("tracks").unwrap();
            let track_short_py = tracks_short_py.get_item(i).unwrap();
            let track_py = track_short_py.call_method0("fetch_track").unwrap();

            let title_py = track_py.getattr("title").unwrap();
            let title = title_py.extract::<&str>().unwrap();

            let total_duration_py = track_py.getattr("duration_ms").unwrap();
            let total_duration_ms = total_duration_py.extract::<u64>().unwrap();
            tracks.push(Track {
                title: String::from(title),
                total_duration: Duration::from_millis(total_duration_ms),
                track_py: track_py.into(),
            });
        }

        tracks
    }
}

#[derive(Clone)]
pub struct Track {
    title: String,
    total_duration: Duration,
    track_py: PyObject,
}

impl Track {
    pub fn download(&self) -> String {
        let filename = self.title.clone() + ".mp3";
        let gil = Python::acquire_gil();
        let py = gil.python();
        let clone_track_py = self.track_py.clone_ref(py);
        let ref_track_py = clone_track_py.as_ref(py);
        if !Path::new(&filename).exists() {
            // TODO set bitrait from settings
            ref_track_py
                .call_method("download", (&filename, "mp3", 320), None)
                .unwrap();
        }

        filename
    }

    // FIXME: not going to work this way
    pub async fn download_async(&self) -> String {
        let filename = self.title.clone() + ".mp3";
        let gil = Python::acquire_gil();
        let py = gil.python();
        let clone_track_py = self.track_py.clone_ref(py);
        let ref_track_py = clone_track_py.as_ref(py);
        if !Path::new(&filename).exists() {
            // TODO set bitrait from settings
            let coroutine = ref_track_py
                .call_method("download_async", (&filename, "mp3", 320), None)
                .unwrap();
        }

        filename
    }

    pub fn total_duration(&self) -> Option<Duration> {
        let total_duration = self.total_duration;
        Some(total_duration)
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Status {
    Playing(Instant, Duration),
    Paused(Duration),
    Empty,
}

impl Status {
    pub fn elapsed(self) -> Duration {
        match self {
            Status::Paused(d) => d,
            Status::Playing(start, extra) => start.elapsed() + extra,
            Status::Empty => panic!("Cannot take elapsed time if there is nothing to play"),
        }
    }

    pub fn pause(&mut self) {
        *self = match *self {
            Status::Paused(_) => *self,
            Status::Playing(start, extra) => Status::Paused(start.elapsed() + extra),
            Status::Empty => panic!("Cannot pause a song if there is nothing to play"),
        };
    }

    pub fn play(&mut self) {
        *self = match *self {
            Status::Playing(_, _) => *self,
            Status::Paused(duration) => Status::Playing(Instant::now(), duration),
            Status::Empty => panic!("Cannot play a song if there is nothing to play"),
        };
    }
}

pub struct Player {
    sink: rodio::Sink,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    status: Status,
}

unsafe impl Send for Player {}

impl Player {
    pub fn append(&mut self, filename: &str) {
        // Load a sound from a file, using a path relative to Cargo.toml
        let file = BufReader::new(File::open(filename).unwrap());
        let source = Decoder::new(file).unwrap();
        // FIXME: for some reason
        // we cannot get duration from Source here
        // self.current_sound = Some(Sound {
        //     total_duration: source
        //         .total_duration()
        //         .expect("Cannot get duration of source"),
        // });
        if self.sink.empty() {
            self.status = Status::Paused(Duration::from_secs(0));
        }

        self.sink.append(source);
        if !self.sink.is_paused() {
            self.sink.pause();
        }
    }

    pub fn play(&mut self) {
        if !self.sink.empty() {
            self.sink.play();
            self.status.play();
        } else {
            panic!("There is nothing to play in Player");
        }
    }

    pub fn pause(&mut self) {
        if !self.sink.empty() {
            self.sink.pause();
            self.status.pause();
        } else {
            panic!("There is nothing to pause in Player");
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        *self = Self::default();
    }

    pub fn status(&mut self) -> Result<Status> {
        if self.sink.empty() && self.status != Status::Empty {
            self.status = Status::Empty
        }

        Ok(self.status)
    }
}

impl Default for Player {
    fn default() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        Self {
            sink,
            _stream: stream,
            _stream_handle: stream_handle,
            status: Status::Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn create_client() -> Client {
        let mut token_file = File::open("../token").unwrap();
        let mut token = String::new();
        let _ = token_file.read_to_string(&mut token);
        let _ = token.pop();

        Client::new(&token)
    }

    #[test]
    fn it_works() {
        let client = create_client();
        let track = client.get_random_track();
        println!("Track name = {}", track.title);

        let local_track_path = track.download();

        let mut player = Player::default();
        player.append(&local_track_path);
        player.play();
        let status = player.status();
        assert!(status.is_ok());
        println!("Started playing...");

        std::thread::sleep(std::time::Duration::from_secs(1));

        player.pause();
        let status = player.status();
        assert!(status.is_ok());
    }

    #[test]
    fn track_can_get_total_duration() {
        let client = create_client();
        let track = client.get_random_track();
        let total_duration: Duration = track.total_duration().unwrap();
        assert!(total_duration.as_secs() > 60);
    }

    #[test]
    fn client_can_get_playlist_of_the_day() {
        let client = Client::new("AQAAAAA59C-DAAG8Xn4u-YGNfkkqnBG_DcwEnjM");
        let playlist = client.playlist_of_the_day();

        assert_eq!(playlist.len(), 60);
    }

    #[test]
    fn status_can_pause_and_play() {
        let mut status = Status::Playing(Instant::now(), Duration::from_secs(0));
        status.pause();

        let duration = match status {
            Status::Paused(d) => d,
            _ => panic!("Unexpected value"),
        };

        ::std::thread::sleep(Duration::from_secs(2));
        status.play();

        assert_eq!(status.elapsed().as_secs(), duration.as_secs());
    }

    #[test]
    #[should_panic]
    fn player_can_stop() {
        let client = create_client();
        let track = client.get_random_track();
        println!("Track name = {}", track.title);

        let local_track_path = track.download();

        let mut player = Player::default();
        player.append(&local_track_path);
        let _status = player.play();
        println!("Started playing...");

        std::thread::sleep(std::time::Duration::from_secs(1));
        player.stop();

        player.play();
    }

    #[test]
    fn player_can_get_status() {
        let client = create_client();
        let track = client.get_random_track();
        println!("Track name = {}", track.title);

        let local_track_path = track.download();

        let mut player = Player::default();
        assert_eq!(Status::Empty, player.status().unwrap());

        player.append(&local_track_path);
        player.play();
        let duration = Duration::from_secs(2);

        ::std::thread::sleep(duration);
        player.pause();
        let status = player.status().unwrap();
        assert_eq!(status.elapsed().as_secs(), duration.as_secs());

        let status = player.status().unwrap();
        assert_eq!(status.elapsed().as_secs(), duration.as_secs());
    }
}
