use rusqlite::{Connection, Error};

pub fn query_first_field(conn: &Connection, sql: &str) -> Result<String, Error> {
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query([])?;

    let row = rows.next()?;

    let value: String = row.as_ref().unwrap().get(0)?;
    
    Ok(value) 
}
