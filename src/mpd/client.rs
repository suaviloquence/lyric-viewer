use std::{
	collections::HashMap,
	io::{self, BufRead, BufReader, Write},
	net::TcpStream,
};

#[derive(Debug)]
pub enum Error {
	IO(io::Error),
	ACK(String),
	InvalidData(&'static str),
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Self {
		Self::IO(e)
	}
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait IntoRes<T> {
	fn into_res(self) -> Result<T>;
}

impl<T> IntoRes<T> for io::Result<T> {
	fn into_res(self) -> Result<T> {
		self.map_err(Error::from)
	}
}

pub struct MPDClient<R: BufRead, W: Write> {
	reader: R,
	writer: W,
}

impl<R: BufRead, W: Write> MPDClient<R, W> {
	pub fn run_command(&mut self, command: &str) -> Result<()> {
		write!(&mut self.writer, "{}\n", command)?;
		self.writer.flush().into_res()
	}

	pub fn read_until_ok(&mut self) -> Result<Vec<String>> {
		let mut buf = Vec::new();

		for line in (&mut self.reader).lines() {
			let line = line?;
			if line == "OK" {
				break;
			} else if let Some(ack) = line.strip_prefix("ACK") {
				return Err(Error::ACK(ack.to_owned()));
			}

			buf.push(line)
		}

		Ok(buf)
	}

	pub fn get_command(&mut self, command: &str) -> Result<HashMap<String, String>> {
		self.run_command(command)?;
		let mut map = HashMap::new();

		const DELIMITER: &'static str = ": ";
		for line in self.read_until_ok()? {
			let index = line
				.find(DELIMITER)
				.expect("Invalid result from MPD (not key: value format).");
			let (key, value) = line.split_at(index);
			// delimiter is two characters long
			map.insert(key.to_owned(), value[DELIMITER.len()..].to_owned());
		}
		Ok(map)
	}

	pub fn flush(&mut self) -> Result<()> {
		self.read_until_ok().map(|_| ())
	}

	pub fn idle_player(&mut self) -> Result<()> {
		self.run_command("idle player")?;
		self.flush()
	}
}

impl MPDClient<BufReader<TcpStream>, TcpStream> {
	pub fn connect(url: &str) -> Result<Self> {
		let stream = TcpStream::connect(url)?;
		let reader = BufReader::new(stream.try_clone()?);
		let mut client = MPDClient {
			writer: stream,
			reader,
		};

		let mut greeting = String::new();
		if client.reader.read_line(&mut greeting)? == 0 || &greeting[..2] != "OK" {
			return Err(Error::ACK(greeting));
		}

		Ok(client)
	}

	pub fn try_clone(&self) -> Result<Self> {
		Ok(Self {
			reader: BufReader::new(self.writer.try_clone()?),
			writer: self.writer.try_clone()?,
		})
	}
}
