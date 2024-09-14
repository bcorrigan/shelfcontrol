#[cfg(test)]
mod test {

	use crate::scanner;
	use crate::ttvy;
	use crate::Sqlite;
	use serial_test::serial;
	use std::fs;
	use std::io::Error;
	use std::{thread, time};

	struct DirsCleanup;

	impl ::std::ops::Drop for DirsCleanup {
		fn drop(&mut self) {
			tidy();
		}
	}

	fn tidy() {
		fs::remove_dir_all("target/images");
		fs::remove_dir_all("target/index");
	}

	fn get_reader() -> Result<ttvy::TantivyReader, Error> {
		tidy();
		fs::create_dir("target/images")?;
		//fs::create_dir("target/index")?;
		let db_dir = &"target/index".to_string();
		let writer = ttvy::TantivyWriter::new(db_dir).unwrap();
		let sql_writer = Sqlite::new(&format!("{}/counts.sqlite", &db_dir)).unwrap();
		scanner::scan_dirs(
			["test/library".to_string()].to_vec(),
			"target/images".to_string(),
			true,
			Box::new(writer),
			sql_writer,
		)
		.expect("Scanner failed");
		let reader = ttvy::TantivyReader::new("target/index".to_string()).expect("Reader failed");
		Ok(reader)
	}

	#[test]
	#[serial]
	fn integration_test() -> Result<(), Error> {
		tidy();
		let _dirs_cleanup = DirsCleanup;
		let reader = get_reader()?;
		let mut result = reader.search("darwin", 0, 10).expect("Search failed");

		println!("result: {}", result.to_json());
		assert!(result.to_json().contains("\"count\":1,"));

		assert!(result.to_json().contains("\"query\":\"darwin\","));
		assert!(result.to_json().contains("\"id\":\"-5302641238507735522\","));
		assert!(result.to_json().contains("distinguished amateur"));
		assert!(result.to_json().contains("\"creator\":\"Charles Darwin\","));
		assert!(result.to_json().contains("\"evolution (biology)\"")); //should this be in Caps??
		assert!(result.to_json().contains("\"subject\":"));
		assert!(result.to_json().contains("\"payload\":["));
		assert!(result.to_json().contains("\"count\":1, \"position\":0,"));
		assert!(result.to_json().contains("\"pubdate\":\"2019-01-15T04:52:30Z\""));
		assert!(result.to_json().contains("\"moddate\":\"20"));
		assert!(result.to_json().contains(",\"cover_mime\":\"image/jpeg\"}]}"));

		result = reader.search("creator:\"Thomas de Quincey\"", 0, 10).expect("Search failed");
		assert!(result.count == 1);
		let book = result.payload.get(0).unwrap();
		println!("{}", book.creator.as_ref().unwrap());
		assert!(book.creator.as_ref().unwrap() == "Thomas De Quincey");
		assert!(book.filesize == 115227);

		result = reader.search("*", 0, 10).expect("Search failed");
		println!("Result count: {}", result.count);
		assert!(result.count == 8);

		let bm = reader.get_book(-5302641238507735522).unwrap();
		assert!(bm.creator.as_ref().unwrap() == "Charles Darwin");

		drop(reader);
		drop(_dirs_cleanup);

		//need to sleep else race condition prevents delete
		let second = time::Duration::from_millis(1000);
		thread::sleep(second);
		Ok(())
	}

	#[test]
	#[serial]
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

		let prefix_cats = reader
			.categorise("creator", "C", None, 0)
			.expect("Categorisation with 1 letter prefix failed.");

		//prefix_cats.categories.iter().for_each(|cat| {
		//	println!("Got category:{} ({})", cat.prefix, cat.count);
		//});

		assert!(prefix_cats.count == 1);

		let prefcat = prefix_cats.categories.get(0).expect("Should have a category");
		assert!(prefcat.prefix == "CH");

		let prefix_cats3 = reader
			.categorise("creator", "CHA", None, 0)
			.expect("Categorisation with 3 letter prefix failed.");

		/*prefix_cats3.categories.iter().for_each(|cat| {
			println!("Got category:{} ({})", cat.prefix, cat.count);
		});*/

		assert!(prefix_cats3.count == 1);

		let auth_cats = reader
			.count_by_field("creator", "CHA")
			.expect("Categorisation by authors should work");

		assert!(auth_cats.count == 2);

		let cat = auth_cats.categories.get(1).expect("Should be a 2nd category populated");
		assert!(cat.prefix == "Charles Dickens");
		assert!(cat.count == 2);

		/*auth_cats.categories.iter().for_each(|cat| {
			println!("Got category:{} ({})", cat.prefix, cat.count);
		});*/

		Ok(())
	}
}
