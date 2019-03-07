//use std::env;
#[macro_use]
extern crate clap;
extern crate epub;
extern crate tantivy;
extern crate walkdir;
extern crate chrono;
extern crate ammonia;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate rouille;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate core;

use std::fs;
use std::error::Error;
use std::process;
use std::io;
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use epub::doc::EpubDoc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use clap::{App, Arg};
use std::path::Path;
use walkdir::WalkDir;
use chrono::{DateTime, Local};

mod ttvy;
mod server;

//to embed resources use rust-embed or include_str

#[derive(Debug,Serialize, Deserialize)]
pub struct BookMetadata {
	#[serde(with = "string")]
    id: i64,
    title: Option<String>,
    description: Option<String>,
    publisher: Option<String>,
    creator: Option<String>,
    subject: Option<Vec<String>>, //aka tags
    file: String,
    filesize: i64,
    modtime: i64,
    pubdate: Option<String>,
    moddate: Option<String>,
}


//Javascript can't cope with i64 so we use this for ID field to translate to string
//lifted from https://github.com/serde-rs/json/issues/329
mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Serializer, Deserialize, Deserializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
        where T: Display,
              S: Serializer
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where T: FromStr,
              T::Err: Display,
              D: Deserializer<'de>
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
                    tags.get_mut(&tag).unwrap().push(self.id.clone());
                } else {
                    let mut val = Vec::new();
                    val.push(self.id.clone());
                    tags.insert(tag.clone(), val);
                }
            }
        }
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
        ).hash(state);
    }
}

trait BookWriter {
    fn write_tags(&self, tags: HashMap<String, Vec<i64>>, limit: usize) -> Result<(), Box<Error>>;
    fn write_epubs(
        &mut self,
        bms: Vec<BookMetadata>,
        tags: &mut HashMap<String, Vec<i64>>,
    ) -> Result<(), Box<Error>>;
	fn commit(&mut self) -> Result<(), Box<Error>>;
}

