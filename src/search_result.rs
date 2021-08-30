use std::fmt::Debug;

use crate::error::ClientError;
//use BookMetadata;
//Responsible for representing search results, serialising into variopus formats etc

#[derive(Debug)]
pub struct SearchResult<T: Debug + serde::Serialize> {
	pub count: usize,
	pub start: usize,
	pub query: Option<String>,
	pub payload: Vec<T>,
}

#[derive(Debug)]
pub struct CategorySearchResult {
	pub count: usize,
	pub categories: Vec<Category>,
}

#[derive(Debug)]
pub struct Category {
	pub prefix: String,
	pub count: usize,
}

#[derive(Debug)]
pub struct OpdsPage {
	pub id: String,
	pub date: String,
	pub title: String,
	pub url: String,
}

impl<T: Debug + serde::Serialize> SearchResult<T> {
	pub fn to_json(&self) -> String {
		let query_str = match self.query.as_ref() {
			Some(q) => format!("\"query\":\"{}\",", q.replace("\"", "\\\"")),
			None => String::new(),
		};
		let mut json_str: String = format!(
			"{{\"count\":{}, \"position\":{}, \"query\":\"{}\", \"books\":[",
			self.count,
			self.start,
			query_str,
		)
		.to_string();

		let num_items = self.payload.len();

		for (i, bm) in self.payload.iter().enumerate() {
			json_str.push_str(match &serde_json::to_string(bm) {
				Ok(str) => str,
				Err(_) => continue,
			});

			if (i + 1) != num_items {
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
