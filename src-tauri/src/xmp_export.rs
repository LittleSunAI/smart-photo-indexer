use crate::*;
use serde_json::{json,Value};
use std::io::Write;
use std::collections::HashSet;
fn xml(value:&str)->Result<String,String>{
 if value.chars().any(|c|(c<'\u{20}' && !matches!(c,'\t'|'\n'|'\r')) || matches!(c,'\u{fffe}'|'\u{ffff}')){return Err("Метаданные содержат недопустимый для XML управляющий символ.".into());}
 Ok(value.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&apos;"))
}
#[tauri::command]
pub async fn export_image_xmp(app:tauri::AppHandle,paths:Vec<String>)->Result<Value,String>{
 if paths.is_empty() || paths.len()>256{return Err("Выберите от 1 до 256 физических копий.".into());}
 tauri::async_runtime::spawn_blocking(move||{
  let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let mut seen=HashSet::new();let mut results=Vec::new();
  for path in paths{if !seen.insert(path.to_lowercase()){continue;}
   let result=export_one(&mut db,&path);results.push(match result{Ok(target)=>json!({"path":path,"targetPath":target,"saved":true}),Err(error)=>json!({"path":path,"saved":false,"error":error})});
  }
  Ok(json!({"saved":results.iter().filter(|r|r["saved"]==true).count(),"failed":results.iter().filter(|r|r["saved"]!=true).count(),"items":results}))
 }).await.map_err(|e|e.to_string())?
}
fn export_one(db:&mut Connection,path:&str)->Result<String,String>{
 let source=fs::canonicalize(path).map_err(|e|format!("Фотография недоступна: {e}"))?;let path=path_to_string(&source);
 let tx=db.transaction().map_err(|e|e.to_string())?;
 let (id,hash,description,origin,model):(String,String,Option<String>,String,Option<String>)=tx.query_row("SELECT i.id,i.sha256,i.description,i.description_origin,i.ai_model FROM images i JOIN image_paths p ON p.image_id=i.id WHERE p.path=?1 AND p.is_current=1",[&path],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|"Сначала добавьте фотографию в индекс.".to_string())?;
 let tags=read_tags_for_image(&tx,&id)?;
 let origins={let mut stmt=tx.prepare("SELECT t.name,it.source FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=?1 ORDER BY t.name").map_err(|e|e.to_string())?;
  let rows=stmt.query_map([&id],|r|Ok(json!({"tag":r.get::<_,String>(0)?,"source":r.get::<_,String>(1)?}))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 tx.commit().map_err(|e|e.to_string())?;
 if compute_file_sha256(&source)?!=hash{return Err("Содержимое фотографии изменилось. Обновите индекс перед экспортом.".into());}
 let description=xml(description.as_deref().unwrap_or(""))?;
 let tags=tags.iter().map(|tag|xml(tag).map(|escaped|format!("<rdf:li>{escaped}</rdf:li>"))).collect::<Result<Vec<_>,_>>()?.join("");
 let provenance=xml(&json!({"imageId":id,"sha256":hash,"descriptionSource":origin,"model":model,"tags":origins}).to_string())?;
 let output=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:spi=\"urn:smart-photo-indexer:metadata:1.0\"><xmp:CreatorTool>Smart Photo Indexer</xmp:CreatorTool><dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{description}</rdf:li></rdf:Alt></dc:description><dc:subject><rdf:Bag>{tags}</rdf:Bag></dc:subject><spi:Provenance>{provenance}</spi:Provenance></rdf:Description></rdf:RDF></x:xmpmeta>\n");
 if output.len()>1_000_000{return Err("Метаданные превышают ограничение XMP-экспорта.".into());}
 let target=source.with_file_name(format!("{}.xmp",file_name(&source)));
 file_safety::atomic_save(&target,|file,_|file.write_all(output.as_bytes()).map_err(|e|e.to_string()))?;
 Ok(path_to_string(&target))
}
