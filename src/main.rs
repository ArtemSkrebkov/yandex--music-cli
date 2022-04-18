use pyo3::prelude::*;
use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, source::Source};

fn main() -> PyResult<()> {
        Python::with_gil(|py| {
                let yandex_music = PyModule::import(py, "yandex_music")?;
                let client_class = yandex_music.getattr("Client").unwrap();
                let client = client_class.call0().unwrap();
                client.call_method0("init")?;
                let tracks = client.call_method1("tracks", (vec!["10994777:1193829", "556959:59721"], ))?;
                let track = tracks.get_item(1)?;
                let artists_name = track.call_method0("artists_name")?;
                let name = artists_name.get_item(0)?;
                println!("{}", name);

                track.call_method("download", ("123.mp3", "mp3", 128), None)?;
                // Get a output stream handle to the default physical sound device
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                // Load a sound from a file, using a path relative to Cargo.toml
                let file = BufReader::new(File::open("123.mp3").unwrap());
                // Decode that sound file into a source
                let source = Decoder::new(file).unwrap();
                // // Play the sound directly on the device
                stream_handle.play_raw(source.convert_samples());
                //
                // The sound plays in a separate audio thread,
                // so we need to keep the main thread alive while it's playing.
                std::thread::sleep(std::time::Duration::from_secs(30));
                Ok(())
            })
}
