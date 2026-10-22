//! Research-only CLI: the production lexical expression against an isolated in-memory FTS.
#[path = "../src/lexical_search.rs"]
mod lexical_search;
use std::io::Read;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut text=String::new();std::io::stdin().take(2*1024*1024).read_to_string(&mut text)?;
    let input:Value=serde_json::from_str(&text)?;
    let db=Connection::open_in_memory()?;
    db.execute_batch("CREATE VIRTUAL TABLE documents USING fts5(key UNINDEXED,description,group_name UNINDEXED,tokenize='unicode61 remove_diacritics 2');")?;
    for document in input["corpus"].as_array().ok_or("Missing corpus")? {
        db.execute("INSERT INTO documents VALUES(?1,?2,?3)",params![document["key"].as_str(),document["description"].as_str(),document["group"].as_str()])?;
    }
    let mut baselines=Vec::new();
    for expand in [false,true] {
        let mut results=Vec::new();
        for sample in input["queries"].as_array().ok_or("Missing queries")? {
            let query=sample["query"].as_str().ok_or("Missing query")?;
            let terms=query.split(|c:char|!c.is_alphanumeric()).filter(|s|!s.is_empty()).take(32).map(str::to_lowercase).collect::<Vec<_>>();
            let expression=lexical_search::expression(&terms,expand);
            let group=sample["group"].as_str();
            let started=std::time::Instant::now();
            let mut statement=db.prepare("SELECT key,bm25(documents) FROM documents WHERE documents MATCH ?1 AND (?2 IS NULL OR group_name=?2) ORDER BY bm25(documents),key")?;
            let ranked=statement.query_map(params![expression,group],|row|Ok(json!({"key":row.get::<_,String>(0)?,"score":row.get::<_,f64>(1)?})))?.collect::<Result<Vec<_>,_>>()?;
            results.push(json!({"query":query,"group":group,"ranked":ranked,"ms":started.elapsed().as_secs_f64()*1000.0}));
        }
        baselines.push(json!({"mode":if expand{"production-lexical-expansion"}else{"literal-FTS"},"results":results}));
    }
    println!("{}",json!({"baselines":baselines,"scope":"Production lexical expression reused in a research-only CLI and in-memory FTS; not a native UI acceptance."}));
    Ok(())
}
