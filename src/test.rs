use scanner;
use std::fs;
use std::io::Error;
use std::{thread, time};
use ttvy;

#[test]
fn integration_test() -> Result<(), Error> {
	fs::create_dir("target/images")?;

	let writer = ttvy::TantivyWriter::new("target/index".to_string()).unwrap();
	scanner::scan_dirs(["test/library".to_string()].to_vec(), Some("target/images"), true, Box::new(writer)).expect("Scanner failed");

	let reader = ttvy::TantivyReader::new("target/index".to_string()).expect("Reader failed");
	let result = reader.search("darwin", 0, 10).expect("Search failed");

	assert!(result.contains("The Origin of Species"));

	println!("result: {}", result);

	drop(reader);

	//need to sleep else race condition prevents delete
	let second = time::Duration::from_millis(1000);
	thread::sleep(second);
	fs::remove_dir_all("target/images")?;
	fs::remove_dir_all("target/index")
}
