use std::error::Error;

use std::collections::HashMap;

use std::fs;
use std::io;
use std::path::Path;
use std::process;

use tantivy::collector::{Collector, Count, SegmentCollector, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::TermQuery;
use tantivy::schema::*;
use tantivy::store::Compressor;
use tantivy::store::StoreReader;
use tantivy::DocId;
use tantivy::IndexWriter;
use tantivy::Score;
use tantivy::SegmentOrdinal;
use tantivy::SegmentReader;
use tantivy::{Index, IndexReader, IndexSettings, IndexSortByField, Order, ReloadPolicy};

use crate::error::StoreError;
use crate::search_result::{Category, CategorySearchResult, SearchResult};
use crate::BookMetadata;
use crate::BookWriter;
use ammonia::{Builder, UrlRelative};
use futures::executor;
use tantivy::query::QueryParser;
use time::serde::rfc3339::deserialize;
use time::{Duration, OffsetDateTime};

pub struct TantivyWriter<'a> {
	index_writer: std::sync::RwLock<IndexWriter>,
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
	pub fn new(dir: &String) -> Result<TantivyWriter<'a>, tantivy::TantivyError> {
		if Path::new(&dir).exists() {
			println!("Error: Must remove directory {} to run.", &dir);
			process::exit(3);
		}
		fs::create_dir(&dir)?;

		//build our schema
		let mut schema_builder = SchemaBuilder::default();
		//let id_options = IntOptions::default().set_stored().set_indexed();
		let id = schema_builder.add_i64_field("id", NumericOptions::default().set_stored().set_indexed());
		let title = schema_builder.add_text_field("title", TEXT | STORED);
		let description = schema_builder.add_text_field("description", TEXT | STORED);
		let publisher = schema_builder.add_text_field("publisher", TEXT | STORED);
		let creator = schema_builder.add_text_field("creator", TEXT | STORED);
		//subject
		let file = schema_builder.add_text_field("file", STRING | STORED);
		let filesize = schema_builder.add_i64_field("filesize", NumericOptions::default().set_stored().set_indexed());
		//let modtime = schema_builder.add_i64_field("modtime", IntOptions::default().set_stored().set_indexed().set_fast(Cardinality::SingleValue));
		let modtime = schema_builder.add_date_field("modtime", FAST | STORED);
		let pubdate = schema_builder.add_text_field("pubdate", TEXT | STORED);
		let moddate = schema_builder.add_text_field("moddate", TEXT | STORED);
		//let moddate = schema_builder.add_date_field("moddate", STORED | INDEXED);
		let cover_mime = schema_builder.add_text_field("cover_mime", TEXT | STORED);
		let tags = schema_builder.add_facet_field("tags", STORED | INDEXED);
		let schema = schema_builder.build();
		let path_dir = dir.clone();
		let path = Path::new(&path_dir);
		//let mmap_dir = MmapDirectory::open(path)?;
		/*let index_settings = IndexSettings {
			sort_by_field: Some(IndexSortByField {
				field: "modtime".to_string(),
				order: Order::Desc,
			}),
			docstore_compression: Compressor::Lz4,
			docstore_blocksize: 16384,
			docstore_compress_dedicated_thread: true,
		};*/
		let index = Index::create_in_dir(path, schema.clone())?;
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
			index_writer: std::sync::RwLock::new(writer),
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
	fn write_epubs(&mut self, bms: &Vec<BookMetadata>) -> Result<(), Box<dyn Error>> {
		let empty_str = String::new();
		for bm in bms {
			let mut ttdoc = TantivyDocument::default();
			ttdoc.add_i64(self.id, bm.id);
			ttdoc.add_text(self.title, bm.title.as_ref().unwrap_or(&empty_str));
			ttdoc.add_text(
				self.description,
				self.sanitiser
					.clean(bm.description.as_ref().unwrap_or(&empty_str))
					.to_string()
					.as_str(),
			);
			ttdoc.add_text(self.publisher, bm.publisher.as_ref().unwrap_or(&empty_str));
			ttdoc.add_text(self.creator, bm.creator.as_ref().unwrap_or(&empty_str));
			ttdoc.add_text(self.file, &bm.file);
			ttdoc.add_i64(self.filesize, bm.filesize);

			ttdoc.add_date(self.modtime, tantivy::DateTime::from_utc(bm.modtime));
			ttdoc.add_text(self.pubdate, bm.pubdate.as_ref().unwrap_or(&empty_str));
			ttdoc.add_text(self.moddate, &bm.moddate.as_ref().unwrap_or(&empty_str));
			ttdoc.add_text(self.cover_mime, &bm.cover_mime.as_ref().unwrap_or(&empty_str));

			if bm.subject.is_some() {
				let mut tagsmap = HashMap::new();
				bm.add_tags(&mut tagsmap);

				for tag in tagsmap.keys() {
					if !tag.is_empty() {
						let mut tagprefix = "/".to_string();
						tagprefix.push_str(tag);
						ttdoc.add_facet(self.tags, Facet::from(tagprefix.as_str()));
					}
				}
			}

			self.index_writer.write().unwrap().add_document(ttdoc);

			//tags can probably be "facets" in tantivy, see: https://github.com/tantivy-search/tantivy/issues/215
			//this appears ot be only way to support multiple of them
		}
		Ok(())
	}

	fn commit(&mut self) -> Result<(), Box<dyn Error>> {
		match self.index_writer.write().unwrap().commit() {
			Ok(_) => Ok(()),
			Err(e) => Err(Box::new(io::Error::new(io::ErrorKind::Other, e))),
		}?;

		match executor::block_on(self.index_writer.write().unwrap().garbage_collect_files()) {
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
		let reader = index.reader_builder().reload_policy(ReloadPolicy::OnCommitWithDelay).try_into()?;
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
			Ok(f) => Ok(f),
			Err(_) => Err(StoreError::InitError(
				format!(
					"Mismatching schema - specified field {} does not exist in tantivy schema for this index.",
					field_name
				)
				.to_string(),
			)),
		}
	}

	//    /api/search
	pub fn search(&self, query: &str, start: usize, limit: usize) -> Result<SearchResult<BookMetadata>, StoreError> {
		let searcher = &self.reader.searcher();

		let tquery = &self.query_parser.parse_query(query)?;

		let top_collector = TopDocs::with_limit(start + limit);
		let count_collector = Count;
		let docs = searcher.search(tquery, &(top_collector, count_collector))?;
		let count = docs.1;

		let mut books = Vec::new(); //0 {}[]

		for doc_addr in docs.0.iter().skip(start) {
			let retrieved = match searcher.doc(doc_addr.1) {
				Ok(doc) => doc,
				Err(_) => continue,
			};

			books.push(self.to_bm(&retrieved, &searcher.schema()));
		}

		Ok(SearchResult {
			count,
			start,
			query: Some(query.to_string()),
			payload: books,
		})
	}

	pub fn categorise(&self, field: &str, prefix: &str, query: Option<&str>, floor: usize) -> Result<CategorySearchResult, StoreError> {
		let searcher = self.reader.searcher();
		let fld = TantivyReader::get_field(searcher.schema(), field)?;

		let cat_collector = AlphabeticalCategories::new(prefix.len() + 1, fld, prefix);
		let query = match query {
			Some(q) => self.query_parser.parse_query(q)?,
			None => Box::new(tantivy::query::RegexQuery::from_pattern(
				&format!("{}.*", prefix.to_ascii_lowercase()),
				fld,
			)?), //TODO case sensitivity
		};

		//let count = searcher.search(&query, &tantivy::collector::Count)?;
		//println!("Query returns {}", count);

		let cats = searcher.search(&query, &cat_collector)?;
		let mut cats_vec: Vec<Category> = cats
			.iter()
			.map(|(k, v)| Category {
				prefix: {
					let mut prefix = prefix.to_owned();
					prefix.push_str(&k.to_string());
					prefix
				},
				count: *v,
			})
			.filter(|f| f.count > floor)
			.collect();

		cats_vec.sort_by(|a, b| a.prefix.cmp(&b.prefix));

		Ok(CategorySearchResult {
			count: cats_vec.len(),
			categories: cats_vec,
		})
	}

	pub fn count_by_field(&self, field: &str, prefix: &str) -> Result<CategorySearchResult, StoreError> {
		let searcher = self.reader.searcher();
		let prefix = prefix.to_ascii_lowercase();
		let fld = TantivyReader::get_field(searcher.schema(), field)?;

		let fld_collector = FieldCategories::new(fld);
		let query: Box<dyn tantivy::query::Query> = Box::new(tantivy::query::RegexQuery::from_pattern(&format!("{}.*", prefix), fld)?);

		//let count = searcher.search(&query, &tantivy::collector::Count)?;
		//println!("Query {:?} returns {}", query, count);

		let cats = searcher.search(&query, &fld_collector)?;

		let mut cats_vec: Vec<Category> = cats
			.iter()
			.map(|(k, v)| {
				if k.to_ascii_lowercase().starts_with(&prefix) {
					Some(Category {
						prefix: { k.to_string() },
						count: *v,
					})
				} else {
					None
				}
			})
			.filter(|c| c.is_some())
			.map(|c| c.unwrap())
			.collect();

		cats_vec.sort_by(|a, b| a.prefix.cmp(&b.prefix));

		Ok(CategorySearchResult {
			count: cats_vec.len(),
			categories: cats_vec,
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

	fn to_bm(&self, doc: &tantivy::TantivyDocument, schema: &Schema) -> BookMetadata {
		BookMetadata {
			id: self.get_doc_i64("id", &doc, &schema), //not populated ?
			title: self.get_doc_str("title", &doc, &schema),
			description: self.get_doc_str("description", &doc, &schema),
			publisher: self.get_doc_str("publisher", &doc, &schema),
			creator: self.get_doc_str("creator", &doc, &schema),
			subject: self.get_tags("tags", &doc),
			file: self.get_doc_str("file", &doc, &schema).unwrap(),
			filesize: self.get_doc_i64("filesize", &doc, &schema),
			modtime: self.get_doc_datetime("modtime", &doc, &schema),
			pubdate: self.get_doc_str("pubdate", &doc, &schema),
			moddate: self.get_doc_str("moddate", &doc, &schema),
			cover_mime: self.get_doc_str("cover_mime", &doc, &schema),
		}
	}

	//I *know* the fields are present in schema, and I *know* that certain fields eg id are always populated, so just unwrap() here
	fn get_doc_str(&self, field: &str, doc: &tantivy::TantivyDocument, schema: &Schema) -> Option<String> {
		doc.get_first(schema.get_field(field).unwrap()).map(|val| match val.as_str() {
			Some(t) => t.to_string(),
			_ => "".to_string(),
		})
	}

	fn get_doc_i64(&self, field: &str, doc: &tantivy::TantivyDocument, schema: &Schema) -> i64 {
		doc.get_first(schema.get_field(field).unwrap()).unwrap().as_i64().unwrap()
	}

	fn get_doc_datetime(&self, field: &str, doc: &tantivy::TantivyDocument, schema: &Schema) -> OffsetDateTime {
		doc.get_first(schema.get_field(field).unwrap())
			.unwrap()
			.as_datetime()
			.unwrap()
			.into_utc()
	}

	fn get_tags(&self, _field: &str, doc: &tantivy::TantivyDocument) -> Option<Vec<String>> {
		let vals = doc.get_all(self.tags_field);

		let mut tags = Vec::new();

		for v in vals {
			//println!("{:?}", (v as &Facet));
			tags.push(v.as_facet().unwrap().encoded_str().to_string());
			//tags.push(v.as_str().unwrap_or("").to_string());
		}

		if tags.len() == 0 {
			return None;
		}

		Some(tags)
	}
}

//Reduce the search results to top categories with numbers of each
pub struct AlphabeticalCategories<'a> {
	char_position: usize, //1 means first letter, 2 means 2nd letter etc
	category_field: Field,
	prefix: &'a str,
}

impl<'a> AlphabeticalCategories<'a> {
	pub fn new(char_position: usize, category_field: Field, prefix: &'a str) -> AlphabeticalCategories<'a> {
		if char_position < 1 {
			panic!("Position must be positive.");
		}

		//println!("Created AlphabeticalCategories({:?}, {:?})", char_position, category_field);

		AlphabeticalCategories {
			char_position,
			category_field,
			prefix,
		}
	}
}

impl<'a> Collector for AlphabeticalCategories<'a> {
	type Fruit = HashMap<char, usize>;

	type Child = AlphabeticalCategoriesSegmentCollector;

	fn for_segment(&self, _: SegmentOrdinal, segment_reader: &SegmentReader) -> tantivy::Result<Self::Child> {
		Ok(AlphabeticalCategoriesSegmentCollector::new(
			self.char_position,
			self.category_field,
			segment_reader,
			self.prefix.to_owned(),
		))
	}

	fn requires_scoring(&self) -> bool {
		false
	}

	fn merge_fruits(&self, child_fruits: Vec<HashMap<char, usize>>) -> tantivy::Result<Self::Fruit> {
		let mut merged: HashMap<char, usize> = HashMap::new();

		for fruit in child_fruits {
			for (letter, count) in fruit {
				if merged.contains_key(&letter) {
					merged.insert(letter, merged.get(&letter).unwrap() + count);
				} else {
					merged.insert(letter, count);
				}
			}
		}

		Ok(merged)
	}
}

pub struct AlphabeticalCategoriesSegmentCollector {
	char_position: usize,
	category_field: Field,
	fruit: HashMap<char, usize>,
	store_reader: StoreReader,
	prefix: String,
}

impl AlphabeticalCategoriesSegmentCollector {
	pub fn new(
		char_position: usize,
		category_field: Field,
		segment_reader: &SegmentReader,
		prefix: String,
	) -> AlphabeticalCategoriesSegmentCollector {
		AlphabeticalCategoriesSegmentCollector {
			char_position,
			category_field,
			fruit: HashMap::new(),
			store_reader: segment_reader.get_store_reader(100).unwrap(), //FIXME no earthly idea what cache_num_store_blocks is
			prefix,
		}
	}
}

impl SegmentCollector for AlphabeticalCategoriesSegmentCollector {
	type Fruit = HashMap<char, usize>;

	fn collect(&mut self, doc: DocId, _: Score) {
		//segmentReader.get_store_reader().get(docId) => slow (returns LZ4 block to decompress!)
		//If it is a facet - segmentReader.facet_reader() then facet_reader.facet_ords() & facet_from_ords()
		let document: TantivyDocument = self.store_reader.get(doc).unwrap();
		let field_text = document.get_first(self.category_field).unwrap().as_str().unwrap();
		//println!("pos: {} text:{:?}:", self.char_position, &field_text.chars());
		//not populated - just ignore it

		if field_text.to_ascii_uppercase().starts_with(&self.prefix) {
			match field_text.chars().nth(self.char_position - 1) {
				Some(char) => self.fruit.insert(
					char.to_ascii_uppercase(),
					self.fruit.get(&char.to_ascii_uppercase()).unwrap_or(&0) + 1,
				),
				None => None,
			};
		}
	}

	fn harvest(self) -> Self::Fruit {
		self.fruit
	}
}

pub struct FieldCategories {
	category_field: Field,
}

impl FieldCategories {
	pub fn new(category_field: Field) -> FieldCategories {
		FieldCategories { category_field }
	}
}

impl Collector for FieldCategories {
	type Fruit = HashMap<String, usize>;

	type Child = FieldCategoriesSegmentCollector;

	fn for_segment(&self, _: SegmentOrdinal, segment_reader: &SegmentReader) -> tantivy::Result<Self::Child> {
		Ok(FieldCategoriesSegmentCollector::new(self.category_field, segment_reader))
	}

	fn requires_scoring(&self) -> bool {
		false
	}

	fn merge_fruits(&self, child_fruits: Vec<HashMap<String, usize>>) -> tantivy::Result<Self::Fruit> {
		let mut merged: HashMap<String, usize> = HashMap::new();

		for fruit in child_fruits {
			for (field_val, count) in fruit {
				if merged.contains_key(&field_val) {
					let other_count = merged.get(&field_val).unwrap().clone();
					merged.insert(field_val, other_count + count);
				} else {
					merged.insert(field_val, count);
				}
			}
		}

		Ok(merged)
	}
}

pub struct FieldCategoriesSegmentCollector {
	category_field: Field,
	fruit: HashMap<String, usize>,
	store_reader: StoreReader,
}

impl FieldCategoriesSegmentCollector {
	pub fn new(category_field: Field, segment_reader: &SegmentReader) -> FieldCategoriesSegmentCollector {
		FieldCategoriesSegmentCollector {
			category_field,
			fruit: HashMap::new(),
			store_reader: segment_reader.get_store_reader(100).unwrap(),
		}
	}
}

impl SegmentCollector for FieldCategoriesSegmentCollector {
	type Fruit = HashMap<String, usize>;

	fn collect(&mut self, doc: DocId, _: Score) {
		//segmentReader.get_store_reader().get(docId) => slow (returns LZ4 block to decompress!)
		//If it is a facet - segmentReader.facet_reader() then facet_reader.facet_ords() & facet_from_ords()
		let document: TantivyDocument = self.store_reader.get(doc).unwrap();
		let field_text = document.get_first(self.category_field).unwrap().as_str().unwrap();
		//println!("pos: {} text:{:?}:", self.char_position, &field_text.chars());
		//not populated - just ignore it
		self.fruit
			.insert(field_text.to_string(), self.fruit.get(field_text).unwrap_or(&0) + 1);
	}

	fn harvest(self) -> Self::Fruit {
		self.fruit
	}
}
