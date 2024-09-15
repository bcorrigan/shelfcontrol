#[macro_use]
extern crate clap;
extern crate ammonia;
extern crate epub;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rayon;
extern crate rusqlite;
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
extern crate urlencoding;

use clap::{Parser, Subcommand};
use server::Server;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process;
use time::OffsetDateTime;

use crate::sqlite::Sqlite;

mod error;
mod scanner;
mod search_result;
mod server;
mod sqlite;
mod test;
mod ttvy;

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
	modtime: OffsetDateTime,
	pubdate: Option<String>,
	moddate: Option<String>,
	cover_mime: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct AuthorCount {
	creator: String,
	count: u32,
}

#[derive(Debug, Serialize)]
pub struct PublisherCount {
	publisher: String,
	count: u32,
}

#[derive(Debug, Serialize)]
pub struct TagCount {
	tag: String,
	count: u32,
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

	use serde::Serializer;

	pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Display,
		S: Serializer,
	{
		serializer.collect_str(value)
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
		modtime: OffsetDateTime::now_utc(),
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

	testbm.subject = Some(vec![
		"FIC027020  FICTION / Romance / Contemporary; FIC044000  FICTION / Contemporary Women".to_string(),
	]);
	tagmap = HashMap::new();
	testbm.add_tags(&mut tagmap);
	assert_eq!(4, tagmap.len());

	testbm.subject = Some(vec![
		"Fiction / Action & Adventure, Fiction / Fantasy / Epic, Fiction / Fantasy / Historical, Fiction / War & Military".to_string(),
	]);
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

				if semi_count > comma_count && semi_count > slash_count {
					self.split_tags(tags, ";", subjectlc);
					continue;
				} else if comma_count > semi_count && comma_count > slash_count {
					self.split_tags(tags, ",", subjectlc);
					continue;
				} else if slash_count > comma_count && slash_count > semi_count {
					self.split_tags(tags, "/", subjectlc);
					continue;
				} else {
					tags.insert(subjectlc.to_string(), tags.get(subjectlc).unwrap_or(&0) + 1);
					continue;
				}
			}
		}
	}

	fn split_tags(&self, tags: &mut HashMap<String, u32>, delimiter: &str, subject: &str) {
		for tag_candidate in subject.split(delimiter) {
			if (tag_candidate.contains(delimiter) && !tag_candidate.contains(")"))
				|| (!tag_candidate.contains("(") && tag_candidate.contains(")"))
				|| tag_candidate.contains("fictitious character")
			{
				tags.insert(subject.to_string(), tags.get(subject).unwrap_or(&0) + 1);
				return;
			}
		}
		for tag_candidate in subject.split(delimiter) {
			let tag_candidate = tag_candidate.trim();
			tags.insert(tag_candidate.to_string(), tags.get(tag_candidate).unwrap_or(&0) + 1);
		}
	}

	pub fn add_counts(val: &Option<String>, counts: &mut HashMap<String, u32>) {
		if let Some(cat) = val {
			counts.insert(cat.to_string(), counts.get(cat).unwrap_or(&0) + 1);
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
/// A fast OPDS server and epub indexer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	/// Where the index is
	#[arg(short, long, default_value = ".shelfcontrol")]
	dbFile: String,

	/// If scanning, store covers here. If serving, get covers here. By default, server will extract from epub files directly at some performance cost.
	#[arg(short, long)]
	coverdir: Option<String>,

	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	/// start an OPDS server
	Serve {
		/// Which port to listen on
		#[arg(short, long, default_value_t = 8080)]
		port: u16,

		/// Hostname to bind to
		#[arg(short, long, default_value = "localhost")]
		host: String,
	},

	/// Run the indexer
	Index {
		/// Which directories to scan for books. Multiple directories can be specified.
		#[arg(short, long, default_value = ".", num_args=1.., value_parser)]
		dir: Vec<String>,
	},
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	let db_dir = cli.dbFile;

	let (use_coverdir, coverdir) = match cli.coverdir {
		Some(dir) => {
			if Path::new(&dir).exists() {
				(true, dir)
			} else {
				eprintln!("Covers directory {} does not exist.", &dir);
				process::exit(4);
			}
		}
		None => (false, "".to_string()),
	};

	match cli.command {
		Command::Serve { port, host } => {
			start_server(db_dir, port, host, coverdir, use_coverdir);
		}
		Command::Index { dir } => {
			start_indexer(db_dir, dir, coverdir, use_coverdir);
		}
	}

	Ok(())
}

fn start_indexer(db_dir: String, dirs: Vec<String>, coverdir: String, use_coverdir: bool) {
	let writer: Box<dyn BookWriter + Sync + Send> = match ttvy::TantivyWriter::new(&db_dir) {
		Ok(writer) => Ok(Box::new(writer)),
		Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:"))),
	}
	.expect("Could not create indexer, is the db directory writeable?");

	let sqlite = Sqlite::new(&format!("{}/counts.sqlite", &db_dir)).expect("Could not create sqlite db.");
	scanner::scan_dirs(dirs, coverdir, use_coverdir, writer, sqlite);
}

fn start_server(db_dir: String, port: u16, host: String, coverdir: String, use_coverdir: bool) {
	let sqlite =
		Sqlite::new(&format!("{}/counts.sqlite", &db_dir)).expect("Could not open sqlite db. Check dbFile directory is writeable.");
	match ttvy::TantivyReader::new(db_dir) {
		Ok(reader) => {
			let server = Server::new(reader, sqlite, host, port, use_coverdir, coverdir);
			server.serve().expect("Could not start server. Is port already bound?");
		}
		Err(e) => panic!("Could not read given index: {}", e),
	};
}
