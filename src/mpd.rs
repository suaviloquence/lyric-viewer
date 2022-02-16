use std::{
	collections::HashMap,
	io::{BufRead, BufReader, Error, ErrorKind, Write},
	net::TcpStream,
};

pub struct MPDClient {
	stream: TcpStream,
	reader: BufReader<TcpStream>,
}

impl MPDClient {
	pub fn run_command(&mut self, command: &str) -> std::io::Result<()> {
		write!(&mut self.stream, "{}\n", command)?;
		self.stream.flush()
	}

	pub fn get_command(&mut self, command: &str) -> std::io::Result<HashMap<String, String>> {
		self.run_command(command)?;
		let mut map = HashMap::new();
		for line in (&mut self.reader).lines() {
			let line = line?;
			if line == "OK" {
				break;
			}
			let index = line
				.find(": ")
				.expect("Invalid result from MPD (not key-value format).");
			let key = &line[..index];
			let value = &line[index + 2..]; // 2: length of delimeter
			map.insert(key.to_owned(), value.to_owned());
		}
		Ok(map)
	}

	pub fn idle_player(&mut self) -> std::io::Result<()> {
		self.run_command("idle player")?;
		for line in (&mut self.reader).lines() {
			if line? == "OK" {
				break;
			}
		}
		Ok(())
	}
}

pub fn connect(url: &str) -> std::io::Result<MPDClient> {
	let stream = TcpStream::connect(url)?;
	let reader = BufReader::new(stream.try_clone()?);
	let mut client = MPDClient { stream, reader };

	let mut greeting = String::new();
	if client.reader.read_line(&mut greeting)? == 0 || &greeting[..2] != "OK" {
		return Err(Error::new(
			ErrorKind::InvalidData,
			"Connection to MPD not OK.",
		));
	}

	Ok(client)
}
