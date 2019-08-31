use chrono::{DateTime, Local};
use epub::doc::EpubDoc;
use itertools::Itertools;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::fs::File;

use std::io::Write;
use std::path::Path;
use std::process;
use std::time::SystemTime;
use walkdir::WalkDir;

use BookMetadata;
use BookWriter;

pub fn scan_dirs(
	dirs: Vec<String>,
	coverdir: Option<&str>,
	use_coverdir: bool,
	mut writer: Box<dyn BookWriter>,
) -> Result<(), Box<dyn std::error::Error>> {
	for directory in &dirs {
		if !Path::new(&directory).exists() {
			eprintln!("Directory {} does not exist.", &directory);
			process::exit(3);
		}
	}

	let mut total_books: u64 = 0;

	for dir in &dirs {
		for entry in WalkDir::new(&dir).into_iter() {
			match entry {
				Ok(l) => {
					if l.path().display().to_string().ends_with(".epub") && l.file_type().is_file() {
						total_books += 1;
					}
				}
				Err(_) => process::exit(1),
			}
		}
	}

	println!("{} books to be scanned.", &total_books);

	let mut books = Vec::new();

	//TODO make this a bookkeeping struct
	let mut tags = HashMap::new();
	let mut seen_bookids = HashSet::new();
	let mut wrote: u64 = 0;
	let mut processed: u64 = 0;
	let mut batch_start = SystemTime::now();
	let scan_start = SystemTime::now();

	for dir in &dirs {
		let walker = WalkDir::new(&dir).into_iter();
		for entry in walker {
			match entry {
				Ok(l) => {
					if l.path().display().to_string().ends_with(".epub") && l.file_type().is_file() {
						match parse_epub(&l.path().display().to_string(), use_coverdir, coverdir) {
							Ok(bm) => {
								if seen_bookids.insert(bm.id) {
									books.push(bm);
									wrote += 1;
								} else {
									println!("DUPLICATE: {}", &l.path().display());
								}
							}
							Err(err) => eprintln!("Error with {}: {:?}", &l.path().display(), err),
						}

						processed += 1;

						if processed % 1000 == 0 {
							if let Err(e) = writer.write_epubs(books, &mut tags) {
								eprintln!("Error writing batch:{}", e);
							}

							books = Vec::new();
							report_progress(processed, total_books, wrote, batch_start, scan_start);
							batch_start = SystemTime::now();
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
	if let Err(e) = writer.write_epubs(books, &mut tags) {
		eprintln!("Error writing batch:{}", e);
	}

	report_progress(processed, total_books, wrote, batch_start, scan_start);
	println!("Scan complete.");
	//we commit only once at the end, this results in one segment which is much faster than 5000 segments
	writer.commit()?;
	println!("Index created and garbage collected");

	writer.write_tags(tags, 10)
}

fn parse_epub(book_loc: &str, use_coverdir: bool, coverdir: Option<&str>) -> Result<BookMetadata, Box<dyn Error>> {
	let mut doc = EpubDoc::new(&book_loc)?;
	let metadata = fs::metadata(&book_loc)?;
	let modtime = match metadata
		.modified()
		.unwrap_or(std::time::UNIX_EPOCH)
		.duration_since(std::time::UNIX_EPOCH)
	{
		Ok(t) => t.as_secs() as i64,
		Err(_) => match std::time::UNIX_EPOCH.duration_since(metadata.modified().unwrap_or(std::time::UNIX_EPOCH)) {
			Ok(t) => -(t.as_secs() as i64),
			Err(_) => panic!("Impossible time for {}", &book_loc),
		},
	};

	let cover_img = if use_coverdir { doc.get_cover().ok() } else { None };

	let cover_mime = if use_coverdir {
		match doc.get_cover_id() {
			Ok(cover_id) => doc.get_resource_mime(&cover_id).ok(),
			Err(_) => None,
		}
	} else {
		None
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
		file: file,
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
				let mut file = File::create(format!("{}/{}", coverdir.unwrap(), &bm.id)).or_else(|e| {
					eprintln!("Could not create cover file for {}", &book_loc);
					Err(e)
				})?;
				file.write_all(&cover).or_else(|e| {
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
			let bps = (1000f64 / millis as f64) * 1000_f64;

			let total_secs = SystemTime::now().duration_since(scan_start).unwrap().as_secs();
			let total_bps = processed as f64 / total_secs as f64;
			let est_secs = chrono::Duration::seconds(((total_books - processed) as f64 / total_bps) as i64);

			let local: DateTime<Local> = Local::now();
			let end_time = local.checked_add_signed(est_secs).unwrap();

			println!(
				"Batch rate: {}bps. Wrote {}. Overall {}bps, estimated completion at {}",
				bps,
				&wrote,
				total_bps,
				end_time.to_rfc2822()
			);
		}
		Err(_) => panic!("Time went backwards."),
	}
}
