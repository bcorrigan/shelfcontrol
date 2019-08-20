use std::error::Error;

use std::collections::HashMap;

use std::fs;
use std::io;
use std::path::Path;
use std::process;
use std::fmt;

use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, ReloadPolicy};
use tantivy::IndexWriter;
//use tantivy::schema::Value::Facet;
use tantivy::collector::{Count, TopDocs};

use ammonia::{Builder, UrlRelative};
use BookMetadata;
use BookWriter;
use tantivy::query::QueryParser;
use error::{ClientError,StoreError,get_query_error};

pub struct TantivyWriter<'a> {
	index_writer: IndexWriter,
	id: Field,
	title: Field,
	description: Field,
	publisher: Field,
	creator: Field,
	file: Field,
	filesize: Field,
	modtime: Field,
	pubdate: Field,
	moddate: Field,
	cover_mime: Field,
	tags: Field,
	sanitiser: Builder<'a>,
}

impl<'a> TantivyWriter<'a> {
	pub fn new(dir: String) -> Result<TantivyWriter<'a>, tantivy::TantivyError> {
		if Path::new(&dir).exists() {
			println!("Error: Must remove directory {} to run.", &dir);
			process::exit(3);
		}
		fs::create_dir(&dir)?;

		//build our schema
		let mut schema_builder = SchemaBuilder::default();
		//let id_options = IntOptions::default().set_stored().set_indexed();
		let id = schema_builder.add_i64_field("id", IntOptions::default().set_stored().set_indexed());
		let title = schema_builder.add_text_field("title", TEXT | STORED);
		let description = schema_builder.add_text_field("description", TEXT | STORED);
		let publisher = schema_builder.add_text_field("publisher", TEXT | STORED);
		let creator = schema_builder.add_text_field("creator", TEXT | STORED);
		//subject
		let file = schema_builder.add_text_field("file", STRING | STORED);
		let filesize = schema_builder.add_i64_field("filesize", IntOptions::default().set_stored().set_indexed());
		let modtime = schema_builder.add_i64_field("modtime", IntOptions::default().set_stored().set_indexed());
		let pubdate = schema_builder.add_text_field("pubdate", TEXT | STORED);
		let moddate = schema_builder.add_text_field("moddate", TEXT | STORED);
		let cover_mime = schema_builder.add_text_field("cover_mime", TEXT | STORED);
		let tags = schema_builder.add_facet_field("tags");
		let schema = schema_builder.build();
		let path_dir = dir.clone();
		let path = Path::new(&path_dir);
		let mmap_dir = MmapDirectory::open(path)?;
		let index = Index::create(mmap_dir, schema.clone())?;
		let writer = index.writer(50_000_000)?;

		let mut b = Builder::default();
		{
			b.link_rel(None).url_relative(UrlRelative::PassThrough).tags(hashset![
				"b",
				"i",
				"p",
				"a",
				"blockquote",
				"code",
				"q",
				"em",
				"br",
				"ul",
				"u",
				"tt",
				"tr",
				"th",
				"td",
				"ol",
				"li",
				"h6",
				"h5",
				"h4",
				"h3",
				"abbr"
			]);
		}

		Ok(TantivyWriter {
			index_writer: writer,
			id,
			title,
			description,
			publisher,
			creator,
			file,
			filesize,
			modtime,
			pubdate,
			moddate,
			cover_mime,
			tags,
			sanitiser: b,
		})
	}
}

impl<'a> BookWriter for TantivyWriter<'a> {
	fn write_tags(&self, _tags: HashMap<String, Vec<i64>>, _limit: usize) -> Result<(), Box<dyn Error>> {
		//not sure if facets can be added later in tantivy??
		Ok(())
	}

	fn write_epubs(&mut self, bms: Vec<BookMetadata>, tags: &mut HashMap<String, Vec<i64>>) -> Result<(), Box<dyn Error>> {
		for bm in bms {
			bm.add_tags(tags);

			let mut ttdoc = Document::default();
			ttdoc.add_i64(self.id, bm.id);
			ttdoc.add_text(self.title, &bm.title.unwrap_or_else(|| "".to_string()));
			ttdoc.add_text(
				self.description,
				self.sanitiser.clean(&bm.description.unwrap_or_else(|| "".to_string())).to_string().as_str(),
			);
			ttdoc.add_text(self.publisher, &bm.publisher.unwrap_or_else(|| "".to_string()));
			ttdoc.add_text(self.creator, &bm.creator.unwrap_or_else(|| "".to_string()));
			ttdoc.add_text(self.file, &bm.file);
			ttdoc.add_i64(self.filesize, bm.filesize);
			ttdoc.add_i64(self.modtime, bm.modtime);
			ttdoc.add_text(self.pubdate, &bm.pubdate.unwrap_or_else(|| "".to_string()));
			ttdoc.add_text(self.moddate, &bm.moddate.unwrap_or_else(|| "".to_string()));
			ttdoc.add_text(self.cover_mime, &bm.cover_mime.unwrap_or_else(|| "".to_string()));

			if bm.subject.is_some() {
				for subject in &bm.subject.unwrap() {
					if !&subject.trim().is_empty() {
						//println!("Making facet from: {}", &subject);
						let mut tag = "/".to_string();
						tag.push_str(&subject);
						ttdoc.add_facet(self.tags, Facet::from(tag.as_str()));
					}
				}
			}

			self.index_writer.add_document(ttdoc);

			//tags can probably be "facets" in tantivy, see: https://github.com/tantivy-search/tantivy/issues/215
			//this appears ot be only way to support multiple of them
		}
		Ok(())
	}

