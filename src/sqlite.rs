use rusqlite::{params, Connection, Result};
use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;
use std::collections::HashMap;
use crate::{TagCount, AuthorCount, PublisherCount};

pub struct Sqlite {
    pool: Pool<SqliteConnectionManager>,
}

impl Sqlite {
    pub fn new(dir: &String) -> Result<Sqlite, rusqlite::Error> {
        let manager = SqliteConnectionManager::file(dir);
        let pool = r2d2::Pool::new(manager).unwrap();

        Ok(Sqlite { 
            pool
        })
    }

    pub fn create_tables(&self) -> Result<(), rusqlite::Error> {
        let conn = self.pool.get().unwrap();
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

        Ok(())
    }

    pub fn write_creator_counts(&self, creator_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (creator, count) in creator_counts {
            self.pool.get().unwrap().execute(
                "INSERT INTO authors(creator, count) values (?1, ?2)",
                params![creator,count],
            )?;
        }
        Ok(())
    }

    pub fn write_publisher_counts(&self, publisher_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (publisher, count) in publisher_counts {
            self.pool.get().unwrap().execute(
                "INSERT INTO publishers(publisher, count) values (?1, ?2)",
                params![publisher,count],
            )?;
        }
        Ok(())
    }

    pub fn write_tag_counts(&self, tag_counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (tag, count) in tag_counts {
            self.pool.get().unwrap().execute(
                "INSERT INTO tags(tag, count) values (?1, ?2)",
                params![tag,count],
            )?;
        }
        Ok(())
    }

    fn get_count_sql(&self, order_by_count:bool, desc:bool, where_clause: bool, offset:u32, count:u32, table:&str, field:&str) -> String {
        let where_clause = if where_clause {format!(" where {} like ?", field)} else {"".to_string()};
        let order_by = if order_by_count {" order by count".to_string()} else {format!(" order by {}", field)};
        let ascdesc = if desc { " DESC" } else { " ASC" };
        format!("select * from {} {} {} {} limit {}, {}", table, where_clause, order_by, ascdesc, offset, count)
    }

    pub fn get_tags(&self, order_by_count:bool, desc:bool, offset:u32, count:u32, filter:Option<String>) -> Result<Vec<TagCount>, rusqlite::Error> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(&self.get_count_sql(order_by_count, desc, filter.is_some(), offset, count, "tags", "tag"))?;
        let x:Vec<TagCount> = stmt.query_map(params![filter.unwrap_or(String::new())], |row| {
            Ok(TagCount {
                tag: row.get(0)?,
                count: row.get(1)?,
            })
        })?.filter_map(|t| t.ok()).collect();
        
        Ok(x)
    }

    pub fn get_authors(&self, order_by_count:bool, desc:bool, offset:u32, count:u32, filter:Option<String>) -> Result<Vec<AuthorCount>, rusqlite::Error> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(&self.get_count_sql(order_by_count, desc, filter.is_some(), offset, count, "authors", "creator"))?;
        let x:Vec<AuthorCount> = stmt.query_map(params![filter.unwrap_or(String::new())], |row| {
            Ok(AuthorCount {
                creator: row.get(0)?,
                count: row.get(1)?,
            })
        })?.filter_map(|t| t.ok()).collect();
        
        Ok(x)
    }

    pub fn get_publishers(&self, order_by_count:bool, desc:bool, offset:u32, count:u32, filter:Option<String>) -> Result<Vec<PublisherCount>, rusqlite::Error> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(&self.get_count_sql(order_by_count, desc, filter.is_some(), offset, count, "publishers", "publisher"))?;
        let x:Vec<PublisherCount> = stmt.query_map(params![filter.unwrap_or(String::new())], |row| {
            Ok(PublisherCount {
                publisher: row.get(0)?,
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