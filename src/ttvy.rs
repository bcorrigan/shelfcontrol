use std::error::Error;

use std::collections::HashMap;
use std::collections::HashSet;

use std::fs;
use std::io;
use std::path::Path;
use std::process;

use tantivy::collector::{Collector, Count, SegmentCollector, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::TermQuery;
use tantivy::schema::*;
use tantivy::DocId;
use tantivy::IndexWriter;
use tantivy::Score;
use tantivy::SegmentLocalId;
use tantivy::SegmentReader;
use tantivy::{Index, IndexReader, ReloadPolicy};

use futures::executor;

use crate::error::StoreError;
use crate::search_result::SearchResult;
use crate::BookMetadata;
use crate::BookWriter;
use ammonia::{Builder, UrlRelative};
use tantivy::query::QueryParser;

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
				self.sanitiser
					.clean(&bm.description.unwrap_or_else(|| "".to_string()))
					.to_string()
					.as_str(),
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

		match executor::block_on(self.index_writer.garbage_collect_files()) {
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
	tags_field: Field,
}

//this returns json for all methods just now. But when OPDF is implemented should make this more generic (return structs)
//and move JSON generation elsewhere
impl TantivyReader {
	pub fn new(index: String) -> Result<TantivyReader, StoreError> {
		let path = Path::new(&index);
		let mmap_dir = MmapDirectory::open(path)?;
		let index = Index::open(mmap_dir)?;
		let reader = index.reader_builder().reload_policy(ReloadPolicy::OnCommit).try_into()?;
		let searcher = reader.searcher();
		let schema = searcher.schema();
		let mut query_parser = QueryParser::for_index(
			&index,
			vec![
				index.schema().get_field("creator").unwrap(),
				index.schema().get_field("title").unwrap(),
				index.schema().get_field("description").unwrap(),
			],
		);
		query_parser.set_conjunction_by_default();
		Ok(TantivyReader {
			reader,
			query_parser,
			id_field: TantivyReader::get_field(schema, "id")?,
			tags_field: TantivyReader::get_field(schema, "tags")?,
		})
	}

	pub fn get_field(schema: &Schema, field_name: &str) -> Result<Field, StoreError> {
		match schema.get_field(field_name) {
			Some(f) => Ok(f),
			None => Err(StoreError::InitError(
				format!(
					"Mismatching schema - specified field {} does not exist in tantivy schema for this index.",
					field_name
				)
				.to_string(),
			)),
		}
	}

	//    /api/search
	pub fn search(&self, query: &str, start: usize, limit: usize) -> Result<SearchResult, StoreError> {
		let searcher = &self.reader.searcher();

		let tquery = &self.query_parser.parse_query(query)?;

		let top_collector = TopDocs::with_limit(start + limit);
		let count_collector = Count;
		let docs = searcher.search(tquery, &(top_collector, count_collector))?;
		let count = docs.0.len();

		let mut books = Vec::new();

		for (i, doc_addr) in docs.0.iter().enumerate() {
			if (i + 1) > start {
				let retrieved = match searcher.doc(doc_addr.1) {
					Ok(doc) => doc,
					Err(_) => continue,
				};

				books.push(self.to_bm(&retrieved, &searcher.schema()));
			}
		}

		Ok(SearchResult {
			count,
			start,
			query: query.to_string(),
			books,
		})
	}

	pub fn get_book(&self, id: i64) -> Option<BookMetadata> {
		let searcher = &self.reader.searcher();
		let id_term = Term::from_field_i64(self.id_field, id);
		let term_query = TermQuery::new(id_term, IndexRecordOption::Basic);

		//could this be better with TopFieldCollector which uses a FAST field?
		let maybedocs = searcher.search(&term_query, &TopDocs::with_limit(1));

		match maybedocs {
			Ok(docs) => {
				if docs.len() == 1 {
					match docs.first() {
						Some(doc_addr) => match searcher.doc(doc_addr.1) {
							Ok(doc) => Some(self.to_bm(&doc, searcher.schema())),
							Err(e) => {
								println!("Doc disappeared. id:{}, err: {}", id, e);
								None
							}
						},
						None => {
							println!("Doc disappeared. id:{}", id);
							None
						}
					}
				} else {
					println!("Found {} matching docs for supposedly unique id {}.", docs.len(), id);
					None
				}
			}
			Err(_) => None,
		}
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

//Reduce the search results to top categories with numbers of each
pub struct TopCategories {
	precision: usize, //1 means first letter, 2 means 2nd letter etc
}

impl TopCategories {
	pub fn with_precision(precision: usize) -> TopCategories {
		if precision < 1 {
			panic!("Precision must be > 0");
		}

		TopCategories { precision }
	}
}

impl Collector for TopCategories {
	type Fruit = HashSet<(char, usize)>; //FIXME actually a hashmap I guess is better

	type Child = TopCategoriesSegmentCollector;

	fn for_segment(&self, segment_local_id: SegmentLocalId, reader: &SegmentReader) -> tantivy::Result<Self::Child> {
		unimplemented!("A segment constructor I think?");
	}

	fn requires_scoring(&self) -> bool {
		false
	}

	fn merge_fruits(&self, child_fruits: Vec<HashSet<(char, usize)>>) -> tantivy::Result<Self::Fruit> {
		unimplemented!("Simply merge the hashsets via some util fn");
	}
}

pub struct TopCategoriesSegmentCollector {
	precision: usize,
	fruit: HashSet<(char, usize)>,
}

impl SegmentCollector for TopCategoriesSegmentCollector {
	type Fruit = HashSet<(char, usize)>;

	fn collect(&mut self, doc: DocId, score: Score) {
		unimplemented!("Grab each incoming doc here, extract field of interest, look at char at index precision, push to Vec?");
	}

	fn harvest(self) -> Self::Fruit {
		unimplemented!("Return the fruit");
	}
}
