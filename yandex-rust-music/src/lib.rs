use pyo3::prelude::*;
use rand::{thread_rng, Rng};

use rodio::source::Source;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

#[derive(Clone)]
pub struct Client {
    tracks: Vec<Track>,
}

impl Client {
    pub fn new(token: &str) -> Self {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let yandex_music_py = PyModule::import(py, "yandex_music").unwrap();
        let client_class_py = yandex_music_py.getattr("Client").unwrap();
        let client_py = client_class_py.call1(((token),)).unwrap();
        client_py.call_method0("init");

        let feed_py = client_py.call_method0("feed").unwrap();
        let generated_playlists_py = feed_py.getattr("generated_playlists").unwrap();
        // TODO: hard-code playlist of the day - update to extract playlist of the day
        let playlist_of_the_day_py = generated_playlists_py.get_item(2).unwrap();
        let playlist_type_py = playlist_of_the_day_py.getattr("type").unwrap();
        println!("Playlist type {}", playlist_type_py);
        let playlist_py = playlist_of_the_day_py.getattr("data").unwrap();
        let track_count_py = playlist_py.getattr("track_count").unwrap();
        let track_count = track_count_py.extract::<usize>().unwrap();
        println!("Track count {}", track_count_py);
        let track_count = 2;

        let mut tracks = Vec::new();
        for i in 0..track_count {
            let tracks_short_py = playlist_py.getattr("tracks").unwrap();
            let track_short_py = tracks_short_py.get_item(i).unwrap();
            let track_py = track_short_py.call_method0("fetch_track").unwrap();
            let title_py = track_py.getattr("title").unwrap();
            let title = title_py.extract::<&str>().unwrap();
            tracks.push(Track {
                title: String::from(title),
                track_py: track_py.into(),
            });
        }

        Self { tracks: tracks }
    }

    pub fn get_random_track(&self) -> &Track {
        let random_track_num = thread_rng().gen_range(0..self.tracks.len());

        &self.tracks[random_track_num]
    }
}

#[derive(Clone)]
pub struct Track {
    title: String,
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
}

#[derive(Clone)]
pub struct Sound {
    total_duration: Duration,
}

impl Sound {
    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }
}

pub struct Player {
    sink: rodio::Sink,
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    current_sound: Option<Sound>,
}

unsafe impl Send for Player {}

impl Player {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        Self {
            sink,
            stream,
            stream_handle,
            current_sound: None,
        }
    }

    pub fn append(&mut self, filename: &str) {
        // Load a sound from a file, using a path relative to Cargo.toml
        let file = BufReader::new(File::open(filename).unwrap());
        // Decode that sound file into a source
        // FIXME:
        // let s10 = Duration::from_secs(10);
        let source = Decoder::new(file).unwrap();
        // self.current_sound = Some(Sound {
        //     total_duration: source
        //         .total_duration()
        //         .expect("Cannot get duration of source"),
        // });
        // println!("Duration {}", source.total_duration().unwrap().as_secs());
        self.sink.append(source);
        if !self.sink.is_paused() {
            self.sink.pause();
        }
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn current_sound(&self) -> Option<&Sound> {
        self.current_sound.as_ref()
    }
}

#[cfg(test)]
mod tests {
    // FIXME: why does not work?
    // use yandex-rust-music::{Client, Track, Downloader, Player};
    // FIXME: What does it mean?
    use super::*;
    #[test]
    fn it_works() {
        let client = Client::new("AQAAAAA59C-DAAG8Xn4u-YGNfkkqnBG_DcwEnjM");
        let track = client.get_random_track();
        println!("Track name = {}", track.title);

        let local_track_path = track.download();

        let mut player = Player::new();
        player.append(&local_track_path);
        player.play();
        println!("Started playing...");
        std::thread::sleep(std::time::Duration::from_secs(10));
        player.pause();
    }

    #[test]
    fn player_can_get_total_duration() {
        let mut player = Player::new();
        player.append("Зол.mp3");
        let duration = player.current_sound().unwrap().total_duration();
        assert!(duration.as_secs() > 0);
    }
}
