use rusqlite::{params, Connection, Result};
use std::error::Error;

use crate::BookWriter;

pub struct SqlWriter {
    conn: Connection
}

impl SqlWriter {
    pub fn new(dir: String) -> Result<SqlWriter, rusqlite::Error> {
        let conn = Connection::open(&dir)?;

        conn.execute(
            "CREATE TABLE books (
                    id  INTEGER primary key,
                    creator TEXT,
                    publisher TEXT
            )", 
    [],
        )?;

        Ok(SqlWriter { 
            conn
        })
    }
}