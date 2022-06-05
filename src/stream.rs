use std::{
	sync::mpsc::{self, Receiver, RecvTimeoutError},
	thread,
	time::Duration,
};

use crate::{
	cli::Config,
	lyric_parser::{self, Lyrics},
	mpd::{self, CurrentSong, MPDClient, State, Status},
};

#[derive(Debug)]
struct Event {
	status: Status,
	song: CurrentSong,
}

struct Stream<'a> {
	rx: &'a Receiver<Event>,
	config: Config,
}

impl<'a> Stream<'a> {
	fn new(rx: &'a Receiver<Event>, config: Config) -> Self {
		Self { rx, config }
	}

	// returns None when a song finishes and Some when it is interrupted
	fn song_loop(&mut self, mut lyrics: Lyrics, mut elapsed: f64) -> Option<Event> {
		let previous = lyrics.get_lyric_for_time(elapsed);

		let mut lyric = match lyrics.next() {
			Some(lyric) => lyric,
			None => return None,
		};

		let mut delay = lyric.min_secs - elapsed;

		if let Some(previous) = previous {
			println!("{}", previous.lyric);
		}

		loop {
			match self.rx.recv_timeout(Duration::from_secs_f64(delay)) {
				Err(RecvTimeoutError::Timeout) => {
					println!("{}", lyric.lyric);

					lyric = match lyrics.next() {
						Some(l) => l,
						None => return None,
					};

					elapsed += delay;
					delay = lyric.min_secs - elapsed;
				}
				Err(RecvTimeoutError::Disconnected) => panic!("Sender disconnected"),
				Ok(evt) => return Some(evt),
			};
		}
	}

	fn handle_event(&mut self, evt: Event) {
		match evt.status.state {
			State::PLAY => {
				let filename = format!(
					"{}/{} - {}.lrc",
					&self.config.lyric_dir, evt.song.artist, evt.song.title
				);

				let lyrics = match lyric_parser::load_from_file(&filename, self.config.blank_lines)
				{
					Ok(l) => l,
					Err(e) => {
						return eprintln!("Error loading lyrics from {:?}: {:?}", filename, e)
					}
				};

				if let Some(evt) = self.song_loop(lyrics, evt.status.elapsed) {
					self.handle_event(evt);
				}
			}
			State::PAUSE => println!("Paused"),
			State::STOP => println!("Stopped"),
		}
	}
}
pub fn run(config: Config) -> mpd::Result<()> {
	let mut client = MPDClient::connect(&config.url)?;

	let (sx, rx) = mpsc::channel();

	sx.send(Event {
		status: client.status()?,
		song: client.current_song()?,
	})
	.expect("Error sending! (main loop)");

	thread::spawn(move || loop {
		let mut handle = || {
			client.idle_player()?;

			let event = Event {
				song: client.current_song()?,
				status: client.status()?,
			};

			sx.send(event).expect("Error sending (from idle thread)!");
			Ok(())
		};

		if let Err(e) = handle() as mpd::Result<()> {
			eprintln!("Error while idling: {:?}", e);
		}
	});

	let mut stream = Stream::new(&rx, config);

	while let Ok(evt) = rx.recv() {
		stream.handle_event(evt);
	}

	Ok(())
}
