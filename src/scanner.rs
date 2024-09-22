use epub::doc::EpubDoc;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::fs::File;

use crate::sqlite::Sqlite;
use crate::BookWriter;
use crate::{AuthorCount, BookMetadata, PublisherCount, TagCount};
use std::io::Write;
use std::path::Path;
use std::process;
use std::time::SystemTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Duration, OffsetDateTime};
use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
	entry.file_name().to_str().map(|s| !s.starts_with(".")).unwrap_or(false)
}

//TODO move these params to struct & pass struct instead
//TODO Could this be more intelligently parallelised across different devices?
// Scanning is IO bound unless on an nvme ssd or something. Therefore if dir A is on /dev/sdb and dir B is on /mnt/synology,
// scanning across both devices in parrallel can better utilise CPU and get speed ups?
pub fn scan_dirs(
	dirs: Vec<String>,
	coverdir: String,
	use_coverdir: bool,
	mut writer: Box<dyn BookWriter + Send + Sync>,
	sqlite_writer: Sqlite,
) -> Result<(), Box<dyn std::error::Error>> {
	for directory in &dirs {
		if !Path::new(&directory).exists() {
			eprintln!("Directory {} does not exist.", &directory);
			process::exit(3);
		}
	}

	let mut total_books: u64 = 0;

	for dir in &dirs {
		for entry in WalkDir::new(&dir).into_iter().filter_entry(|e| !is_hidden(e)) {
			match entry {
				Ok(l) => {
					if l.file_type().is_file() && l.into_path().ends_with(".epub") {
						total_books += 1;
					}
				}
				Err(_) => process::exit(1),
			}
		}
	}

	println!("{} books to be scanned.", &total_books);

	//TODO make this a bookkeeping struct
	let mut tags = HashMap::new();
	let mut creator_counts = HashMap::new();
	let mut publisher_counts = HashMap::new();
	let seen_bookids = std::sync::RwLock::new(HashSet::new());
	let mut wrote: u64 = 0;
	let errored = std::sync::Mutex::new(0);
	let mut processed: u64 = 0;
	let mut batch_start = SystemTime::now();
	let scan_start = SystemTime::now();
	let mut book_batch = vec![];

	for dir in &dirs {
		let walker = WalkDir::new(&dir).into_iter();
		for entry in walker.filter_entry(|e| !is_hidden(e)) {
			match entry {
				Ok(l) => {
					if l.file_type().is_file() && l.path().starts_with(".epub") {
						book_batch.push(l.path().display().to_string());

						processed += 1;

						if processed % 10000 == 0 || processed >= total_books {
							let bms: Vec<BookMetadata> = book_batch
								.par_iter()
								.map(|book_path| match parse_epub(book_path, use_coverdir, &coverdir) {
									Ok(bm) => {
										if !seen_bookids.read().unwrap().contains(&bm.id) {
											seen_bookids.write().unwrap().insert(bm.id);
											Some(bm)
										} else {
											None
										}
									}
									Err(err) => {
										eprintln!("Error with {}: {:?}", book_path, err);
										let mut error_lock = errored.lock().unwrap();
										*error_lock += 1;
										None
									}
								})
								.filter(|bmo| bmo.is_some())
								.map(|bms| bms.unwrap())
								.collect();

							wrote += bms.len() as u64;

							if let Err(e) = writer.write_epubs(&bms) {
								eprintln!("Error writing batch:{}", e);
							} else {
								for bm in &bms {
									bm.add_tags(&mut tags);
									BookMetadata::add_counts(&bm.creator, &mut creator_counts);
									BookMetadata::add_counts(&bm.publisher, &mut publisher_counts);
								}
								writer.commit()?;
							}

							report_progress(processed, total_books, wrote, batch_start, scan_start);
							batch_start = SystemTime::now();

							book_batch.clear();
						}
					}
				}
				Err(e) => {
					eprintln!("Unrecoverable error while scanning books:{}", e);
					process::exit(1);
				}
			}
		}
	}

	report_final(total_books, wrote, *errored.lock().unwrap(), scan_start);

	println!(
		"Writing counts to sqlite - {} creators, {} publishers, {} tags",
		creator_counts.len(),
		publisher_counts.len(),
		tags.len()
	);
	sqlite_writer.make_db()?;
	sqlite_writer.write_counts::<AuthorCount>(creator_counts)?;
	sqlite_writer.write_counts::<PublisherCount>(publisher_counts)?;
	sqlite_writer.write_counts::<TagCount>(tags)?;

	println!("Scan complete.");
	//we commit only once at the end, this results in one segment which is much faster than 5000 segments
	//writer.commit()?;
	println!("Index created and garbage collected");

	Ok(())
}

