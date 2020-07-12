#[cfg(test)]
mod test {

	use crate::scanner;
	use crate::ttvy;
	use std::fs;
	use std::io::Error;
	use std::{thread, time};

	#[test]
	fn integration_test() -> Result<(), Error> {
		fs::create_dir("target/images")?;

		let writer = ttvy::TantivyWriter::new("target/index".to_string()).unwrap();
		scanner::scan_dirs(["test/library".to_string()].to_vec(), Some("target/images"), true, Box::new(writer)).expect("Scanner failed");

		let reader = ttvy::TantivyReader::new("target/index".to_string()).expect("Reader failed");
		let mut result = reader.search("darwin", 0, 10).expect("Search failed");

		println!("result: {}", result.to_json());

		assert!(result.to_json().contains("\"count\":1, \"position\":0, \"query\":\"darwin\", \"books\":[{\"id\":\"-5302641238507735522\",\"title\":\"The Origin of Species\",\"description\":\"A distinguished amateur scientist lays out the evidence for the origin of species by means of natural selection.\",\"publisher\":\"Standard Ebooks\",\"creator\":\"Charles Darwin\",\"subject\":[\"Evolution (Biology)\",\"Natural selection\"],\"file"));
		assert!(result
			.to_json()
			.contains("charles-darwin_the-origin-of-species.epub\",\"filesize\":94482,\"modtime\""));
		assert!(result
			.to_json()
			.contains("\"pubdate\":\"2019-01-15T04:52:30Z\",\"moddate\":\"2019-01-15T04:52:30Z\",\"cover_mime\":\"image/jpeg\"}]}"));

		result = reader.search("creator:\"Thomas de Quincey\"", 0, 10).expect("Search failed");
		assert!(result.count == 1);
		let book = result.books.get(0).unwrap();
		println!("{}", book.creator.as_ref().unwrap());
		assert!(book.creator.as_ref().unwrap() == "Thomas De Quincey");
		assert!(book.filesize == 115227);

		result = reader.search("*", 0, 10).expect("Search failed");
		println!("Result count: {}", result.count);
		assert!(result.count == 8);

		let bm = reader.get_book(-5302641238507735522).unwrap();
		assert!(bm.creator.as_ref().unwrap() == "Charles Darwin");

		drop(reader);

		//need to sleep else race condition prevents delete
		let second = time::Duration::from_millis(1000);
		thread::sleep(second);
		fs::remove_dir_all("target/images")?;
		fs::remove_dir_all("target/index")
	}
}
