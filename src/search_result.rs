use crate::BookMetadata;
use crate::error::ClientError;
//use BookMetadata;
//Responsible for representing search results, serialising into variopus formats etc

#[derive(Debug)]
pub struct SearchResult {
	pub count: usize,
	pub start: usize,
	pub query: String,
	pub books: Vec<BookMetadata>,
}

#[derive(Debug)]
pub struct OpdsPage {
	pub id:String,
	pub date:String,
	pub title:String,
	pub url:String,
}

impl SearchResult {
	pub fn to_json(&self) -> String {
		let mut json_str: String = format!(
			"{{\"count\":{}, \"position\":{}, \"query\":\"{}\", \"books\":[",
			self.count,
			self.start,
			self.query.replace("\"", "\\\"")
		)
		.to_string();

		let num_books = self.books.len();

		for (i, bm) in self.books.iter().enumerate() {
			json_str.push_str(match &serde_json::to_string(bm) {
				Ok(str) => str,
				Err(_) => continue,
			});

			if (i + 1) != num_books {
				json_str.push_str(",");
			}
		}

		json_str.push_str("]}");
		json_str
	}
}

impl ClientError {
	pub fn get_error_response_json(&self) -> String {
		format!("{{\"error\":[{:?}]}}", serde_json::to_string(&self))
	}
}
