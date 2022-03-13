use std::env;

#[derive(Clone, Debug)]
pub enum Mode {
	Instant,
	Stream,
	ShowHelp,
	Sync,
}
#[derive(Clone, Debug)]
pub struct Config {
	pub mode: Mode,
	pub url: String,
	pub lyric_dir: String,
	pub blank_lines: bool,
	pub unsynced_filename: Option<String>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			mode: Mode::ShowHelp,
			url: "localhost:6600".to_owned(),
			lyric_dir: "$XDG_DATA_HOME/lyrics".to_owned(),
			blank_lines: false,
			unsynced_filename: None,
		}
	}
}

pub fn print_help() {
	let default = Config::default();
	println!(
		"Options:
	--now | -n => Show lyrics for this point in time and exit.
	--stream | -f => Show lyrics in a stream.
	--url <url> | -u <url> => Set the URL for MPD (default: {})
	--dir <dir> | -d <dir> => Set the directory to look for lyric files (default: {})
	--blanklines => Whether to print blank lines (default: {})
	--help | -h | -? => Show this menu.",
		default.url, default.lyric_dir, default.blank_lines
	)
}

pub fn parse_args() -> Config {
	let mut args = env::args();
	args.next(); // skip filename as $0

	let mut config = Config::default();

	config.lyric_dir = config.lyric_dir.replace(
		"$XDG_DATA_HOME",
		&env::var("XDG_DATA_HOME").unwrap_or(format!(
			"{}/.local/share",
			env::var("HOME").expect("Error getting home directory.")
		)),
	);

	let mut invalid = false;

	while let Some(arg) = args.next() {
		match arg.as_str() {
			"--now" | "-n" => config.mode = Mode::Instant,
			"--stream" | "-f" => config.mode = Mode::Stream,
			"--sync" | "-s" => match args.next() {
				Some(f) => {
					config.mode = Mode::Sync;
					config.unsynced_filename = Some(f);
				}
				None => invalid = true,
			},
			"--help" | "-h" | "-?" => config.mode = Mode::ShowHelp,
			"--url" | "-u" => match args.next() {
				Some(u) => config.url = u,
				None => invalid = true,
			},
			"--dir" | "-d" => match args.next() {
				Some(d) => config.lyric_dir = d,
				None => invalid = true,
			},
			"--blanklines" => config.blank_lines = true,
			_ => invalid = true,
		}
	}

	if invalid {
		config.mode = Mode::ShowHelp;
	}

	config
}
