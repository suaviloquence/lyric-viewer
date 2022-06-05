use std::{fmt, fs, io, iter::Peekable, num::ParseFloatError};

#[derive(Debug)]
pub struct Lyric {
	pub min_secs: f64,
	pub lyric: String,
}
impl fmt::Display for Lyric {
	fn fmt(&self, mut f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			&mut f,
			"[{}]{}",
			seconds_to_timestr(&self.min_secs),
			&self.lyric
		)
	}
}

#[derive(Debug)]
pub struct Lyrics {
	data: Peekable<std::vec::IntoIter<Lyric>>,
	previous: Option<Lyric>,
	repeat_count: usize,
}

impl From<Vec<Lyric>> for Lyrics {
	fn from(data: Vec<Lyric>) -> Self {
		Self {
			data: data.into_iter().peekable(),
			previous: None,
			repeat_count: 1,
		}
	}
}

impl Iterator for Lyrics {
	type Item = Lyric;

	fn next(&mut self) -> Option<Self::Item> {
		match self.data.next() {
			Some(Lyric { min_secs, lyric }) => {
				if self.previous.as_ref().map(|l| &l.lyric) == Some(&lyric) {
					self.repeat_count += 1
				} else {
					self.repeat_count = 1
				}

				self.previous = Some(Lyric {
					min_secs,
					lyric: lyric.clone(),
				});

				Some(Lyric {
					min_secs,
					lyric: if self.repeat_count == 1 {
						lyric
					} else {
						format!("{} ({})", lyric, self.repeat_count)
					},
				})
			}
			None => None,
		}
	}
}

impl Lyrics {
	pub fn get_lyric_for_time(&mut self, time: f64) -> Option<Lyric> {
		while let Some(lyric) = self.next() {
			if time >= lyric.min_secs {
				if let Some(next) = self.data.peek() {
					if next.min_secs > time {
						return Some(lyric);
					}
				}
			} else {
				break;
			}
		}
		None
	}

	pub fn write(self, mut stream: &mut impl io::Write) -> io::Result<()> {
		for lyric in self.data {
			write!(&mut stream, "{}\n", lyric)?;
		}

		Ok(())
	}
}

// doesn't work for things over 99 hours... but why would you do that?
pub fn seconds_to_timestr(seconds: &f64) -> String {
	format!(
		"{:02.0}:{:02.0}:{:06.3}",
		(seconds / 3600.0).floor(),
		((seconds % 3600.0) / 60.0).floor(),
		(seconds % 60.0)
	)
}

// won't work for songs with units longer than hours... but that shouldn't matter
fn timestr_to_seconds(timestr: &str) -> Result<f64, ParseFloatError> {
	let mut secs = 0.0;
	let mut i = 1.0;
	for s in timestr.split(":").collect::<Vec<_>>().into_iter().rev() {
		secs += s.parse::<f64>()? * i;
		i *= 60.0;
	}
	Ok(secs)
}

pub fn load(contents: String, blank_lines: bool) -> Lyrics {
	let mut data = Vec::with_capacity(contents.lines().count());

	for line in contents.lines() {
		let index = match line.find(']') {
			Some(i) => i,
			None => continue,
		};

		let split = line.split_at(index);
		let timestr = &split.0[1..]; // strip leading '['
		let lyric = split.1[1..].to_owned(); // strip the ']'

		if !blank_lines && lyric == "" {
			continue;
		}

		let min_secs = match timestr_to_seconds(timestr) {
			Ok(s) => s,
			Err(_) => continue, // ignore malformed data
		};

		data.push(Lyric { min_secs, lyric });
	}

	Lyrics::from(data)
}

pub fn load_from_file(filename: &str, blank_lines: bool) -> std::io::Result<Lyrics> {
	Ok(load(fs::read_to_string(filename)?, blank_lines))
}

pub fn load_from_data(data: Vec<Lyric>) -> Lyrics {
	Lyrics::from(data)
}
