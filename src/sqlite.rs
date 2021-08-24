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
}