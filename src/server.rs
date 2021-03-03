use crate::ttvy::TantivyReader;

use epub::doc::EpubDoc;

use crate::error::ClientError;
use crate::error::StoreError;
use rouille::Response;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use crate::search_result::OpdsPage;
use crate::OpdsCategory;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

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

impl std::error::Error for ServerError {}

impl fmt::Display for ServerError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.msg)
	}
}

impl Server {
	pub fn new(reader: TantivyReader, host: &str, port: u32, use_coverdir: bool, coverdir: Option<String>) -> Result<Server, ServerError> {
		Ok(Server {
			reader,
			host: host.to_string(),
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
							None => return self.get_json_error_response("Query error", "\"query\" should be provided when performing a query")
						}.trim();

						let start = match request.get_param("start").unwrap_or_else(|| "0".to_string()).parse::<usize>() {
							Ok(start) => start,
							Err(_) => return self.get_json_error_response("Type error", "\"start\" should have an integer argument"),
						};

						let limit = match request.get_param("limit").unwrap_or_else(|| "20".to_string()).parse::<usize>() {
							Ok(lim) => lim,
							Err(_) => return self.get_json_error_response("Type error", "\"limit\" should have an integer argument"),
						};

						return match self.reader.search(query_str, start, limit) {
							Ok(response) => Response::from_data("application/json", response.to_json()).with_additional_header("Access-Control-Allow-Origin", "*"),
							Err(e) => {
								if let StoreError::ClientError(ce) = e {
									Response::from_data("application/json", ce.get_error_response_json()).with_additional_header("Access-Control-Allow-Origin", "*")
								} else {
									println!("Error searching tantivy: {}", e);
									self.get_json_error_response("Server error","There was a server side error.").with_status_code(500)
								}
							}
						}
					},
					(GET) (/api/book/{id: i64}) => {
						return match self.reader.get_book(id) {
							Some(doc) => {
								let mut f = match File::open(doc.file) {
									Ok(f) => f,
									Err(_) => {println!("Book {} vanished since indexed.", id); return Response::empty_404()},
								};
								let mut buffer = Vec::new();
								// read the whole file
								match f.read_to_end(&mut buffer) {
									Ok(_) => (),
									Err(_) => {println!("Could not read all of book {} from file system.", id); return Response::empty_404()},
								}
								Response::from_data("application/epub+zip", buffer).with_additional_header("Access-Control-Allow-Origin", "*")
																							.with_content_disposition_attachment(&format!("{} - {}",
																							doc.creator.unwrap_or("unknown".to_string()),
																							doc.title.unwrap_or("unknown author".to_string())))
							},
							None => Response::empty_404(),
						}
					},
					(GET) (/opds) => {
						//in this case we return only root nav entries:
						//Authors, Tags, Year of Publication, Author, Titles
						let navs = vec!(
							OpdsCategory::new("Authors".to_string(), "/opds/authors".to_string()),
							OpdsCategory::new("Tags".to_string(), "/opds/tags".to_string()),
							OpdsCategory::new("Year of Publication".to_string(), "".to_string()),
							OpdsCategory::new("Titles".to_string(), "".to_string()),
						);

						let mut buf = Vec::new();
						match templates::opds(&mut buf, &OpdsPage {id:"1".to_string(),date:"2021-01-21T10:56:30+01:00".to_string(),title:"ShelfControl".to_string(),url:"localhost:8000".to_string()}, &None, &Some(navs)) { 
							Ok(_) => return Response::from_data("application/xml", buf),
							Err(e) => {println!("Error {:?}", e);self.get_json_error_response("OPDS error", "OPDS Error")},
						}

					},
					(GET) (/opds/authors) => {
						let cat_param = &request.get_param("categorise");
						let (cat_str, query) = match cat_param {
							Some(cat) => (cat.to_string(), None),
							None => ("".to_string(), Some("*"))
						};

						//call categorise
						let search_result = match self.reader.categorise("creator", &cat_str, query, 100) {
							Ok(result) => result,
							Err(_) => return self.get_json_error_response("Author search error", "Author search error"), //FIXME opds error response!
						};

						//populate OpdsCategory navs, for each search result
						let navs:Vec<OpdsCategory> = search_result.categories.iter().map(|cat| {
							OpdsCategory::new(format!( "{} ({})", cat.prefix, cat.count), format!("/opds/authors?categorise={}", cat.prefix))
						}).collect();

						let mut buf = Vec::new();
						match templates::opds(&mut buf, &OpdsPage {id:"1".to_string(),date:"2021-01-21T10:56:30+01:00".to_string(),title:"ShelfControl".to_string(),url:"localhost:8000".to_string()}, &None, &Some(navs)) { 
							Ok(_) => return Response::from_data("application/xml", buf),
							Err(e) => {println!("Error {:?}", e);self.get_json_error_response("OPDS error", "OPDS Error")},
						}
					},
					(GET) (/opds/tags) => {
						unimplemented!()
					},
					(GET) (/img/{id: i64}) => {
						return match self.reader.get_book(id) {
							Some(doc) => {
								if self.use_coverdir {
									let mime = match doc.cover_mime {
										Some(mime) => mime,
										None =>  return Response::empty_404(),
									};
									if mime.is_empty() {
										return Response::empty_404();
									}

									let mut imgfile = match File::open(format!("{}/{}",self.coverdir.clone().unwrap_or(".".to_string()),id)) {
										Ok(file) => file,
										Err(_) => {println!("Could not open img {}.", id); return Response::empty_404()},
									};
									let mut imgbytes = Vec::new();
									match imgfile.read_to_end(&mut imgbytes) {
										Ok(_) => {
											Response::from_data(mime, imgbytes).with_additional_header("Access-Control-Allow-Origin", "*")
										},
										Err(_) => {println!("Could not read img file to end for book {}.", id); Response::empty_404()},
									}
								} else {
									//ok doing it inline like this for a very low use server
									let mut epub = match EpubDoc::new(doc.file) {
										Ok(epub) => epub,
										Err(_) => return Response::empty_404(),
									};
									match epub.get_cover() {
										Ok(cover) => {
												let cover_id_opt = &epub.get_cover_id();
												let cover_id = match cover_id_opt {
													Ok(id) => id,
													Err(_) => {println!("No cover id in book {}", id); return Response::empty_404()},
												};
												let mime = match epub.get_resource_mime(cover_id) {
													Ok(mime) => mime,
													Err(_) => {println!("No mime in book {}", id); return Response::empty_404()},
												};
												Response::from_data(mime, cover).with_additional_header("Access-Control-Allow-Origin", "*")
											},
										Err(_) => Response::empty_404(),
									}
								}
							},
							None => Response::empty_404(),
						}
					},
					_ => Response::empty_404()
				)
			})
		})
	}

	fn get_json_error_response(&self, name: &str, msg: &str) -> Response {
		Response::from_data(
			"application/json",
			format!(
				"{{\"error\":[{:?}]}}",
				serde_json::to_string(&ClientError {
					name: name.to_string(),
					msg: msg.to_string(),
				})
			),
		)
		.with_additional_header("Access-Control-Allow-Origin", "*")
	}
}
