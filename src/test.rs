#[cfg(test)]
mod test {

	use crate::scanner;
	use crate::ttvy;
	use std::fs;
	use std::io::Error;
	use crate::error::StoreError;
	use std::{thread, time};

	struct DirsCleanup;

	impl ::std::ops::Drop for DirsCleanup {
		fn drop(&mut self) {
			fs::remove_dir_all("target/images").unwrap();
			fs::remove_dir_all("target/index").unwrap();
		}
	}

	fn get_reader() -> Result<ttvy::TantivyReader, Error> {
    	fs::create_dir("target/images")?;

    	let writer = ttvy::TantivyWriter::new("target/index".to_string()).unwrap();
    	scanner::scan_dirs(["test/library".to_string()].to_vec(), Some("target/images"), true, Box::new(writer)).expect("Scanner failed");
    	let reader = ttvy::TantivyReader::new("target/index".to_string()).expect("Reader failed");
    	Ok(reader)
	}

	#[test]
	fn integration_test() -> Result<(), Error> {
		let _dirs_cleanup = DirsCleanup;
		let reader = get_reader()?;
		let mut result = reader.search("darwin", 0, 10).expect("Search failed");

		println!("result: {}", result.to_json());

		assert!(result.to_json().contains("\"count\":1, \"position\":0, \"query\":\"darwin\", \"books\":[{\"id\":\"-5302641238507735522\",\"title\":\"The Origin of Species\",\"description\":\"A distinguished amateur scientist lays out the evidence for the origin of species by means of natural selection.\",\"publisher\":\"Standard Ebooks\",\"creator\":\"Charles Darwin\",\"subject\":[\"Evolution (Biology)\",\"Natural selection\"],\"file"));

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
		Ok(())
	}



	#[test]
	fn categorise() -> Result<(), Error> {
		let _dirs_cleanup = DirsCleanup;
		let reader = get_reader()?;
		let cats = reader.categorise("creator", "", Some("*"), 0).expect("Categorisation failed.");

		//cats.categories.iter().for_each(|cat| {
		//	println!("Got category:{} ({})", cat.prefix, cat.count);
		//});

		assert!(cats.count == 6);

		let sum = cats.categories.iter().fold(0, |acc, cat| acc + cat.count);
		assert!(sum == 8); //number of books in each category should add to 8
		
		let prefix_cats = reader.categorise("creator", "C", None, 0).expect("Categorisation with 1 letter prefix failed.");

		//prefix_cats.categories.iter().for_each(|cat| {
		//	println!("Got category:{} ({})", cat.prefix, cat.count);
		//});

		assert!(prefix_cats.count == 1);

		let prefcat = prefix_cats.categories.get(0).expect("Should have a category");
		assert!(prefcat.prefix == "CH");

		let prefix_cats3 = reader.categorise("creator", "CHA", None, 0).expect("Categorisation with 3 letter prefix failed.");
		
		/*prefix_cats3.categories.iter().for_each(|cat| {
			println!("Got category:{} ({})", cat.prefix, cat.count);
		});*/

		assert!(prefix_cats3.count == 1);

		Ok(())
	}
}
