use std::io::{BufRead, Write};

use super::{Error, MPDClient, Result};

#[derive(Debug, PartialEq)]
pub enum State {
	PLAY,
	STOP,
	PAUSE,
}

impl Into<&'static str> for State {
	fn into(self) -> &'static str {
		match self {
			Self::PLAY => "play",
			Self::PAUSE => "pause",
			Self::STOP => "stop",
		}
	}
}

#[derive(Debug)]
pub struct Status {
	pub state: State,
	pub elapsed: f64,
}

#[derive(Debug)]
pub struct CurrentSong {
	pub artist: String,
	pub title: String,
}

macro_rules! take {
	($map: expr, $key: expr, $msg: expr) => {
		$map.remove($key).ok_or(Error::InvalidData($msg))?
	};
}

impl<R: BufRead, W: Write> MPDClient<R, W> {
	pub fn status(&mut self) -> Result<Status> {
		let mut status = self.get_command("status")?;

		let state = match take!(status, "state", "missing state").as_str() {
			"play" => State::PLAY,
			"pause" => State::PAUSE,
			"stop" => State::STOP,
			_ => return Err(Error::InvalidData("state is invalid")),
		};

		let elapsed = match status.remove("elapsed") {
			Some(s) => s,
			None => take!(status, "time", "missing time"),
		}
		.parse()
		.map_err(|_| Error::InvalidData("elapsed is not a number"))?;

		Ok(Status { state, elapsed })
	}

	pub fn current_song(&mut self) -> Result<CurrentSong> {
		let mut song = self.get_command("currentsong")?;

		let artist = take!(song, "Artist", "missing artist");
		let title = take!(song, "Title", "missing title");

		Ok(CurrentSong { artist, title })
	}

	pub fn seek_cur(&mut self, to: f64) -> Result<()> {
		self.run_command(&format!("seekcur {:.3}", to))?;
		self.flush()
	}

	pub fn ping(&mut self) -> Result<()> {
		self.run_command("ping")?;
		self.flush()
	}
}
