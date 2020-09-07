//use std::env;
#[macro_use]
extern crate clap;
extern crate ammonia;
extern crate chrono;
extern crate epub;
extern crate tantivy;
extern crate walkdir;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate rouille;

#[macro_use]
extern crate serde_derive;
extern crate core;
extern crate failure;
extern crate itertools;
extern crate serde;
extern crate serde_json;

use clap::{App, Arg};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process;

use server::Server;

mod error;
mod scanner;
mod search_result;
mod server;
mod test;
mod ttvy;

//to embed resources use rust-embed or include_str

#[derive(Debug, Serialize, Deserialize)]
pub struct BookMetadata {
	#[serde(with = "string")]
	id: i64,
	title: Option<String>,
	description: Option<String>,
	publisher: Option<String>,
	creator: Option<String>,
	subject: Option<Vec<String>>, //aka tags
	#[serde(skip)]
	file: String,
	filesize: i64,
	modtime: i64,
	pubdate: Option<String>,
	moddate: Option<String>,
	cover_mime: Option<String>,
}

//A navigation category (primarily for opds)
#[derive(Debug, Serialize, Deserialize)]
pub struct OpdsCategory {
	id: i64,
	moddate: String,
	title: String,
	url: String,
	icon: Option<String>,
}

impl OpdsCategory {
	fn new(title: String, url: String) -> OpdsCategory {
		OpdsCategory {
			id: 1,
			moddate: "now".to_string(),
			title: title,
			url: url,
			icon: None,
		}
	}
}

//Javascript can't cope with i64 so we use this for ID field to translate to string
//lifted from https://github.com/serde-rs/json/issues/329
mod string {
	use std::fmt::Display;
	use std::str::FromStr;

	use serde::{de, Deserialize, Deserializer, Serializer};

	pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Display,
		S: Serializer,
	{
		serializer.collect_str(value)
	}

	pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
	where
		T: FromStr,
		T::Err: Display,
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
	}
}

impl BookMetadata {
	pub fn add_tags(&self, tags: &mut HashMap<String, Vec<i64>>) {
		//add any known tags
		if self.subject.is_some() {
			for tag in self.subject.clone().unwrap() {
				if tags.contains_key(&tag) {
					tags.get_mut(&tag).unwrap().push(self.id);
				} else {
					let mut val = Vec::new();
					val.push(self.id);
					tags.insert(tag.clone(), val);
				}
			}
		}
	}

	pub fn hash_md(&self) -> i64 {
		let mut s = DefaultHasher::new();
		self.hash(&mut s);
		s.finish() as i64
	}
}

impl Hash for BookMetadata {
	fn hash<H>(&self, state: &mut H)
	where
		H: Hasher,
	{
		(
			&self.title,
			&self.description,
			&self.publisher,
			&self.creator,
			&self.subject,
			&self.filesize,
		)
			.hash(state);
	}
}

pub trait BookWriter {
	fn write_tags(&self, tags: HashMap<String, Vec<i64>>, limit: usize) -> Result<(), Box<dyn Error>>;
	fn write_epubs(&mut self, bms: Vec<BookMetadata>, tags: &mut HashMap<String, Vec<i64>>) -> Result<(), Box<dyn Error>>;
	fn commit(&mut self) -> Result<(), Box<dyn Error>>;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let matches = App::new("Epub Indexer")
        .version("0.0.1")
        .author("Barry Corrigan <b.j.corrigan@gmail.com>")
        .about("Indexes epubs into sqlite or tantivy database")
        .arg(Arg::with_name("dbfile")
            .short("D")
            .long("dbfile")
            .value_name("FILE")
            .help("This is where shelfcontrol settings and index will be located. Warning: this will erase any existing DB. Default: .shelfcontrol")
            .required(false)
            .takes_value(true))
		.arg(Arg::with_name("directory")
			.short("d")
			.long("dir")
			.help("Which directories to scan for books. Multiple directories can be specified. Default: .")
			.multiple(true)
			.takes_value(true))
		.arg(Arg::with_name("db")
			.short("b")
			.long("db")
			.help("Which db backend to use: tantivy or sqlite. Default: tantivy")
			.takes_value(true)
			.required(false))
		.arg(Arg::with_name("search")
			.short("s")
			.long("serve")
			.help("Serve books over http")
			.required(false)
			.takes_value(false))
		.arg(Arg::with_name("coverdir")
			.short("c")
			.long("coverdir")
			.help("Use this directory for cover images. If not specified the server will just extract from epub files on demand, at some performance cost.")
			.takes_value(true)
			.required(false))
		.get_matches();

	if matches.is_present("search") {
		match ttvy::TantivyReader::new(value_t!(matches, "dbfile", String).unwrap_or_else(|_| ".shelfcontrol".to_string())) {
			Ok(reader) => {
				let server = Server::new(
					reader,
					"localhost",
					8000,
					true,
					Some(value_t!(matches, "coverdir", String).unwrap_or_else(|_| ".shelfcontrol/covers".to_string())),
				)?;
				server.serve().expect("Could not start server.");
			}
			Err(e) => panic!("Could not read given index: {}", e),
		};
	}

	let (use_coverdir, coverdir) = match matches.value_of("coverdir") {
		Some(coverdir) => {
			if Path::new(coverdir).exists() {
				(true, Some(coverdir))
			} else {
				eprintln!("Covers directory {} does not exist.", coverdir);
				process::exit(4);
			}
		}
		None => (false, None),
	};

	let writer: Box<dyn BookWriter> = match matches.value_of("db").unwrap_or("tantivy") {
		"tantivy" => Box::new(match ttvy::TantivyWriter::new(
			value_t!(matches, "dbfile", String).unwrap_or_else(|_| ".shelfcontrol".to_string()),
		) {
			Ok(writer) => Ok(writer),
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:"))),
		}?),
		_ => process::exit(2),
	};

	let dirs = values_t!(matches.values_of("directory"), String).unwrap_or_else(|_| vec![".".to_string()]);

	scanner::scan_dirs(dirs, coverdir, use_coverdir, writer)
}
