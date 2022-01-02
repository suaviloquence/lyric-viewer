use std::{env, borrow::Cow};

#[derive(Clone, Debug)]
pub enum Mode {
    Instant,
		Stream,
		ShowHelp,
}
#[derive(Clone, Debug)]
pub struct Config {
	pub mode: Mode,
	pub url: Cow<'static, str>,
	pub lyric_dir: Cow<'static, str>,
	pub blank_lines: bool,
}

const DEFAULT_CONFIG: Config = Config {
	mode: Mode::ShowHelp,
	url: Cow::Borrowed("localhost:6600"),
	lyric_dir: Cow::Borrowed("$XDG_DATA_HOME/lyrics"),
	blank_lines: false,
};

pub fn print_help() {
	println!("Options:
	--now | -n => Show lyrics for this point in time and exit.
	--stream | -f => Show lyrics in a stream.
	--url <url> | -u <url> => Set the URL for MPD (default: {})
	--dir <dir> | -d <dir> => Set the directory to look for lyric files (default: {})
	--blanklines => Whether to print blank lines (default: {})
	--help | -h | -? => Show this menu.", DEFAULT_CONFIG.url, DEFAULT_CONFIG.lyric_dir, DEFAULT_CONFIG.blank_lines)
}

pub fn parse_args() -> Config {
	let mut args = env::args();
	let _filename = args.next().unwrap();
	
	let mut config = DEFAULT_CONFIG.clone();
	
	config.lyric_dir = Cow::Owned(
		config.lyric_dir.replace("$XDG_DATA_HOME", 
		&env::var("XDG_DATA_HOME").unwrap_or(format!("{}/.local/share", env::var("HOME").unwrap())))
	);
	
	let mut invalid = false;
	
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"--now" | "-n" => config.mode = Mode::Instant,
			"--stream" | "-f" => config.mode = Mode::Stream,
			"--help" | "-h" | "-?" => config.mode = Mode::ShowHelp,
			"--url" | "-u" => match args.next() {
				Some(u) => config.url = Cow::Owned(u),
				None => invalid = true,
			},
			"--dir" | "-d" => match args.next() {
				Some(d) => config.lyric_dir = Cow::Owned(d),
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