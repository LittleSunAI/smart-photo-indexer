//! Windows-style, case/accent-insensitive numeric names, shared with the UI.
//! An explicit Latin-before-Cyrillic locale avoids OS/ICU locale-default drift.
use rusqlite::{Connection,functions::FunctionFlags};

#[cfg(windows)]
fn key(text:&str)->Result<Vec<u8>,String>{
    use windows_sys::Win32::Globalization::{LCMapStringEx,LCMAP_SORTKEY,NORM_IGNORECASE,NORM_IGNORENONSPACE,SORT_DIGITSASNUMBERS};
    if text.is_empty(){return Ok(vec![0]);}
    let locale:Vec<u16>="en-US\0".encode_utf16().collect();
    let source:Vec<u16>=text.encode_utf16().collect();
    let count=i32::try_from(source.len()).map_err(|_|"Sort input too long")?;
    let flags=LCMAP_SORTKEY|NORM_IGNORECASE|NORM_IGNORENONSPACE|SORT_DIGITSASNUMBERS;
    let needed=unsafe{LCMapStringEx(locale.as_ptr(),flags,source.as_ptr(),count,std::ptr::null_mut(),0,std::ptr::null(),std::ptr::null(),0)};
    if needed<=0{return Err("Windows name comparison failed".into());}
    let mut result=vec![0u8;needed as usize];
    let written=unsafe{LCMapStringEx(locale.as_ptr(),flags,source.as_ptr(),count,result.as_mut_ptr().cast(),needed,std::ptr::null(),std::ptr::null(),0)};
    if written!=needed{return Err("Windows name comparison failed".into());}
    Ok(result)
}

#[cfg(not(windows))]
fn key(text:&str)->Result<Vec<u8>,String>{Ok(text.to_lowercase().into_bytes())}

pub(crate) fn register(db:&Connection)->Result<(),String>{
    db.create_scalar_function("photo_sort_key",1,FunctionFlags::SQLITE_UTF8|FunctionFlags::SQLITE_DETERMINISTIC,|ctx|{
        let text=ctx.get::<String>(0)?;
        key(&text).map_err(|error|rusqlite::Error::UserFunctionError(std::io::Error::other(error).into()))
    }).map_err(|error|error.to_string())
}

#[cfg(all(test,windows))]
mod tests{
    use super::*;
    #[test]
    fn natural_russian_names_and_page_boundaries(){
        let db=Connection::open_in_memory().unwrap();register(&db).unwrap();
        db.execute_batch("CREATE TABLE names(name TEXT);INSERT INTO names VALUES('фото10.png'),('Фото2.png'),('фото1.png'),('z10.jpg'),('z2.jpg');").unwrap();
        let read=|offset|db.prepare("SELECT name FROM names ORDER BY photo_sort_key(name) LIMIT 2 OFFSET ?1").unwrap().query_map([offset],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(read(0),vec!["z2.jpg","z10.jpg"]);
        assert_eq!(read(2),vec!["фото1.png","Фото2.png"]);
        assert_eq!(read(4),vec!["фото10.png"]);
        assert_eq!(key("ФОТО2.png").unwrap(),key("фото2.png").unwrap());
    }
    #[test]
    fn processed_date_sort_keeps_null_last_in_both_directions(){
        let mut db=Connection::open_in_memory().unwrap();crate::run_index_migrations(&mut db).unwrap();
        for (id,time) in [("new",Some(200)),("old",Some(100)),("unknown",None)]{
            db.execute("INSERT INTO images(id,identity_key,file_name,ai_processed_at_unix,created_at_unix,updated_at_unix) VALUES(?1,?1,?1,?2,1,1)",rusqlite::params![id,time]).unwrap();
        }
        for (direction,expected) in [("ASC",vec!["old","new","unknown"]),("DESC",vec!["new","old","unknown"]) ]{
            let ids=db.prepare(&format!("SELECT id FROM images ORDER BY ai_processed_at_unix IS NULL,ai_processed_at_unix {direction}")).unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();assert_eq!(ids,expected);
        }
    }
}
