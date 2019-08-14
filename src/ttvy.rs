use std::error::Error;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

//extern crate tantivy;
use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, ReloadPolicy};
use tantivy::IndexWriter;

use ammonia::{Builder, UrlRelative};
use maplit;

use core::borrow::{Borrow, BorrowMut};
use BookMetadata;
use BookWriter;
use tantivy::query::QueryParser;

pub struct TantivyWriter<'a> {
	dir: String,
	index: Index,
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
			dir: dir,
			index: index,
			index_writer: writer,
			id: id,
			title: title,
			description: description,
			publisher: publisher,
			creator: creator,
			file: file,
			filesize: filesize,
			modtime: modtime,
			pubdate: pubdate,
			moddate: moddate,
			cover_mime: cover_mime,
			tags: tags,
			sanitiser: b,
		})
	}
}

impl<'a> BookWriter for TantivyWriter<'a> {
	fn write_tags(&self, tags: HashMap<String, Vec<i64>>, limit: usize) -> Result<(), Box<Error>> {
		//not sure if facets can be added later in tantivy??
		Ok(())
	}

	fn write_epubs(&mut self, bms: Vec<BookMetadata>, tags: &mut HashMap<String, Vec<i64>>) -> Result<(), Box<Error>> {
		for bm in bms {
			bm.add_tags(tags);

			let mut ttdoc = Document::default();
			ttdoc.add_i64(self.id, bm.id);
			ttdoc.add_text(self.title, &bm.title.unwrap_or("".to_string()));
			ttdoc.add_text(
				self.description,
				self.sanitiser.clean(&bm.description.unwrap_or("".to_string())).to_string().as_str(),
			);
			ttdoc.add_text(self.publisher, &bm.publisher.unwrap_or("".to_string()));
			ttdoc.add_text(self.creator, &bm.creator.unwrap_or("".to_string()));
			ttdoc.add_text(self.file, &bm.file);
			ttdoc.add_i64(self.filesize, bm.filesize);
			ttdoc.add_i64(self.modtime, bm.modtime);
			ttdoc.add_text(self.pubdate, &bm.pubdate.unwrap_or("".to_string()));
			ttdoc.add_text(self.moddate, &bm.moddate.unwrap_or("".to_string()));
			ttdoc.add_text(self.cover_mime, &bm.cover_mime.unwrap_or("".to_string()));

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

			&self.index_writer.add_document(ttdoc);

			//tags can probably be "facets" in tantivy, see: https://github.com/tantivy-search/tantivy/issues/215
			//this appears ot be only way to support multiple of them
		}
		Ok(())
	}

	fn commit(&mut self) -> Result<(), Box<Error>> {
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
	pub reader: IndexReader,
	pub query_parser: QueryParser,
}

impl TantivyReader {
	pub fn new(index: String) -> Result<TantivyReader, tantivy::TantivyError> {
		let path = Path::new(&index);
		let mmap_dir = MmapDirectory::open(path)?;
		let index = Index::open(mmap_dir)?;

		let mut query_parser = QueryParser::for_index(&index, vec![index.schema().get_field("title").unwrap(), index.schema().get_field("description").unwrap()]);
		query_parser.set_conjunction_by_default();
		Ok(TantivyReader {
			reader: index
				.reader_builder()
				.reload_policy(ReloadPolicy::OnCommit)
				.try_into()?,
			query_parser: query_parser
		})
	}
}
