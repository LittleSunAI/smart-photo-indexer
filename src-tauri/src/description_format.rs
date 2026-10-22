//! A bounded, typed document. No HTML, scripts, CSS or arbitrary attributes are stored.
use crate::*;
#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
struct Document { version:u32, blocks:Vec<Block> }
#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
struct Block { kind:String, runs:Vec<Run> }
#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
struct Run { text:String, #[serde(default,skip_serializing_if="Option::is_none")] bold:Option<bool>, #[serde(default,skip_serializing_if="Option::is_none")] italic:Option<bool>, #[serde(default,skip_serializing_if="Option::is_none")] underline:Option<bool>, #[serde(default,skip_serializing_if="Option::is_none")] link:Option<String> }
pub fn validate(value:Option<serde_json::Value>,plain:&str)->Result<Option<String>,String>{
    let Some(value)=value else{return Ok(None)};
    let encoded=value.to_string();if encoded.len()>512*1024{return Err("Оформление описания превышает 512 КБ.".into());}
    let document:Document=serde_json::from_value(value).map_err(|_|"Некорректное оформление описания.".to_string())?;
    if document.version!=1 || document.blocks.len()>2048{return Err("Неподдерживаемый формат описания.".into());}
    let mut texts=Vec::new();let mut total_runs=0;
    for block in &document.blocks {
        if !["paragraph","bullet","number"].contains(&block.kind.as_str()){return Err("Неизвестный тип абзаца.".into());}
        total_runs+=block.runs.len();if total_runs>4096{return Err("Слишком много фрагментов оформления.".into());}
        let mut text=String::new();
        for run in &block.runs {
            if run.text.contains(['\r','\n']){return Err("Перенос строки должен быть отдельным абзацем.".into());}
            if let Some(link)=&run.link {
                if link.len()>2048 || link.chars().any(char::is_control){return Err("Некорректная ссылка.".into());}
                let url=reqwest::Url::parse(link).map_err(|_|"Некорректная ссылка.".to_string())?;
                if !["http","https"].contains(&url.scheme()) || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some(){return Err("Разрешены только обычные ссылки HTTP и HTTPS.".into());}
            }
            text.push_str(&run.text);
        }
        texts.push(text);
    }
    if texts.join("\n").trim()!=plain {return Err("Текст и оформление описания не совпадают; изменения не сохранены.".into());}
    serde_json::to_string(&document).map(Some).map_err(|e|e.to_string())
}
pub fn read(db:&Connection,id:&str)->Result<Option<serde_json::Value>,String>{
    let raw:Option<String>=db.query_row("SELECT description_format FROM images WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
    raw.map(|value|serde_json::from_str(&value).map_err(|_|"Повреждено оформление описания.".to_string())).transpose()
}
#[cfg(test)] mod tests {
    use super::*;use serde_json::json;
    #[test] fn typed_format_rejects_unsafe_links_mismatch_and_attributes(){
        let mut value=json!({"version":1,"blocks":[{"kind":"paragraph","runs":[{"text":"Привет 🌿","bold":true}]}]});
        assert!(validate(Some(value.clone()),"Привет 🌿").unwrap().is_some());
        assert!(validate(Some(value.clone()),"Другой текст").is_err());
        value["blocks"][0]["runs"][0]["link"]=json!("javascript:alert(1)");assert!(validate(Some(value.clone()),"Привет 🌿").is_err());
        value["blocks"][0]["runs"][0]["link"]=json!("https://example.com/path");assert!(validate(Some(value.clone()),"Привет 🌿").is_ok());
        value["blocks"][0]["runs"][0]["onclick"]=json!("bad");assert!(validate(Some(value),"Привет 🌿").is_err());
    }
}
