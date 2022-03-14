use std::{
	collections::VecDeque,
	fs::{self, File},
	io::{self, Write},
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, Ordering},
		mpsc, Arc,
	},
	thread,
	time::{Duration, Instant},
};

use cli::{Config, Mode};

use crate::{
	lyric_parser::Lyric,
	mpd::{CurrentSong, IntoRes, MPDClient, State},
};

mod cli;
mod lyric_parser;
mod mpd;
mod test_lib;

fn run_instant(config: Config) -> mpd::Result<()> {
	let mut client = MPDClient::connect(&config.url)?;

	let ret = client.status()?;

	if ret.state == State::STOP {
		panic!("MPD is not playing.");
	}
	let elapsed_time = ret.elapsed;

	let CurrentSong { artist, title } = client.current_song()?;

	let lyrics = match lyric_parser::load_from_file(
		&format!("{}/{} - {}.lrc", config.lyric_dir, artist, title),
		config.blank_lines,
	) {
		Ok(c) => c,
		Err(_) => panic!("Error loading lyrics from file."),
	};
	print!("{}", lyrics.get_lyric_for_time(elapsed_time).unwrap_or(""));
	Ok(())
}

fn run_stream(config: Config) -> mpd::Result<()> {
	let mut client = MPDClient::connect(&config.url)?;

	loop {
		let stop = Arc::new(AtomicBool::new(false));
		thread::spawn({
			let stop_thread = stop.clone();
			let current_status = client.status()?;
			let song_data = client.current_song()?;
			let lyric_dir = config.lyric_dir.clone();

			move || {
				if current_status.state != State::PLAY {
					return;
				}

				let filename = format!(
					"{}/{} - {}.lrc",
					lyric_dir, song_data.artist, song_data.title
				);

				let mut secs = current_status.elapsed;

				let lyrics = match lyric_parser::load_from_file(&filename, config.blank_lines) {
					Ok(l) => l,
					Err(_) => return,
				};

				let mut iter = lyrics.into_iter();
				while let Some(next) = iter.next() {
					if secs > next.min_secs {
						continue;
					}

					while next.min_secs - secs > 0.001 {
						if stop_thread.load(Ordering::SeqCst) {
							return;
						}
						let dur = Duration::from_millis(200)
							.min(Duration::from_secs_f64(next.min_secs - secs));
						thread::sleep(dur);
						secs += dur.as_secs_f64();
					}
					println!("{}", next.lyric);
				}
			}
		});
		client.idle_player()?;
		stop.store(true, Ordering::SeqCst);
	}
}

fn run_sync(config: Config) -> mpd::Result<()> {
	let mut client = MPDClient::connect(&config.url)?;
	let unsynced_lyrics = fs::read_to_string(&config.unsynced_filename.unwrap())?;
	let current_song = client.current_song()?;

	let title = current_song.title;
	let artist = current_song.artist;

	println!("Playing {} - {} -- Press ENTER to start", artist, title);

	let mut buf = String::new();
	let stdin = io::stdin();

	stdin.read_line(&mut buf)?;

	println!("Press <ENTER> when the line displayed on screen begins");
	println!("Press z<ENTER> to retry the line.");
	println!("Press x<ENTER> to go back 5 seconds.");

	client.seek_cur(0.0)?;

	// keep alive

	thread::spawn({
		let mut client = client.try_clone()?;
		move || loop {
			if let Err(e) = client.ping() {
				eprintln!("Error while idling: {:?}", e);
				break;
			}
			// the real duration is 60, but by then, it'd be cut off ? so -5
			thread::sleep(Duration::from_secs(55)); // TODO find how to find out actual timeout if it's not the default
		}
	});

	let mut data = Vec::<Lyric>::new();

	let mut start = Instant::now();
	let mut min_secs = 0.0;

	let mut lines: VecDeque<_> = unsynced_lyrics.lines().map(&str::to_string).collect();

	while let Some(lyric) = lines.pop_front() {
		buf.clear();
		if lyric == "" && !config.blank_lines {
			continue;
		}
		println!("{}", lyric);
		stdin.read_line(&mut buf)?;

		if buf.trim() == "z" {
			lines.push_front(lyric);
			start = Instant::now();
			min_secs = if let Some(prev) = data.pop() {
				lines.push_front(prev.lyric);
				if let Some(base) = data.last() {
					base.min_secs
				} else {
					0.0
				}
			} else {
				0.0
			};

			client.seek_cur(min_secs)?;
			continue;
		} else if buf.trim() == "x" {
			lines.push_front(lyric);
			min_secs += start.elapsed().as_secs_f64();
			start = Instant::now();
			min_secs = dbg!(0f64.max(min_secs - 5.0));
			client.seek_cur(min_secs)?;
			continue;
		}

		min_secs += start.elapsed().as_secs_f64();
		start = Instant::now();
		data.push(Lyric { lyric, min_secs });
		// println!("{:?}", data);
	}

	let lyrics = lyric_parser::load_from_data(data);
	let mut filename = PathBuf::from(config.lyric_dir.to_string());
	filename.push(format!("{} - {}.lrc", artist, title));

	let mut file = File::create(filename)?;
	write!(&mut file, "{}", lyrics).into_res()
}
fn main() -> mpd::Result<()> {
	let config = cli::parse_args();

	match config.mode {
		Mode::ShowHelp => Ok(cli::print_help()),
		Mode::Stream => run_stream(config),
		Mode::Instant => run_instant(config),
		Mode::Sync => run_sync(config),
	}
}
