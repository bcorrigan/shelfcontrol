use rusqlite::{params, Connection, Result};
use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;
use serde::Serialize;
use std::collections::HashMap;
use crate::{TagCount, AuthorCount, PublisherCount};
use crate::search_result::SearchResult;
pub trait DbInfo<T: std::fmt::Debug + Serialize> {
    fn new(key:String, count: u32) -> T;
	fn get_table() -> String;
	fn get_pkcol() -> String;
}

impl DbInfo<AuthorCount> for AuthorCount {
    fn new(key:String, count:u32) -> AuthorCount {
        AuthorCount {
            creator: key,
            count,
        }
    }

	fn get_table() -> String {
		"authors".to_string()
	}

	fn get_pkcol() -> String {
		"creator".to_string()
	}
}

impl DbInfo<TagCount> for TagCount {
    fn new(key:String, count:u32) -> TagCount {
        TagCount {
            tag: key,
            count,
        }
    }

	fn get_table() -> String {
		"tags".to_string()
	}

	fn get_pkcol() -> String {
		"tag".to_string()
	}
}

impl DbInfo<PublisherCount> for PublisherCount {
    fn new(key:String, count:u32) -> PublisherCount {
        PublisherCount {
            publisher: key,
            count,
        }
    }
    fn get_table() -> String {
        "publishers".to_string()
    }

    fn get_pkcol() -> String {
        "publisher".to_string()
    }
}

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

    pub fn make_db(&self) -> Result<(), rusqlite::Error> {
        self.create_table::<AuthorCount>()?;
        self.create_table::<PublisherCount>()?;
        self.create_table::<TagCount>()?;
        Ok(())
    }

    fn create_table<T: DbInfo<T> + std::fmt::Debug + Serialize>(&self) -> Result<(), rusqlite::Error> {
        let conn = self.pool.get().unwrap();
        conn.execute(
            &format!("CREATE TABLE {} (
                    {} TEXT primary key,
                    count INTEGER
            )", T::get_table(), T::get_pkcol() ), 
    [],
        )?;

        Ok(())
    }

    pub fn write_counts<T: DbInfo<T> + std::fmt::Debug + Serialize>(&self, counts: HashMap<String, u32>) -> Result<(), rusqlite::Error> {
        for (key, count) in counts {
            self.pool.get().unwrap().execute(
                &format!("INSERT INTO {}({}, count) values (?1, ?2)", T::get_table(), T::get_pkcol()),
                params![key,count],
            )?;
        }
        Ok(())
    }

    fn get_count_sql(&self, order_by_count:bool, asc:bool, where_clause: bool, offset:u32, count:u32, table:&str, field:&str) -> String {
        let where_clause = if where_clause {format!(" where {} like ?", field)} else {"".to_string()};
        let order_by = if order_by_count {" order by count".to_string()} else {format!(" order by {}", field)};
        let ascdesc = if asc { " ASC" } else { " DESC" };
        format!("select *, count(*) OVER() from {} {} {} {} limit {}, {}", table, where_clause, order_by, ascdesc, offset, count)
    }

    pub fn get_counts<T: DbInfo<T> + std::fmt::Debug + Serialize>(&self, order_by_count:bool, asc:bool, offset:u32, count:u32, filter:Option<String>) -> Result<SearchResult<T>, rusqlite::Error> {
        let conn = self.pool.get().unwrap();
        let query = &self.get_count_sql(order_by_count, asc, filter.is_some(), offset, count, &T::get_table(), &T::get_pkcol());
        let mut stmt = conn.prepare(query)?;
        let mut fullcount=0;
        let new_str = String::new();

        let some_params = params![format!("%{}%", filter.as_ref().unwrap_or(&new_str))];
        let params = if filter.is_some() {some_params} else {params![]};
        let payload:Vec<T> = stmt.query_map(params, |row| {
            if fullcount!=0 {
                fullcount=row.get(3)?;
            }
            Ok(T::new(
                row.get(0)?, 
                row.get(1)?
            ))        
        })?.filter_map(|t| t.ok()).collect();

        Ok(SearchResult {
            count: fullcount,
            start: offset as usize,
            query: filter,
            payload,
        })
    }
    /*handy queries
    select * from tags where tag like "%lovecraft%" order by count desc limit 20,20;
    limit term is skip,count

    What if we want the total count when limited?

    try window function:

    select *, count(*) OVER() as full_count from tags where tag like "%lovecraft%" order by count desc limit 20,20;

    This seems to be pretty fast and does it in one query.
    */
}