use tantivy::collector::{TopDocs};

use tantivy::query::TermQuery;
use tantivy::schema::IndexRecordOption;

use ttvy::TantivyReader;
use tantivy::schema::Term;

use epub::doc::EpubDoc;

use std::io;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use error::ClientError;

pub struct Server {
	pub reader: TantivyReader,
	pub host: String,
	pub port: u32,
	pub use_coverdir: bool,
	pub coverdir: Option<String>,
}

#[derive(Debug)]
pub struct ServerError {
	msg: String,
}

impl std::error::Error for ServerError{}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Server {
	pub fn new(reader:TantivyReader,host:&str,port:u32, use_coverdir:bool, coverdir:Option<String>) -> Result<Server, ServerError> {
		Ok(Server {
			reader,
			host:host.to_string(),
			port,
			use_coverdir,
			coverdir,
		})
	}

	#[allow(unreachable_code)]
	pub fn serve(self) -> Result<(), tantivy::TantivyError> {
		println!("Starting server on localhost:8000");

		rouille::start_server("localhost:8000", move |request| {
			rouille::log(&request, io::stdout(), || {
				router!(request,
					(GET) (/api/search) => {
						let q = &request.get_param("query");
						let query_str = match q {
							Some(query) => query,
							None => return self.get_str_error_response("Query error", "\"query\" should be provided when performing a query")
						}.trim();

						let start = match request.get_param("start").unwrap_or_else(|| "0".to_string()).parse::<usize>() {
							Ok(start) => start,
							Err(_) => return self.get_str_error_response("Type error", "\"start\" should have an integer argument"),
						};

						let limit = match request.get_param("limit").unwrap_or_else(|| "20".to_string()).parse::<usize>() {
							Ok(lim) => lim,
							Err(_) => return self.get_str_error_response("Type error", "\"limit\" should have an integer argument"),
						}

						rouille::Response::from_data("application/json", json_str).with_additional_header("Access-Control-Allow-Origin", "*")
					},
					(GET) (/api/book/{id: i64}) => {
						let searcher = &self.reader.reader.searcher();

						let id_field = self.id_field;
						let id_term = Term::from_field_i64(id_field, id);

						let term_query = TermQuery::new(id_term, IndexRecordOption::Basic);

						//could this be better with TopFieldCollector which uses a FAST field?
						let docs = searcher.search(&term_query, &TopDocs::with_limit(1)).unwrap();
						
						if docs.len()==1 {
							let retrieved = searcher.doc(docs.first().unwrap().1.to_owned()).unwrap();
							let file_loc = retrieved.get_first(self.file_field).unwrap().text().unwrap();
							let creator = retrieved.get_first(self.creator_field).unwrap().text().unwrap();
							let title = retrieved.get_first(self.title_field).unwrap().text().unwrap();
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
						let id_term = Term::from_field_i64(self.id_field, id);

						let term_query = TermQuery::new(id_term, IndexRecordOption::Basic);

						//could this be better with TopFieldCollector which uses a FAST field?
						let docs = searcher.search(&term_query, &TopDocs::with_limit(1)).unwrap();

						if docs.len()==1 {
							let retrieved = searcher.doc(docs.first().unwrap().1.to_owned()).unwrap();
							if self.use_coverdir {
								let mime = match retrieved.get_first(self.mime_field) {
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
								let mut doc = EpubDoc::new(retrieved.get_first(self.file_field).unwrap().text().unwrap()).unwrap();
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

	fn get_error_response(&self, client_error: &ClientError) -> rouille::Response {
		rouille::Response::from_data(
			"application/json",
			format!("{{\"error\":[{:?}]}}", serde_json::to_string(client_error)),
		)
	}

	fn get_str_error_response(&self, name: &str, msg: &str) -> rouille::Response {
		rouille::Response::from_data(
			"application/json",
			format!(
				"{{\"error\":[{:?}]}}",
				serde_json::to_string(&ClientError {
					name: name.to_string(),
					msg: msg.to_string(),
				})
			),
		)
	}
}
