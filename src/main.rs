use std::{thread, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime}, fs::{self, File}, io::{stdin, Write}, path::PathBuf};

use cli::{Config, Mode};

use crate::lyric_parser::Lyric;

mod mpd;
mod lyric_parser;
mod cli;


fn run_instant(config: Config) -> std::io::Result<()> {
	let mut client = mpd::connect(&config.url)?;

	let ret = client.get_command("status")?;
	if ret.get("state").unwrap() == "stop" {
		return Ok(())
	}
	let elapsed_time = ret.get("elapsed").unwrap().parse::<f64>().unwrap();
	
	let song_data = client.get_command("currentsong")?;
	let artist = song_data.get("Artist").unwrap();
	let title = song_data.get("Title").unwrap();
	
	let lyrics = match lyric_parser::load_from_file(&format!("{}/{} - {}.lrc", config.lyric_dir, artist, title), config.blank_lines) {
		Err(_) => return Ok(()),  // TODO - print msg?
		Ok(c) => c,
	};
	print!("{}", lyrics.get_lyric_for_time(elapsed_time).unwrap_or(""));
	Ok(())
}

fn run_stream(config: Config) -> std::io::Result<()> {
	let mut client = mpd::connect(&config.url)?;
	
	loop {
		let stop = Arc::new(AtomicBool::new(false));
		thread::spawn({
			let stop_thread = stop.clone();
			let current_status = client.get_command("status")?;
			let song_data = client.get_command("currentsong")?;
			let lyric_dir = config.lyric_dir.clone();
			move || {
				if current_status.get("state").unwrap() != "play" {return}

				let title = song_data.get("Title").unwrap();
				let filename = match song_data.get("Artist") {
					Some(artist) => format!("{}/{} - {}.lrc", lyric_dir, artist, title),
					None => format!("{}/{}.lrc", lyric_dir, title), 	
				};
				
				let mut secs = current_status.get("elapsed").unwrap().parse::<f64>().unwrap();

				let lyrics = match lyric_parser::load_from_file(&filename, config.blank_lines) {
					Ok(l) => l,
					Err(_) => return,
				};

				let mut iter = lyrics.into_iter();
				while let Some(next) = iter.next() {
					if secs > next.min_secs {continue}

					while next.min_secs - secs > 0.001 {
						if stop_thread.load(Ordering::SeqCst) {return}
						let dur = Duration::from_millis(200).min(Duration::from_secs_f64(next.min_secs - secs));
						thread::sleep(dur);
						secs += dur.as_secs_f64();
					}
					println!("{}", next.lyric);
				}
			}});
			client.idle_player()?;
			stop.store(true, Ordering::SeqCst);
		}
}


fn run_sync(config: Config) -> std::io::Result<()> {
	let mut client = mpd::connect(&config.url)?;
	let unsynced_lyrics = fs::read_to_string(&config.unsynced_filename.unwrap())?;
	let current_song = client.get_command("currentsong")?;
	
	let title = current_song.get("Title").unwrap();
	let artist = current_song.get("Artist").unwrap();
	
	println!("Playing {} - {} -- Press ENTER to start", artist, title);

	let mut buf = String::new();
	stdin().read_line(&mut buf)?;
	
	println!("Press ENTER when the line displayed on screen begins");
	client.run_command("stop")?;  // TODO -- check for OK response
	client.run_command("play")?;
	
	let mut data = Vec::<Lyric>::new();

	let start = SystemTime::now();
	for line in unsynced_lyrics.lines() {
		if line == "" && !config.blank_lines {continue}
		println!("{}", line);
		stdin().read_line(&mut buf)?;
		data.push(Lyric {lyric: line.to_owned(), min_secs: SystemTime::now().duration_since(start).unwrap().as_secs_f64()});
		lyric_parser::seconds_to_timestr(&data.last().unwrap().min_secs);
	}
	
	let lyrics = lyric_parser::load_from_data(data);
	let mut filename = PathBuf::from(config.lyric_dir.to_string());
	filename.push(format!("{} - {}.lrc", artist, title));
	
	let mut file = File::create(filename)?;
	write!(&mut file, "{}", lyrics.to_string())?;
	
	
	Ok(())
}
fn main() -> std::io::Result<()> {
	let config = cli::parse_args();
	
	match config.mode {
		Mode::ShowHelp => cli::print_help(),
		Mode::Stream => run_stream(config)?,
		Mode::Instant => run_instant(config)?,
		Mode::Sync => run_sync(config)?,
	};
		
	Ok(())
}