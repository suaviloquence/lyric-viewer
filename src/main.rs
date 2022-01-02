use std::{thread, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

mod mpd;
mod lyric_parser;
mod cli;


fn run_instant(config: cli::Config) -> std::io::Result<()> {
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

fn run_stream(config: cli::Config) -> std::io::Result<()> {
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

fn main() -> std::io::Result<()> {
	let config = cli::parse_args();
	
	match config.mode {
		cli::Mode::ShowHelp => cli::print_help(),
		cli::Mode::Stream => run_stream(config)?,
		cli::Mode::Instant => run_instant(config)?,
	};
		
	Ok(())
}