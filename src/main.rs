use std::{
	fs::{self, File},
	io::{self, Write},
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	thread,
	time::{Duration, SystemTime},
};

use cli::{Config, Mode};

use crate::lyric_parser::Lyric;

mod cli;
mod lyric_parser;
mod mpd;

fn run_instant(config: Config) {
	let mut client = mpd::connect(&config.url).expect("Error connecting to MPD.");

	let ret = client
		.get_command("status")
		.expect("Error communicating with MPD (status command).");

	if ret.get("state").unwrap() == "stop" {
		panic!("MPD is not playing.");
	}
	let elapsed_time = ret.get("elapsed").unwrap().parse::<f64>().unwrap();

	let song_data = client
		.get_command("currentsong")
		.expect("Error communicating with MPD (currentsong command).");
	let artist = song_data.get("Artist").unwrap();
	let title = song_data.get("Title").unwrap();

	let lyrics = match lyric_parser::load_from_file(
		&format!("{}/{} - {}.lrc", config.lyric_dir, artist, title),
		config.blank_lines,
	) {
		Ok(c) => c,
		Err(_) => panic!("Error loading lyrics from file."),
	};
	print!("{}", lyrics.get_lyric_for_time(elapsed_time).unwrap_or(""));
}

fn run_stream(config: Config) {
	let mut client = mpd::connect(&config.url).expect("Error connecting to MPD.");

	loop {
		let stop = Arc::new(AtomicBool::new(false));
		thread::spawn({
			let stop_thread = stop.clone();
			let current_status = client
				.get_command("status")
				.expect("Error communicating with MPD (status command).");
			let song_data = client
				.get_command("currentsong")
				.expect("Error communicating with MPD (currentsong command).");
			let lyric_dir = config.lyric_dir.clone();
			move || {
				if current_status.get("state").unwrap() != "play" {
					return;
				}

				let title = song_data.get("Title").unwrap();
				let filename = match song_data.get("Artist") {
					Some(artist) => format!("{}/{} - {}.lrc", lyric_dir, artist, title),
					None => format!("{}/{}.lrc", lyric_dir, title),
				};

				let mut secs = current_status
					.get("elapsed")
					.unwrap()
					.parse::<f64>()
					.unwrap();

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
		client
			.idle_player()
			.expect("Error communicating with MPD (idle command).");
		stop.store(true, Ordering::SeqCst);
	}
}

fn run_sync(config: Config) {
	let mut client = mpd::connect(&config.url).expect("Error connecting to MPD.");
	let unsynced_lyrics = fs::read_to_string(&config.unsynced_filename.unwrap())
		.expect("Error reading unsynced lyric file.");
	let current_song = client
		.get_command("currentsong")
		.expect("Error communicating with MPD (currentsong command).");

	let title = current_song
		.get("Title")
		.expect("Invalid response from MPD (currentsong doesn't contain title)");
	let artist = current_song
		.get("Artist")
		.expect("Invalid response from MPD (currentsong doesn't contain artist)");

	println!("Playing {} - {} -- Press ENTER to start", artist, title);

	let mut buf = String::new();
	let stdin = io::stdin();

	stdin
		.read_line(&mut buf)
		.expect("Error reading from user input.");

	println!("Press ENTER when the line displayed on screen begins");
	client
		.run_command("stop")
		.expect("Error communicating with MPD (stop command)."); // TODO -- check for OK response
	client
		.run_command("play")
		.expect("Error communicating with MPD (play command).");

	let mut data = Vec::<Lyric>::new();

	let start = SystemTime::now();
	for line in unsynced_lyrics.lines() {
		if line == "" && !config.blank_lines {
			continue;
		}
		println!("{}", line);
		stdin
			.read_line(&mut buf)
			.expect("Error reading user input.");
		data.push(Lyric {
			lyric: line.to_owned(),
			min_secs: SystemTime::now()
				.duration_since(start)
				.unwrap()
				.as_secs_f64(),
		});
		lyric_parser::seconds_to_timestr(&data.last().unwrap().min_secs);
	}

	let lyrics = lyric_parser::load_from_data(data);
	let mut filename = PathBuf::from(config.lyric_dir.to_string());
	filename.push(format!("{} - {}.lrc", artist, title));

	let mut file = File::create(filename).expect("Error creating output file.");
	write!(&mut file, "{}", lyrics.to_string()).expect("Error writing to output file.");
}
fn main() -> std::io::Result<()> {
	let config = cli::parse_args();

	match config.mode {
		Mode::ShowHelp => cli::print_help(),
		Mode::Stream => run_stream(config),
		Mode::Instant => run_instant(config),
		Mode::Sync => run_sync(config),
	};

	Ok(())
}
