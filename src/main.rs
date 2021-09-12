//use std::env;
#[macro_use]
extern crate clap;
extern crate ammonia;
extern crate chrono;
extern crate rayon;
extern crate epub;
extern crate tantivy;
extern crate walkdir;
extern crate r2d2;
extern crate r2d2_sqlite;
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

use crate::sqlite::Sqlite;

mod error;
mod scanner;
mod search_result;
mod server;
mod test;
mod ttvy;
mod sqlite;

//to embed resources use rust-embed or include_str

#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct AuthorCount {
	creator: String,
	count:u32,
}

#[derive(Debug, Serialize)]
pub struct PublisherCount {
	publisher: String,
	count:u32,
}

#[derive(Debug, Serialize)]
pub struct TagCount {
	tag: String,
	count:u32,
}

trait DbInfo {
	fn get_table() -> String;
	fn get_pkcol() -> String;
}

impl DbInfo for AuthorCount {
	fn get_table() -> String {
		"authors".to_string()
	}

	fn get_pkcol() -> String {
		"author".to_string()
	}
}

//A navigation category (primarily for opds)
#[derive(Debug, Serialize)]
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

#[test]
fn test_unmangle_tags() {

	let mut testbm = BookMetadata {
		id: 0,
		title: None,
		description: None,
		publisher: None,
		creator: None,
		subject: Some(vec!["Contemporary romance fiction; contemporary romance; contemporary women’s fiction; romance; Small Town & Rural; Women’s Fiction; Opposites attract".to_string()]), //aka tags
		file: "test_file".to_string(),
		filesize: 0,
		modtime: 0,
		pubdate: None,
		moddate: None,
		cover_mime: None,
	};

	let mut tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);

	assert_eq!(7, tagmap.len());

	testbm.subject = Some(vec!["West (AK; CA; CO; HI; ID; MT; NV; UT; WY)".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(1, tagmap.len());

	testbm.subject = Some(vec!["Drew; Nancy (Fictitious Character)".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(1, tagmap.len());

	testbm.subject = Some(vec!["FIC027020  FICTION / Romance / Contemporary; FIC044000  FICTION / Contemporary Women".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(4, tagmap.len());

	testbm.subject = Some(vec!["Fiction / Action & Adventure, Fiction / Fantasy / Epic, Fiction / Fantasy / Historical, Fiction / War & Military".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(6, tagmap.len());

	testbm.subject = Some(vec!["Billionaire Romance; Romantic Heroes Royalty & Aristocrats; Romantic Themes Workplace; New Adult & College Romance; City Life Fiction; Contemporary British Fiction; Coming of Age Fiction; Avery Flynn; Entangled Publishing; Royal Bastard; Royalty Romance; Earl; Romantic Comedy; RomCom; Contemporary Romance; Loudmouth by Avery Flynn; Awkweird by Avery Flynn; Parental Guidance by Avery Flynn; Opposites attract romance; Boss/Employee Romance; Forbidden Romance; Fish out of Water Romance; Instantly Royal; Amara; Stand Alone Romance; Series Romance".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(26, tagmap.len());

	testbm.subject = Some(vec!["Juvenile Fiction / Action & Adventure / General, Juvenile Fiction / Fantasy & Magic, Juvenile Fiction / Science Fiction, Juvenile Fiction / Monsters".to_string()]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(6, tagmap.len());
}

impl BookMetadata {

	pub fn add_tags(&self, tags: &mut HashMap<String, u32>) {
		//add any known tags
		if self.subject.is_some() {
			for subject in self.subject.as_ref().unwrap() {
				let subjectlc = subject.to_ascii_lowercase();
				let subjectlc = subjectlc.trim();
				let semi_count = subjectlc.matches(";").count();
				let comma_count = subjectlc.matches(",").count();
				let slash_count = subjectlc.matches("/").count();

				if semi_count>comma_count && semi_count>slash_count {
					self.split_tags(tags, ";", subjectlc);
					continue;
				} else if comma_count>semi_count && comma_count>slash_count {
					self.split_tags(tags, ",", subjectlc);
					continue;
				} else if slash_count > comma_count && slash_count > semi_count {
					self.split_tags(tags, "/", subjectlc);
					continue;
				} else {
					tags.insert(subjectlc.to_string(), tags.get(subjectlc).unwrap_or(&0)+1);
					continue;
				}
			}
		}
	}

	fn split_tags(&self, tags: &mut HashMap<String, u32>, delimiter: &str, subject: &str) {
		for tag_candidate in subject.split(delimiter) {
			if (tag_candidate.contains(delimiter) && !tag_candidate.contains(")")) 
					   || (!tag_candidate.contains("(") && tag_candidate.contains(")"))
					   || tag_candidate.contains("fictitious character") {
				tags.insert(subject.to_string(), tags.get(subject).unwrap_or(&0)+1);
				return;
			}
		}
		for tag_candidate in subject.split(delimiter) {
			let tag_candidate = tag_candidate.trim();
			tags.insert(tag_candidate.to_string(), tags.get(tag_candidate).unwrap_or(&0)+1);
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

	let db_dir = value_t!(matches, "dbfile", String).unwrap_or_else(|_| ".shelfcontrol".to_string());

	if matches.is_present("search") {
		let sqlite = Sqlite::new(&format!("{}/counts.sqlite", &db_dir))?;
		match ttvy::TantivyReader::new(value_t!(matches, "dbfile", String).unwrap_or_else(|_| ".shelfcontrol".to_string())) {
			Ok(reader) => {
				let server = Server::new(
					reader,
					sqlite,
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

	let writer: Box<dyn BookWriter + Sync + Send> = match matches.value_of("db").unwrap_or("tantivy") {
		"tantivy" => Box::new(match ttvy::TantivyWriter::new(&db_dir) {
			Ok(writer) => Ok(writer),
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:"))),
		}?),
		_ => process::exit(2),
	};

	let dirs = values_t!(matches.values_of("directory"), String).unwrap_or_else(|_| vec![".".to_string()]);

	if matches.is_present("threads") {
		let pool_size = value_t!(matches, "threads", usize).unwrap();
		rayon::ThreadPoolBuilder::new().num_threads(pool_size).build_global()?
	}

	let sqlite = Sqlite::new(&format!("{}/counts.sqlite", &db_dir))?;
	scanner::scan_dirs(dirs, coverdir, use_coverdir, writer, sqlite)
}
