//! Query-only word forms and a small explicit dictionary; this is not semantic search.
use rust_stemmers::{Algorithm,Stemmer};
use std::collections::BTreeSet;
use std::sync::OnceLock;
fn groups()->&'static Vec<Vec<String>>{
    static GROUPS:OnceLock<Vec<Vec<String>>>=OnceLock::new();
    GROUPS.get_or_init(||serde_json::from_str(include_str!("../../src/shared/search/lexical-synonyms.json")).expect("Bundled lexical dictionary"))
}
fn stem(word:&str)->String{
    static RUSSIAN:OnceLock<Stemmer>=OnceLock::new();static ENGLISH:OnceLock<Stemmer>=OnceLock::new();
    if word.chars().all(|c|matches!(c,'а'..='я'|'ё')){RUSSIAN.get_or_init(||Stemmer::create(Algorithm::Russian)).stem(&word.replace('ё',"е")).into_owned()}
    else if word.chars().all(|c|c.is_ascii_lowercase()){ENGLISH.get_or_init(||Stemmer::create(Algorithm::English)).stem(word).into_owned()}
    else{word.into()}
}
pub fn expression(terms:&[String],expanded:bool)->String{
    if !expanded{return terms.iter().map(|s|format!("\"{}\"*",s.replace('"',"\"\""))).collect::<Vec<_>>().join(" OR ");}
    let mut patterns=BTreeSet::new();
    for word in terms{
        let query_stem=stem(&word.replace('ё',"е"));
        let mut forms=vec![word.clone(),word.replace('ё',"е")];
        for group in groups(){
            if group.iter().any(|alias|stem(&alias.replace('ё',"е"))==query_stem){forms.extend(group.iter().cloned());}
        }
        for form in forms{
            let root=stem(&form);
            // Ambiguous short stems use complete dictionary forms, avoiding котёл/carpet.
            patterns.insert(format!("\"{}\"",form));
            if root.chars().count()>=3 && !["кот","пес","пёс","дет","cat","car","dog"].contains(&root.as_str()){
                patterns.insert(format!("\"{root}\"*"));
                // Snowball normalizes ё to е; unicode61 keeps Cyrillic ё in document tokens.
                if form.contains('ё'){
                    let literal_root:String=form.chars().take(root.chars().count()).collect();
                    if literal_root.replace('ё',"е")==root{patterns.insert(format!("\"{literal_root}\"*"));}
                }
            }
        }
        if word.chars().count()>=4{patterns.insert(format!("\"{word}\"*"));}
    }
    patterns.into_iter().collect::<Vec<_>>().join(" OR ")
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn russian_word_forms_dictionary_and_yo_do_not_turn_short_animal_stems_into_boilers(){
        let db=rusqlite::Connection::open_in_memory().unwrap();db.execute_batch("CREATE VIRTUAL TABLE sample USING fts5(description); INSERT INTO sample VALUES('Кошка возле зелёной ёлки'),('Кот на диване'),('Котёл в мастерской');").unwrap();
        for query in ["кошку","котами","елке"]{
            let found:i64=db.query_row("SELECT COUNT(*) FROM sample WHERE sample MATCH ?1",[expression(&[query.into()],true)],|r|r.get(0)).unwrap();
            assert_eq!(found,if query=="елке"{1}else{2},"{query}");
        }
        let strict:i64=db.query_row("SELECT COUNT(*) FROM sample WHERE sample MATCH ?1",[expression(&["кошку".into()],false)],|r|r.get(0)).unwrap();assert_eq!(strict,0);
    }
}
