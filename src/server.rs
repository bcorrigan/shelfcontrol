use tantivy::collector::{Count, TopDocs};
use tantivy::query::QueryParserError;
use tantivy::query::QueryParserError::*;
use tantivy::query::TermQuery;
use tantivy::schema::IndexRecordOption;
use tantivy::schema::Value::Facet;
use ttvy::TantivyReader;
use BookMetadata;
//use tantivy::schema::Facet;
use tantivy::schema::Schema;
use tantivy::schema::Term;

use epub::doc::EpubDoc;

use std::io;

use std::fs::File;
use std::io::prelude::*;

pub struct Server {
	pub reader: TantivyReader,
	pub host: String,
	pub port: u32,
	pub use_coverdir: bool,
	pub coverdir: Option<String>,
}

#[derive(Debug, Serialize)]
struct ClientError {
	name: String,
	msg: String,
}


impl Server {
	#[allow(unreachable_code)]
	pub fn serve(self) -> Result<(), tantivy::TantivyError> {
		println!("Starting server on localhost:8000");

		rouille::start_server("localhost:8000", move |request| {
			rouille::log(&request, io::stdout(), || {
				router!(request,
					(GET) (/api/search) => {
						let searcher = &self.reader.reader.searcher();
						let q = &self.reader.query_parser.parse_query(match &request.get_param("query") {
							Some(query) => query,
							None => return self.get_str_error_response("Query error", "\"query\" should be provided when performing a query")
						}.trim());

						let query = match q {
							Err(e) => return self.get_error_response(&self.get_query_error(&e)),
							Ok(q) => q,
						};

						//in theory we could cache the searcher for subsequent queries with a higher startat.. but too complex for now
						let start_pos = match request.get_param("start").unwrap_or_else(|| "0".to_string()).parse::<usize>() {
							Ok(pos) => pos,
							Err(_) => return self.get_str_error_response("Type error", "\"start\" should have an integer argument"),
						};

						let top_collector = TopDocs::with_limit(match request.get_param("limit").unwrap_or_else(|| "20".to_string()).parse::<usize>() {
							Ok(lim) => lim,
							Err(_) => return self.get_str_error_response("Type error", "\"limit\" should have an integer argument"),
						} + start_pos);

						let count_collector = Count;

						let docs = searcher.search(query, &(top_collector, count_collector)).unwrap();

						let num_docs = docs.0.len();
						//let mut i = 0;
						//json encode query value
						let mut json_str: String = format!("{{\"count\":{}, \"position\":{}, \"query\":\"{}\", \"books\":[", docs.1, start_pos, &request.get_param("query").unwrap().replace("\"","\\\"")).to_owned();
						for (i,doc) in docs.0.iter().enumerate() {
							//i+=1;
							if (i+1)>start_pos {
								let retrieved = searcher.doc(doc.1).unwrap();

								json_str.push_str(&serde_json::to_string(&self.to_bm(&retrieved, &searcher.schema())).unwrap());

								if (i+1)!=num_docs {
									json_str.push_str(",");
								}
							}
						}
						json_str.push_str("]}");

						rouille::Response::from_data("application/json", json_str).with_additional_header("Access-Control-Allow-Origin", "*")
					},
				    (GET) (/api/book/{id: i64}) => {
				        //FIXME so many unwraps, damn
				        let searcher = &self.reader.reader.searcher();
				        let schema = searcher.schema();

				        let id_field = schema.get_field("id").unwrap();
						let id_term = Term::from_field_i64(id_field, id);

						let term_query = TermQuery::new(id_term, IndexRecordOption::Basic);

						//could this be better with TopFieldCollector which uses a FAST field?
						let docs = searcher.search(&term_query, &TopDocs::with_limit(1)).unwrap();

				        if docs.len()==1 {
				            let retrieved = searcher.doc(docs.first().unwrap().1.to_owned()).unwrap();
	                        let file_loc = retrieved.get_first(schema.get_field("file").unwrap()).unwrap().text().unwrap();
	                        let creator = retrieved.get_first(schema.get_field("creator").unwrap()).unwrap().text().unwrap();
	                        let title = retrieved.get_first(schema.get_field("title").unwrap()).unwrap().text().unwrap();
	                        let mut f = File::open(file_loc).unwrap();
	                        let mut buffer = Vec::new();
	                        // read the whole file
	                        f.read_to_end(&mut buffer).unwrap();
	                        return rouille::Response::from_data("application/epub+zip", buffer).with_additional_header("Access-Control-Allow-Origin", "*")
	                                                                                           .with_content_disposition_attachment(&format!("{} - {}", creator, title));
				        } else {
							println!("404 1, found {:?}", docs.len());
							return rouille::Response::empty_404()
						}
				    },
					(GET) (/img/{id: i64}) => {
						//FIXME so many unwraps, damn
						let searcher = &self.reader.reader.searcher();
						let schema = searcher.schema();
						let id_field = schema.get_field("id").unwrap();
						let id_term = Term::from_field_i64(id_field, id);

						let term_query = TermQuery::new(id_term, IndexRecordOption::Basic);

						//could this be better with TopFieldCollector which uses a FAST field?
						let docs = searcher.search(&term_query, &TopDocs::with_limit(1)).unwrap();

						if docs.len()==1 {
							let retrieved = searcher.doc(docs.first().unwrap().1.to_owned()).unwrap();
							if self.use_coverdir {
								let mime = match retrieved.get_first(schema.get_field("cover_mime").unwrap()) {
									Some(mime) => mime.text().unwrap().to_owned(),
									None =>  return rouille::Response::empty_404()
								};
								if mime.is_empty() {
									return rouille::Response::empty_404();
								}

								let mut imgfile = File::open(format!("{}/{}",self.coverdir.clone().unwrap(),id)).unwrap();
								let mut imgbytes = Vec::new();
								match imgfile.read_to_end(&mut imgbytes) {
									Ok(_) => {
										return rouille::Response::from_data(mime, imgbytes).with_additional_header("Access-Control-Allow-Origin", "*");
									},
									Err(_) => return rouille::Response::empty_404(),
								}
							} else {
								//ok doing it inline like this for a very low use server
								let mut doc = EpubDoc::new(retrieved.get_first(schema.get_field("file").unwrap()).unwrap().text().unwrap()).unwrap();
								match doc.get_cover() {
									Ok(cover) => { let mime = doc.get_resource_mime(&doc.get_cover_id().unwrap()).unwrap();
											 	return rouille::Response::from_data(mime, cover).with_additional_header("Access-Control-Allow-Origin", "*"); },
												Err(_) => return rouille::Response::empty_404(),
								}
							}
						} else {
							println!("404 1, found {:?}", docs.len());
							return rouille::Response::empty_404()
						}
					},
					_ => rouille::Response::empty_404()
				)
			})
		})
	}

