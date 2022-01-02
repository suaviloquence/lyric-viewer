use std::{fs, num::ParseFloatError};


#[derive(Debug)]
pub struct Lyric {
	pub min_secs: f64,
	pub lyric: String,
}
#[derive(Debug)]
pub struct Lyrics {
	data: Vec<Lyric>,
}

impl Lyrics {
	pub fn get_lyric_for_time(&self, time: f64) -> Option<&str> {
		let mut result: Option<&str> = None;
		for lyric in &self.data {
			if time >= lyric.min_secs {
				result = Some(&lyric.lyric)
			} else {
				break
			}
		}
		result
	}
}

impl IntoIterator for Lyrics {
	type Item = Lyric;
	type IntoIter = std::vec::IntoIter<Self::Item>;
	fn into_iter(self) -> Self::IntoIter {
		self.data.into_iter()
	}
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
	let mut data = Vec::new();
	
	for line in contents.lines() {
			let mut split = line.split("]");
			let timestr = match split.next() {
				None => continue,
				Some(s) => &s[1..],  // strip leading '['
			};
			let lyric = split.next().unwrap_or("").to_owned();  // TODO - reevaluate default lyric
			if !blank_lines && lyric == "" {continue}
			let min_secs = match timestr_to_seconds(timestr) {
				Ok(s) => s,
				Err(_) => continue, // ignore malformed data
			};
			data.push(Lyric {min_secs, lyric});
	}

	Lyrics {data}
}

pub fn load_from_file(filename: &str, blank_lines: bool) -> std::io::Result<Lyrics> {
	Ok(load(fs::read_to_string(filename)?, blank_lines))
}