	fn commit(&mut self) -> Result<(), Box<dyn Error>> {
		match self.index_writer.commit() {
			Ok(_) => Ok(()),
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:"))),
		}?;

		match self.index_writer.garbage_collect_files() {
			Ok(_) => Ok(()),
			Err(_) => Err(Box::new(io::Error::new(
				io::ErrorKind::Other,
				"TantivyError - garbage collect failed",
			))),
		}
	}
}

pub struct TantivyReader {
	reader: IndexReader,
	query_parser: QueryParser,
	id_field: Field,
	file_field: Field,
	creator_field: Field,
	mime_field: Field,
	tags_field: Field,
	title_field: Field,
}

impl TantivyReader {
	pub fn new(index: String) -> Result<TantivyReader, StoreError> {
		let path = Path::new(&index);
		let mmap_dir = MmapDirectory::open(path)?;
		let index = Index::open(mmap_dir)?;
		let reader = index.reader_builder().reload_policy(ReloadPolicy::OnCommit).try_into()?;
		let schema = reader.searcher().schema();
		let mut query_parser = QueryParser::for_index(&index, vec![index.schema().get_field("creator").unwrap(),index.schema().get_field("title").unwrap(), index.schema().get_field("description").unwrap()]);
		query_parser.set_conjunction_by_default();
		Ok(TantivyReader {
			reader,
			query_parser,
			id_field:TantivyReader::get_field(schema, "id")?,
			file_field:TantivyReader::get_field(schema, "file")?,
			creator_field:TantivyReader::get_field(schema, "creator")?,
			mime_field:TantivyReader::get_field(schema, "cover_mime")?,
			tags_field:TantivyReader::get_field(schema, "tags")?,
			title_field:TantivyReader::get_field(schema, "title")?,
		})
	}

	pub fn get_field(schema:&Schema, field_name:&str) -> Result<Field, StoreError> {
		match schema.get_field(field_name) {
			Some(f) => Ok(f),
			None => Err(StoreError::InitError( format!("Mismatching schema - specified field {} does not exist in tantivy schema for this index.", field_name).to_string() )),
		}
	}

	//    /api/search
	pub fn search(&self, query: &str, start:usize, limit:usize) -> Result<String, StoreError> {
		let searcher = &self.reader.searcher();

		let query_parsed = &self.query_parser.parse_query(query);
		let tquery = match query_parsed {
			Err(e) => return Ok(self.get_error_response(&get_query_error(&e))),
			Ok(q) => q,
		};

		let top_collector = TopDocs::with_limit(start + limit);

		let count_collector = Count;

		let docs = match searcher.search(tquery, &(top_collector, count_collector)) {
			Ok(docs) => docs,
			Err(e) => {println!("Error searching:{}", e); return Ok(self.get_str_error_response("Index error", "Something is wrong with the index"))},
		};


		let num_docs = docs.0.len();
		//json encode query value
		let mut json_str: String = format!("{{\"count\":{}, \"position\":{}, \"query\":\"{}\", \"books\":[", docs.1, start, query.replace("\"","\\\"")).to_owned();
		for (i,doc) in docs.0.iter().enumerate() {
			if (i+1)>start {
				let retrieved = match searcher.doc(doc.1) {
					Ok(doc) => doc,
					Err(_) => continue,
				};

				json_str.push_str(match &serde_json::to_string(&self.to_bm(&retrieved, &searcher.schema())) {
					Ok(str) => str,
					Err(_) => continue,
				});

				if (i+1)!=num_docs {
					json_str.push_str(",");
				}
			}
		}
		json_str.push_str("]}");

		Ok(json_str)
	}

	fn get_error_response(&self, client_error: &ClientError) -> String {
		format!("{{\"error\":[{:?}]}}", serde_json::to_string(client_error))
	}

	fn get_str_error_response(&self, name: &str, msg: &str) -> String {
			format!(
				"{{\"error\":[{:?}]}}",
				serde_json::to_string(&ClientError {
					name: name.to_string(),
					msg: msg.to_string(),
				})
			)
	}

	fn to_bm(&self, doc: &tantivy::Document, schema: &Schema) -> BookMetadata {
		BookMetadata {
			id: self.get_doc_i64("id", &doc, &schema), //not populated ?
			title: self.get_doc_str("title", &doc, &schema),
			description: self.get_doc_str("description", &doc, &schema),
			publisher: self.get_doc_str("publisher", &doc, &schema),
			creator: self.get_doc_str("creator", &doc, &schema),
			subject: self.get_tags("tags", &doc),
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
		doc.get_first(schema.get_field(field).unwrap()).map(|val| match val.text() {
			Some(t) => t.to_string(),
			_ => "".to_string(),
		})
	}

	fn get_doc_i64(&self, field: &str, doc: &tantivy::Document, schema: &Schema) -> i64 {
		doc.get_first(schema.get_field(field).unwrap()).unwrap().i64_value()
	}

	fn get_tags(&self, _field: &str, doc: &tantivy::Document) -> Option<Vec<String>> {
		let vals: Vec<&tantivy::schema::Value> = doc.get_all(self.tags_field);
		if vals.is_empty() {
			return None;
		}
		let mut tags = Vec::new();

		for v in vals {
			tags.push(
				match v {
					 tantivy::schema::Value::Facet(f) => f.encoded_str(),
					_ => "",
				}
				.to_string(),
			)
		}

		Some(tags)
	}
}
