use std::{
	sync::mpsc::{self, Receiver, RecvTimeoutError},
	thread,
	time::Duration,
};

use crate::{
	cli::Config,
	lyric_parser::{self, Lyric, Lyrics},
	mpd::{self, CurrentSong, MPDClient, State, Status},
};

#[derive(Debug)]
struct Event {
	status: Status,
	song: CurrentSong,
}

// returns None when a song finishes and Some when it is interrupted
fn song_loop(rx: &Receiver<Event>, lyrics: Lyrics, mut elapsed: f64) -> Option<Event> {
	let mut lyrics = lyrics.into_iter();

	let (mut delay, mut line) = loop {
		match lyrics.next() {
			Some(Lyric { min_secs, lyric }) => {
				if min_secs >= elapsed {
					break (min_secs - elapsed, lyric);
				}
			}
			None => return None,
		}
	};

	loop {
		println!("Sleeping for {:?} s", delay);
		match rx.recv_timeout(Duration::from_secs_f64(delay)) {
			Err(RecvTimeoutError::Timeout) => {
				println!("{}", line);

				let lyric = match lyrics.next() {
					Some(l) => l,
					None => return None,
				};
				elapsed += delay;
				delay = lyric.min_secs - elapsed;

				line = lyric.lyric;
			}
			Err(RecvTimeoutError::Disconnected) => panic!("Sender disconnected"),
			Ok(evt) => return Some(evt),
		};
	}
}

fn handle_event(config: &Config, rx: &Receiver<Event>, evt: Event) {
	if evt.status.state == State::PLAY {
		let filename = format!(
			"{}/{} - {}.lrc",
			&config.lyric_dir, evt.song.artist, evt.song.title
		);

		let lyrics = match lyric_parser::load_from_file(&filename, config.blank_lines) {
			Ok(l) => l,
			Err(e) => return eprintln!("Error loading lyrics from {:?} : {:?}", filename, e),
		};

		if let Some(evt) = song_loop(rx, lyrics, evt.status.elapsed) {
			handle_event(config, rx, evt);
		}
	}
}

pub fn run(config: Config) -> mpd::Result<()> {
	let mut client = MPDClient::connect(&config.url)?;

	let (sx, rx) = mpsc::channel();

	sx.send(dbg!(Event {
		status: client.status()?,
		song: client.current_song()?,
	}))
	.expect("Error sending! (main loop)");

	thread::spawn(move || loop {
		let mut handle = || {
			client.idle_player()?;

			let event = Event {
				song: client.current_song()?,
				status: client.status()?,
			};

			sx.send(dbg!(event))
				.expect("Error sending (from idle thread)!");
			Ok(())
		};

		if let Err(e) = handle() as mpd::Result<()> {
			eprintln!("Error while idling: {:?}", e);
		}
	});

	while let Ok(evt) = rx.recv() {
		handle_event(&config, &rx, evt);
	}

	Ok(())
}
