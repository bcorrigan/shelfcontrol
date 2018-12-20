use std::error::Error;

use std::io;
use std::fs;
use std::process;
use std::path::Path;
use std::collections::HashMap;

//extern crate tantivy;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::IndexWriter;
use tantivy::directory::MmapDirectory;


use BookWriter;
use BookMetadata;

pub struct TantivyWriter {
    dir:String,
    index:Index,
    index_writer:IndexWriter,
    id:Field,
    title:Field,
    description:Field,
    publisher:Field,
    creator:Field,
    file:Field,
    filesize:Field,
    modtime:Field,
    pubdate:Field,
    moddate:Field,
    tags: Field,
}

impl TantivyWriter {
    pub fn new(dir: String) -> Result<TantivyWriter, tantivy::TantivyError> {
        if Path::new(&dir).exists() {
            println!("Error: Must remove directory {} to run.", &dir);
            process::exit(3);
        }
        fs::create_dir(&dir)?;

        //build our schema
        let mut schema_builder = SchemaBuilder::default();
        let id =schema_builder.add_i64_field("id", INT_STORED | INT_INDEXED);
        let title = schema_builder.add_text_field("title", TEXT | STORED);
        let description = schema_builder.add_text_field("description", TEXT | STORED);
        let publisher = schema_builder.add_text_field("publisher", TEXT | STORED);
        let creator = schema_builder.add_text_field("creator", TEXT | STORED);
        //subject
        let file = schema_builder.add_text_field("file", STRING | STORED);
        let filesize = schema_builder.add_i64_field("filesize", INT_STORED | INT_INDEXED);
        let modtime = schema_builder.add_i64_field("modtime", INT_STORED | INT_INDEXED);
        let pubdate = schema_builder.add_text_field("pubdate", TEXT | STORED);
        let moddate = schema_builder.add_text_field("moddate", TEXT | STORED);
        let tags = schema_builder.add_facet_field("tags");
        let schema = schema_builder.build();
		let path_dir = dir.clone();
        let path = Path::new(&path_dir);
		let mmap_dir = MmapDirectory::open(path)?;
        let index = Index::create(mmap_dir, schema.clone())?;
		let writer = index.writer(50_000_000)?;

        Ok( TantivyWriter {dir: dir,
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
                           tags: tags,
                           } )
    }
}

impl BookWriter for TantivyWriter {
    fn write_tags(&self, tags: HashMap<String, Vec<i64>>, limit:usize ) -> Result<(), Box<Error>> {
		//not sure if facets can be added later in tantivy??
        Ok(())
    }

    fn write_epubs(&mut self, bms: Vec<BookMetadata>, tags: &mut HashMap<String, Vec<i64>>) -> Result<(), Box<Error>> {
        for bm in bms {
            bm.add_tags(tags);

            let mut ttdoc = Document::default();
            ttdoc.add_i64( self.id, bm.id );
            ttdoc.add_text( self.title, &bm.title.unwrap_or("".to_string()) );
            ttdoc.add_text( self.description, &bm.description.unwrap_or("".to_string()));
            ttdoc.add_text( self.publisher, &bm.publisher.unwrap_or("".to_string()));
            ttdoc.add_text( self.creator, &bm.creator.unwrap_or("".to_string()));
            ttdoc.add_text( self.file, &bm.file);
            ttdoc.add_i64( self.filesize, bm.filesize);
            ttdoc.add_i64( self.modtime, bm.modtime);
            ttdoc.add_text( self.pubdate, &bm.pubdate.unwrap_or("".to_string()));
            ttdoc.add_text( self.moddate, &bm.moddate.unwrap_or("".to_string()));

            if bm.subject.is_some() {
                for subject in &bm.subject.unwrap() {
					if !&subject.trim().is_empty() {
						//println!("Making facet from: {}", &subject);
						let mut tag = "/".to_string();
						tag.push_str(&subject);
                    	ttdoc.add_facet( self.tags, Facet::from(tag.as_str()));
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
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError:")))
		}?;

		match self.index_writer.garbage_collect_files() {
			Ok(_) => Ok(()),
			Err(_) => Err(Box::new(io::Error::new(io::ErrorKind::Other, "TantivyError - garbage collect failed")))
		}
	}
}

pub struct TantivyReader {
	pub index:Index
}

impl TantivyReader {
	pub fn new(index:String) -> Result<TantivyReader, tantivy::TantivyError> {
		let path = Path::new(&index);
		let mmap_dir = MmapDirectory::open(path)?;

		Ok( TantivyReader {
			index: Index::open(mmap_dir)?
		})
	}
}
