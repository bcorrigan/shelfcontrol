use itertools::Itertools;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use crate::TagCount;

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

    pub fn get_tags(&self, order_by_count:bool, desc:bool, offset:u32, count:u32, filter:Option<String>) -> Result<Vec<TagCount>, rusqlite::Error> {
        let where_clause = if filter.is_some() {" where tag like ?"} else {""};
        let order_by = if order_by_count {" order by count"} else {" order by tag"};
        let ascdesc = if desc { " DESC" } else { " ASC" };
        let mut stmt = self.conn.prepare(&format!("select * from tags {} {} {} limit {}, {}", where_clause, order_by, ascdesc, offset, count))?;
        let x:Vec<TagCount> = stmt.query_map(params![filter.unwrap_or(String::new())], |row| {
            Ok(TagCount {
                tag: row.get(0)?,
                count: row.get(1)?,
            })
        })?.filter_map(|t| t.ok()).collect();
        
        Ok(x)
    }
    /*handy queries
    select * from tags where tag like "%lovecraft%" order by count desc limit 20,20;
    limit term is skip,count
    */
}