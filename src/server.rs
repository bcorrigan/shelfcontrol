use ttvy::TantivyReader;

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
						let query_param = &request.get_param("query");
						let query_str = match query_param {
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
						};

						return match self.reader.search(query_str, start, limit) {
							Ok(response) => rouille::Response::from_data("application/json", response).with_additional_header("Access-Control-Allow-Origin", "*"),
							Err(e) => { println!("Error searching tantivy: {}", e); self.get_str_error_response("Server error","There was a server side error.").with_status_code(500) },
						}
					},
					(GET) (/api/book/{id: i64}) => {
						return match self.reader.get_book(id) {
							Some(doc) => { let mut f = File::open(doc.file).unwrap();
								let mut buffer = Vec::new();
								// read the whole file
								f.read_to_end(&mut buffer).unwrap();
								rouille::Response::from_data("application/epub+zip", buffer).with_additional_header("Access-Control-Allow-Origin", "*")
																							.with_content_disposition_attachment(&format!("{} - {}", 
																							doc.creator.unwrap_or("unknown".to_string()), 
																							doc.title.unwrap_or("unknown author".to_string())))
							},
							None => rouille::Response::empty_404(),
						}
					},
					(GET) (/img/{id: i64}) => {
						return match self.reader.get_book(id) {
							Some(doc) => {
								if self.use_coverdir {
									let mime = match doc.cover_mime {
										Some(mime) => mime,
										None =>  return rouille::Response::empty_404()
									};
									if mime.is_empty() {
										return rouille::Response::empty_404();
									}

									let mut imgfile = File::open(format!("{}/{}",self.coverdir.clone().unwrap(),id)).unwrap();
									let mut imgbytes = Vec::new();
									match imgfile.read_to_end(&mut imgbytes) {
										Ok(_) => {
											rouille::Response::from_data(mime, imgbytes).with_additional_header("Access-Control-Allow-Origin", "*")
										},
										Err(_) => rouille::Response::empty_404(),
									}
								} else {
									//ok doing it inline like this for a very low use server
									let mut epub = EpubDoc::new(doc.file).unwrap();
									match epub.get_cover() {
										Ok(cover) => { let mime = epub.get_resource_mime(&epub.get_cover_id().unwrap()).unwrap();
													   rouille::Response::from_data(mime, cover).with_additional_header("Access-Control-Allow-Origin", "*") },
													Err(_) => rouille::Response::empty_404(),
									}
								}
							},
							None => rouille::Response::empty_404(),
						}
					},
					_ => rouille::Response::empty_404()
				)
			})
		})
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
		).with_additional_header("Access-Control-Allow-Origin", "*")
	}
}