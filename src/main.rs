//use std::env;
#[macro_use]
extern crate clap;
extern crate ammonia;
extern crate chrono;
extern crate rayon;
extern crate epub;
extern crate tantivy;
extern crate walkdir;
extern crate rusqlite;
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
extern crate urlencoding;

use clap::{App, Arg};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process;

use server::Server;

use crate::sqlite::SqlWriter;

mod error;
mod scanner;
mod search_result;
mod server;
mod test;
mod ttvy;
mod sqlite;

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
			moddate: "2021-01-21T10:56:30+01:00".to_string(),
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
	pub fn add_tags(&self, tags: &mut HashMap<String, u32>) {
		//add any known tags
		if self.subject.is_some() {
			for tag in self.subject.as_ref().unwrap() {
				tags.insert(tag.to_string(), tags.get(tag).unwrap_or(&0)+1);
			}
		}
	}

	pub fn add_counts(val:&Option<String>, counts: &mut HashMap<String, u32>) {
		if let Some(cat) = val {
			counts.insert(cat.to_string(), counts.get(cat).unwrap_or(&0)+1);
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
	fn write_epubs(&mut self, bms: &Vec<BookMetadata>) -> Result<(), Box<dyn Error>>;
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
		.arg(Arg::with_name("threads")
		    .short("t")
			.long("threads")
			.help("Number of threads to use for scanning. Default is same as number of logical CPUs.")
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
					false,
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

	let db_dir = value_t!(matches, "dbfile", String).unwrap_or_else(|_| ".shelfcontrol".to_string());

	let writer: Box<dyn BookWriter + Sync + Send> = match matches.value_of("db").unwrap_or("tantivy") {
		"tantivy" => Box::new(match ttvy::TantivyWriter::new(&db_dir) {
			Ok(writer) => Ok(writer),
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:"))),
		}?),
		_ => process::exit(2),
	};

	let sqlite_writer = SqlWriter::new(&format!("{}/counts.sqlite", &db_dir))?;

	let dirs = values_t!(matches.values_of("directory"), String).unwrap_or_else(|_| vec![".".to_string()]);

	if matches.is_present("threads") {
		let pool_size = value_t!(matches, "threads", usize).unwrap();
		rayon::ThreadPoolBuilder::new().num_threads(pool_size).build_global()?
	}

	scanner::scan_dirs(dirs, coverdir, use_coverdir, writer, sqlite_writer)
}