fn parse_epub(book_loc: &str, use_coverdir: bool, coverdir: &str) -> Result<BookMetadata, Box<dyn Error>> {
	let mut doc = EpubDoc::new(&book_loc)?;
	let metadata = fs::metadata(&book_loc)?;
	let modtime = metadata.modified().unwrap_or(std::time::UNIX_EPOCH).into();

	let cover_img = if use_coverdir { doc.get_cover() } else { None };

	let cover_mime = match doc.get_cover_id() {
		Some(cover_id) => doc.get_resource_mime(&cover_id),
		None => None,
	};

	let file = match Path::new(&book_loc).canonicalize() {
		Ok(f) => f.display().to_string(),
		Err(e) => {
			eprintln!("Could not canonicalize {}", &e);
			return Err(Box::new(e));
		}
	};

	let mut bm = BookMetadata {
		id: 0i64,
		title: get_first_fd("title", &doc.metadata),
		description: get_first_fd("description", &doc.metadata),
		publisher: get_first_fd("publisher", &doc.metadata),
		creator: get_first_fd("creator", &doc.metadata).map(unmangle_creator),
		subject: doc.metadata.get("subject").cloned(),
		file,
		filesize: metadata.len() as i64,
		modtime,
		pubdate: get_first_fd("date", &doc.metadata),
		moddate: get_first_fd("date", &doc.metadata),
		cover_mime,
	};

	bm.id = bm.hash_md();

	if use_coverdir {
		match cover_img {
			Some(cover) => {
				let mut file = File::create(format!("{}/{}", coverdir, &bm.id)).or_else(|e| {
					eprintln!("Could not create cover file for {}", &book_loc);
					Err(e)
				})?;
				file.write_all(&cover.0).or_else(|e| {
					eprintln!("Error writing to cover dir for {}", &book_loc);
					Err(e)
				})?;
			}
			None => println!("No cover for {}", &bm.file),
		}
	}

	Ok(bm)
}

fn get_first_fd(mdfield: &str, md: &HashMap<String, Vec<String>>) -> Option<String> {
	match md.get(mdfield) {
		Some(vec) => Some(vec.get(0).unwrap().clone()),
		None => None,
	}
}

//Attempt to unmangle author names to be consistent
fn unmangle_creator(creator: String) -> String {
	let unspaced_creator = creator.split_whitespace().join(" ");
	if unspaced_creator.matches(',').count() == 1 {
		let parts: Vec<&str> = unspaced_creator.split(',').collect();
		return format!("{} {}", parts[1].trim(), parts[0].trim());
	}
	unspaced_creator
}

#[test]
fn test_unmangle() {
	let lovecraft = "H.P. Lovecraft".to_string();
	assert_eq!(lovecraft, unmangle_creator(lovecraft.clone()));
	assert_eq!(lovecraft, unmangle_creator("Lovecraft, H.P.".to_string()));
	assert_eq!(lovecraft, unmangle_creator("Lovecraft,  H.P. ".to_string()));
	assert_eq!(lovecraft, unmangle_creator("H.P.  Lovecraft".to_string()));
	assert_eq!(lovecraft, unmangle_creator("H.P. \t  Lovecraft".to_string()));
	assert_eq!(lovecraft, unmangle_creator(" H.P.\t \tLovecraft ".to_string()));
}

fn report_progress(processed: u64, total_books: u64, wrote: u64, batch_start: SystemTime, scan_start: SystemTime) {
	match SystemTime::now().duration_since(batch_start) {
		Ok(n) => {
			let millis = n.as_secs() * 1000 + u64::from(n.subsec_millis());
			let bps = (1000f64 / millis as f64) * 10000_f64;

			//took 10,000 ms to do 1000 books
			//= (1000 / 10000) * 1000 =

			let total_secs = SystemTime::now().duration_since(scan_start).unwrap().as_secs();
			let total_bps = processed as f64 / total_secs as f64;
			let est_secs = ((total_books - processed) as f64 / total_bps) as i64;

			let local = OffsetDateTime::now_utc();
			let end_time = local.checked_add(Duration::seconds(est_secs)).unwrap();

			println!(
				"Batch rate: {}bps. Wrote {}. Overall {}bps, estimated completion at {}",
				bps,
				&wrote,
				total_bps,
				end_time.format(FORMAT).unwrap(),
			);
		}
		Err(_) => panic!("Time went backwards."),
	}
}

const FORMAT: &[BorrowedFormatItem<'_>] = format_description!("[hour]:[minute]:[second]");

fn report_final(total_books: u64, wrote: u64, errored_books: u64, scan_start: SystemTime) {
	match SystemTime::now().duration_since(scan_start) {
		Ok(n) => {
			let millis = n.as_secs() * 1000 + u64::from(n.subsec_millis());
			let actual_bps = (1000f64 / millis as f64) * wrote as f64;
			let processed_bps = (1000f64 / millis as f64) * total_books as f64;
			println!(
				"Completed. Actual: {}bps Total processed: {}bps Total written: {} Errored: {} Duplicates: {} Duration(s): {}",
				actual_bps,
				processed_bps,
				wrote,
				errored_books,
				total_books - wrote - errored_books,
				n.as_secs()
			);
		}
		Err(_) => panic!("Time went backwards."),
	}
}
