use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

pub struct SqlWriter {
    conn: Connection
}

impl SqlWriter {
    pub fn new(dir: &String) -> Result<SqlWriter, rusqlite::Error> {
        let conn = Connection::open(&dir)?;

        conn.execute(
            "CREATE TABLE authors (
                    creator TEXT primary key,
                    count INTEGER
            )", 
    [],
        )?;

        conn.execute(
            "CREATE TABLE publishers (
                    publisher TEXT primary key,
                    count INTEGER
            )", 
    [],
        )?;

        conn.execute(
            "CREATE TABLE tags (
                    tag TEXT primary key,
                    count INTEGER
            )", 
    [],
        )?;

        Ok(SqlWriter { 
            conn
        })
    }

    pub fn write_creator_counts(&self, creator_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (creator, count) in creator_counts {
            self.conn.execute(
                "INSERT INTO authors(creator, count) values (?1, ?2)",
                params![creator,count],
            )?;
        }
        Ok(())
    }

    pub fn write_publisher_counts(&self, publisher_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (publisher, count) in publisher_counts {
            self.conn.execute(
                "INSERT INTO publishers(publisher, count) values (?1, ?2)",
                params![publisher,count],
            )?;
        }
        Ok(())
    }

    pub fn write_tag_counts(&self, tag_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (tag, count) in tag_counts {
            self.conn.execute(
                "INSERT INTO tags(tag, count) values (?1, ?2)",
                params![tag,count],
            )?;
        }
        Ok(())
    }
}