	pub fn to_bm(&self, doc: &tantivy::Document, schema: &Schema) -> BookMetadata {
		BookMetadata {
			id: self.get_doc_i64("id", &doc, &schema), //not populated ?
			title: self.get_doc_str("title", &doc, &schema),
			description: self.get_doc_str("description", &doc, &schema),
			publisher: self.get_doc_str("publisher", &doc, &schema),
			creator: self.get_doc_str("creator", &doc, &schema),
			subject: self.get_tags("tags", &doc, &schema),
			file: self.get_doc_str("file", &doc, &schema).unwrap(),
			filesize: self.get_doc_i64("filesize", &doc, &schema),
			modtime: self.get_doc_i64("modtime", &doc, &schema),
			pubdate: self.get_doc_str("pubdate", &doc, &schema),
			moddate: self.get_doc_str("moddate", &doc, &schema),
			cover_mime: self.get_doc_str("cover_mime", &doc, &schema),
		}
	}

	//I *know* the fields are present in schema, and I *know* that certain fields eg id are always populated, so just unwrap() here
	fn get_doc_str(&self, field: &str, doc: &tantivy::Document, schema: &Schema) -> Option<String> {
		doc.get_first(schema.get_field(field).unwrap())
			.map(|val| match val.text() {
				Some(t) => t.to_string(),
				_ => "".to_string()
			})
	}

	fn get_doc_i64(&self, field: &str, doc: &tantivy::Document, schema: &Schema) -> i64 {
		doc.get_first(schema.get_field(field).unwrap()).unwrap().i64_value()
	}

	fn get_tags(&self, _field: &str, doc: &tantivy::Document, schema: &Schema) -> Option<Vec<String>> {
		let vals: Vec<&tantivy::schema::Value> = doc.get_all(schema.get_field("tags").unwrap());
		if vals.is_empty() {
			return None;
		}
		let mut tags = Vec::new();

		for v in vals {
			tags.push(
				match v {
					Facet(f) => f.encoded_str(),
					_ => "",
				}
				.to_string(),
			)
		}

		Some(tags)
	}

	fn get_query_error(&self, query_err: &QueryParserError) -> ClientError {
		match query_err {
			SyntaxError => ClientError {
				name: "Syntax Error".to_string(),
				msg: "There was a syntax error in the search string.".to_string(),
			},
			FieldDoesNotExist(s) => ClientError {
				name: "Field does not Exist".to_string(),
				msg: format!("You searched for a field that does not exist:{}", s),
			},
			ExpectedInt(_) => ClientError {
				name: "Expected Integer".to_string(),
				msg: "Search argument requires an integer.".to_string(),
			},
			AllButQueryForbidden => ClientError {
				name: "All But query forbidden".to_string(),
				msg: "Queries that only exclude (eg. \"-king\") are forbidden.".to_string(),
			},
			NoDefaultFieldDeclared => ClientError {
				name: "No field declared".to_string(),
				msg: "You must specify a field to query.".to_string(),
			},
			FieldNotIndexed(s) => ClientError {
				name: "Field unknown".to_string(),
				msg: format!("The field you searched for is unknown:{}", s),
			},
			UnknownTokenizer(_, _) => ClientError {
				name: "Unknown Tokenizer".to_string(),
				msg: "The tokenizer for the given field is unknown".to_string(),
			},
			FieldDoesNotHavePositionsIndexed(s) => ClientError {
				name: "Field does not have positions indexed".to_string(),
				msg: format!("Field does not have positions indexed: {}", s),
			},
			RangeMustNotHavePhrase => ClientError {
				name: "Range must not have phrase".to_string(),
				msg: "Range must not have phrase".to_string(),
			},
			DateFormatError(_) => ClientError {
				name: "Date must have correct format".to_string(),
				msg: "Date must have correct format".to_string(),
			},
		}
	}

	fn get_error_response(&self, client_error: &ClientError) -> rouille::Response {
		rouille::Response::from_data("application/json", format!( "{{\"error\":[{:?}]}}", serde_json::to_string(client_error)))
	}

	fn get_str_error_response(&self, name:&str, msg:&str) -> rouille::Response {
		rouille::Response::from_data("application/json", format!( "{{\"error\":[{:?}]}}", serde_json::to_string( &ClientError {
			name: name.to_string(),
			msg: msg.to_string(),
		})))
	}
}
