use std::env;

#[derive(Clone, Debug)]
pub enum Mode {
	Instant,
	Stream,
	ShowHelp { program_name: String },
	Sync { unsynced_filename: String },
}
#[derive(Clone, Debug)]
pub struct Config {
	pub mode: Mode,
	pub url: String,
	pub lyric_dir: String,
	pub blank_lines: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			mode: Mode::ShowHelp {
				program_name: "lyric_viewer".to_string(),
			},
			url: "localhost:6600".to_string(),
			lyric_dir: format!(
				"{}/lyrics",
				env::var("XDG_DATA_HOME").unwrap_or(format!(
					"{}/.local/share",
					env::var("HOME").expect("Error getting home directory.")
				))
			),
			blank_lines: false,
		}
	}
}

pub fn print_help(program_name: String) {
	let Config {
		url,
		lyric_dir,
		blank_lines,
		..
	} = Config::default();

	println!(
		"{} <mode> [options]: sync lyrics to MPD

Modes:
\tshow: Show lyrics for this point in time and exit
\tstream: Show lyrics in real time as a stream
\tsync <filename>: Create a synced lyric file from a file <filename> containing unsynced lyrics
\thelp: Show this help menu
Options:
\t--url <url> | -u <url>: Set the URL for MPD (default: {})
\t--dir <dir> | -d <dir>: Set the directory to look for synced lyric files (default: {})
\t--blank-lines: Whether to print blank lines (default: {})",
		program_name, url, lyric_dir, blank_lines
	)
}

pub fn parse_args() -> Config {
	let mut args = env::args();
	let help = Mode::ShowHelp {
		program_name: args.next().unwrap_or("lyric_viewer".into()),
	};

	let Config {
		mut url,
		mut lyric_dir,
		mut blank_lines,
		..
	} = Config::default();

	let mut mode = match args.next().as_deref() {
		Some("show") => Some(Mode::Instant),
		Some("stream") => Some(Mode::Stream),
		Some("sync") => match args.next() {
			Some(unsynced_filename) => Some(Mode::Sync { unsynced_filename }),
			_ => None,
		},
		_ => None,
	};

	if loop {
		match args.next().as_deref() {
			Some("--dir") | Some("-d") => {
				lyric_dir = match args.next() {
					Some(l) => l,
					None => break true,
				}
			}
			Some("--blank-lines") => blank_lines = true,
			Some("--url") | Some("-u") => {
				url = match args.next() {
					Some(u) => u,
					None => break true,
				}
			}
			Some(_) => break true,
			None => break false,
		};
	} {
		mode = None;
	}
	Config {
		mode: mode.unwrap_or(help),
		url,
		lyric_dir,
		blank_lines,
	}
}