fn main() -> Result<(), Box<std::error::Error>> {
    let matches = App::new("Epub Indexer")
        .version("0.0.1")
        .author("Barry Corrigan <b.j.corrigan@gmail.com>")
        .about("Indexes epubs into sqlite or tantivy database")
        .arg(Arg::with_name("dbfile")
            .short("D")
            .long("dbfile")
            .value_name("FILE")
            .help("If using sqlite/tantivy backend, where is dbfile to be located. Warning: this will erase any existing DB. Default: repubin.sqlite or repubin.tantivy")
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
		.get_matches();

	if matches.is_present("search") {
		match ttvy::TantivyReader::new( value_t!(matches, "dbfile", String).unwrap_or("repubin.tantivy".to_string())) {
			Ok(reader) => server::serve(reader),
			Err(_) => panic!("Could not read given index.")
		};
	}

    let mut writer: Box<BookWriter> = match matches.value_of("db").unwrap_or("tantivy") {
		//"sqlite" => Box::new(sqlite::SqliteWriter::new( value_t!(matches, "dbfile", String).unwrap_or("repubin.sqlite".to_string()) )? ),
		"tantivy" => Box::new ( match ttvy::TantivyWriter::new( value_t!(matches, "dbfile", String).unwrap_or("repubin.tantivy".to_string())) {
							Ok(writer) => Ok(writer),
							Err(_) =>  Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:")))
						}? ),
		_ => process::exit(2),
	};

	let dirs = values_t!(matches.values_of("directory"), String).unwrap_or(vec![".".to_string()]);

	for directory in &dirs {
		if !Path::new(&directory).exists() {
			println!("Directory {} does not exist.", &directory);
			process::exit(3);
		}
	}

	let mut total_books:u64 = 0;

	for dir in &dirs {
		for entry in WalkDir::new(&dir).into_iter() {
			match entry {
	            Ok(l) => {
					if l.path().display().to_string().ends_with(".epub") {
						total_books+=1;
					}
				},
				Err(_) => process::exit(1),
			}
		}
	}

	println!("{} books to be scanned.", &total_books);

    let mut books = Vec::new();

    //TODO make this a bookkeeping struct
    let mut tags = HashMap::new();
    let mut seen_bookids = HashSet::new();
	let mut wrote:u64 = 0;
    let mut processed:u64 = 0;
	let mut batch_start = SystemTime::now();
	let scan_start = SystemTime::now();

	for dir in &dirs {
		let walker = WalkDir::new(&dir).into_iter();
		for entry in walker {
	        match entry {
	            Ok(l) => {
					if l.path().display().to_string().ends_with(".epub") {
		                match parse_epub(&l.path().display().to_string()) {
		                    Ok(bm) => {
		                        if seen_bookids.insert(bm.id.clone()) {
		                            books.push(bm);
									wrote+=1;
		                        } else {
		                            println!("DUPLICATE: {}", &l.path().display());
		                        }
		                    }
		                    Err(err) => println!("Error with {}: {:?}", &l.path().display(), err),
		                }

                        processed+=1;

		                if processed % 1000 == 0 {
		                    if let Err(e) = writer.write_epubs(books, &mut tags) {
		                        println!("Error writing batch:{}", e);
		                    }

		                    books = Vec::new();
							report_progress(processed, total_books, wrote, batch_start, scan_start);
							batch_start = SystemTime::now();
		                }
					}
	            }
	            Err(_) => process::exit(1),
	        }
		}
	}
    if let Err(e) = writer.write_epubs(books, &mut tags) {
        println!("Error writing batch:{}", e);
    }

    report_progress(processed, total_books, wrote, batch_start, scan_start);
    println!("Scan complete.");
	//we commit only once at the end, this results in one segment which is much faster than 5000 segments
	writer.commit()?;
	println!("Index created and garbage collected");

    writer.write_tags(tags, 10)
}

fn parse_epub(book_loc: &String) -> Result<BookMetadata, Box<Error>> {
    let doc = EpubDoc::new(&book_loc)?;
//	println!("Got doc! {}", &book_loc);
    let metadata = fs::metadata(&book_loc)?;
    let modtime = match metadata.modified().unwrap().duration_since(std::time::UNIX_EPOCH) {
		Ok(t) => t.as_secs() as i64,
		Err(_) => match std::time::UNIX_EPOCH.duration_since(metadata.modified().unwrap()) {
			Ok(t) => (t.as_secs() as i64)*(-1),
			Err(_) => panic!("Impossible time for {}", &book_loc),
		}
	};

    let mut bm = BookMetadata {
        id: 0i64,
        title: get_first_fd("title", &doc.metadata),
        description: get_first_fd("description", &doc.metadata),
        publisher: get_first_fd("publisher", &doc.metadata),
        creator: get_first_fd("creator", &doc.metadata),
        subject: doc.metadata.get("subject").cloned(),
        file: Path::new(&book_loc).canonicalize().unwrap().display().to_string(),
        filesize: metadata.len() as i64,
        modtime: modtime,
        pubdate: get_first_fd("date", &doc.metadata),
        moddate: get_first_fd("date", &doc.metadata),
    };

    bm.id = hash_md(&bm) as i64;

    /*println!(
        "extracted:{} by {}",
        bm.title.clone().unwrap_or("Unknown".to_string()),
        bm.creator.clone().unwrap_or("Unknown".to_string())
    );*/

    Ok(bm)
}

fn get_first_fd(mdfield: &str, md: &HashMap<String, Vec<String>>) -> Option<String> {
    match md.get(mdfield) {
        Some(vec) => Some(vec.get(0).unwrap().clone()),
        None => None,
    }
}

fn hash_md(bm: &BookMetadata) -> u64 {
    let mut s = DefaultHasher::new();
    bm.hash(&mut s);
    s.finish()
}

fn report_progress(processed: u64, total_books: u64, wrote: u64, batch_start: SystemTime, scan_start: SystemTime) {
    match SystemTime::now().duration_since(batch_start) {
        Ok(n) => {
            let millis = n.as_secs()*1000 + n.subsec_millis() as u64;
            let bps = (1000f64 / millis as f64) * 1000 as f64;

            let total_secs = SystemTime::now().duration_since(scan_start).unwrap().as_secs();
            let total_bps = processed as f64 / total_secs as f64 ;
            let est_secs = chrono::Duration::seconds(((total_books - processed) as f64 / total_bps) as i64);

            let local: DateTime<Local> = Local::now();
            let end_time = local.checked_add_signed(est_secs).unwrap();

            println!("Batch rate: {}bps. Wrote {}. Overall {}bps, estimated completion at {}", bps, &wrote, total_bps, end_time.to_rfc2822());
        }
        Err(_) => panic!("Time went backwards."),
    }
}
