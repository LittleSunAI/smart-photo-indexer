use crate::*;
use serde_json::{json,Value};
use std::collections::{BTreeMap,BTreeSet,HashSet};
use std::io::Read;
const DC:&str="http://purl.org/dc/elements/1.1/";
const RDF:&str="http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const SPI:&str="urn:smart-photo-indexer:metadata:1.0";
const XMP:&str="http://ns.adobe.com/xap/1.0/";
struct Metadata{sha:String,description:String,tags:Vec<String>}
fn text(node:roxmltree::Node<'_,'_>)->Result<String,String>{if node.children().any(|n|n.is_element()){return Err("В текстовом поле XMP обнаружена неподдерживаемая разметка.".into());}Ok(node.children().filter(|n|n.is_text()).filter_map(|n|n.text()).collect())}
fn child<'a,'input>(node:roxmltree::Node<'a,'input>,namespace:&str,name:&str)->Result<roxmltree::Node<'a,'input>,String>{let mut nodes=node.children().filter(|n|n.has_tag_name((namespace,name)));let node=nodes.next().ok_or_else(||format!("В XMP отсутствует поле {name}."))?;if nodes.next().is_some(){return Err(format!("Поле {name} в XMP повторяется."));}Ok(node)}
fn parse(bytes:&[u8])->Result<Metadata,String>{
 if bytes.len()>1_000_000{return Err("XMP превышает 1 МБ.".into());}
 let input=std::str::from_utf8(bytes).map_err(|_|"XMP должен быть в UTF-8.")?;
 let doc=roxmltree::Document::parse_with_options(input,roxmltree::ParsingOptions{allow_dtd:false,nodes_limit:50_000,..Default::default()}).map_err(|e|format!("Не удалось прочитать XML: {e}"))?;
 let root=doc.root_element();if !root.has_tag_name(("adobe:ns:meta/","xmpmeta")){return Err("Поддерживается XMP, созданный Smart Photo Indexer.".into());}
 let description=child(child(root,RDF,"RDF")?,RDF,"Description")?;
 if text(child(description,XMP,"CreatorTool")?)? != "Smart Photo Indexer"{return Err("Поддерживается XMP, созданный Smart Photo Indexer.".into());}
 let provenance:Value=serde_json::from_str(&text(child(description,SPI,"Provenance")?)?).map_err(|_|"Не удалось прочитать сведения о содержимом XMP.")?;
 let sha=provenance["sha256"].as_str().filter(|s|s.len()==64&&s.bytes().all(|b|b.is_ascii_hexdigit())).ok_or("В XMP отсутствует корректный SHA256 фотографии.")?.to_lowercase();
 let alt=child(child(description,DC,"description")?,RDF,"Alt")?;
 let mut entries=alt.children().filter(|n|n.has_tag_name((RDF,"li"))&&n.attribute(("http://www.w3.org/XML/1998/namespace","lang"))==Some("x-default"));
 let value=text(entries.next().ok_or("В XMP отсутствует основное описание.")?)?.trim().to_owned();if entries.next().is_some(){return Err("В XMP несколько основных описаний.".into());}
 if value.len()>256*1024{return Err("Описание в XMP превышает поддерживаемый размер.".into());}
 let bag=child(child(description,DC,"subject")?,RDF,"Bag")?;let mut tags=Vec::new();for node in bag.children().filter(|n|n.is_element()){if !node.has_tag_name((RDF,"li")){return Err("Список тегов XMP имеет неподдерживаемую структуру.".into());}tags.push(text(node)?);}
 Ok(Metadata{sha,description:value,tags:normalize_metadata_tags(tags)})
}
struct Group{paths:Vec<String>,descriptions:BTreeSet<String>,tags:BTreeSet<String>}
fn read_candidate(db:&Connection,path:&str)->Result<(String,String,Metadata),String>{
 let source=fs::canonicalize(path).map_err(|e|format!("Фотография недоступна: {e}"))?;let path=path_to_string(&source);
 let(id,indexed):(String,String)=db.query_row("SELECT i.id,i.sha256 FROM images i JOIN image_paths p ON p.image_id=i.id WHERE p.path=?1 AND p.is_current=1",[&path],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"Сначала добавьте фотографию в индекс.")?;
 let sidecar=source.with_file_name(format!("{}.xmp",file_name(&source)));let mut bytes=Vec::new();fs::File::open(sidecar).map_err(|e|format!("Соседний XMP недоступен: {e}"))?.take(1_000_001).read_to_end(&mut bytes).map_err(|e|e.to_string())?;let metadata=parse(&bytes)?;
 let actual=compute_file_sha256(&source)?;if actual!=indexed||actual!=metadata.sha{return Err("Содержимое фотографии не совпадает с индексом или XMP. Метаданные не перенесены.".into());}Ok((id,path,metadata))
}
struct Merge{description:String,tags:Vec<String>,fields:Vec<String>,conflict:bool,description_version:i64,tags_version:i64}
fn merge(db:&Connection,id:&str,group:&Group)->Result<Merge,String>{
 let(description,description_version,tags_version):(Option<String>,i64,i64)=db.query_row("SELECT description,description_version,tags_version FROM images WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
 let current=description.unwrap_or_default();let old_tags=read_tags_for_image_from_connection(db,id)?;let mut tags:BTreeSet<String>=old_tags.iter().cloned().collect();tags.extend(group.tags.iter().cloned());
 let conflict=group.descriptions.len()>1||(!current.trim().is_empty()&&group.descriptions.iter().any(|s|s!=&current));
 let description=if current.trim().is_empty()&&!conflict{group.descriptions.iter().next().cloned().unwrap_or(current.clone())}else{current.clone()};
 let mut fields=Vec::new();if description!=current{fields.push("description".into());}if tags!=old_tags.into_iter().collect(){fields.push("tags".into());}
 Ok(Merge{description,tags:tags.into_iter().collect(),fields,conflict,description_version,tags_version})
}
#[tauri::command]
pub async fn import_image_xmp(app:tauri::AppHandle,paths:Vec<String>)->Result<Value,String>{
 if paths.is_empty()||paths.len()>256{return Err("Выберите от 1 до 256 физических копий.".into());}
 tauri::async_runtime::spawn_blocking(move||{
  let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let mut seen=HashSet::new();let mut groups:BTreeMap<String,Group>=BTreeMap::new();let mut items=Vec::new();let(mut failed,mut conflicts,mut updated,mut unchanged)=(0,0,0,0);let mut updates=Vec::new();
  for path in paths{if !seen.insert(path.replace('/',"\\").to_lowercase()){continue;}match read_candidate(&db,&path){Ok((id,path,metadata))=>{let group=groups.entry(id).or_insert_with(||Group{paths:Vec::new(),descriptions:BTreeSet::new(),tags:BTreeSet::new()});group.paths.push(path);if !metadata.description.is_empty(){group.descriptions.insert(metadata.description);}group.tags.extend(metadata.tags);},Err(error)=>{failed+=1;items.push(json!({"paths":[path],"error":error}));}}}
  for(id,group)in groups{
   let result=(||{
    let merged=merge(&db,&id,&group)?;let conflict=merged.conflict;
    if merged.fields.is_empty(){return Ok((None,conflict));}
    let input=IndexMetadataUpdateInputDto{image_id:Some(id.clone()),path:group.paths[0].clone(),description:merged.description,description_format:None,tags:merged.tags,fields:Some(merged.fields),source:Some("manual".into()),model:None,expected_description_version:Some(merged.description_version),expected_tags_version:Some(merged.tags_version)};
    Ok::<_,String>((Some(update_image_metadata_internal(app.clone(),input)?),conflict))
   })();
   match result{Ok((update,conflict))=>{if conflict{conflicts+=1;}let changed=update.is_some();if let Some(update)=update{updated+=1;updates.push(update);}else{unchanged+=1;}items.push(json!({"imageId":id,"paths":group.paths,"updated":changed,"descriptionConflict":conflict}));},Err(error)=>{failed+=group.paths.len();items.push(json!({"imageId":id,"paths":group.paths,"error":error}));}}
  }
  Ok(json!({"updatedImages":updated,"unchangedImages":unchanged,"conflicts":conflicts,"failedFiles":failed,"items":items,"updates":updates}))
 }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]mod tests{
 use super::*;
 fn document(description:&str,tags:&str,sha:&str)->String{format!("<x:xmpmeta xmlns:x='adobe:ns:meta/'><r:RDF xmlns:r='{RDF}'><r:Description xmlns:d='{DC}' xmlns:p='{SPI}' xmlns:m='{XMP}'><m:CreatorTool>Smart Photo Indexer</m:CreatorTool><d:description><r:Alt><r:li xml:lang='x-default'>{description}</r:li></r:Alt></d:description><d:subject><r:Bag>{tags}</r:Bag></d:subject><p:Provenance>{{\"imageId\":\"foreign-id\",\"sha256\":\"{sha}\"}}</p:Provenance></r:Description></r:RDF></x:xmpmeta>")}
 #[test]fn namespaces_unicode_entities_and_cdata_are_decoded_without_trusting_prefix_or_old_id(){
  let doc=document("Кошка &amp; &lt;окно&gt; <![CDATA[& свет]]>","<r:li>ТЕГ &amp; слово</r:li><r:li>тег &amp; слово</r:li>",&"A".repeat(64));let parsed=parse(doc.as_bytes()).unwrap();assert_eq!(parsed.description,"Кошка & <окно> & свет");assert_eq!(parsed.tags,vec!["тег & слово"]);assert_eq!(parsed.sha,"a".repeat(64));
 }
 #[test]fn malformed_unsupported_oversize_and_all_dtd_forms_are_rejected(){
  let doc=document("plain","",&"a".repeat(64));
  for bad in ["<broken>".into(),doc.replace(SPI,"urn:other"),doc.replace("Smart Photo Indexer","Other"),format!("<!DOCTYPE x:xmpmeta>{doc}"),format!("<!DOCTYPE x:xmpmeta [<!ENTITY file SYSTEM 'file:///not-read'>]>{doc}")," ".repeat(1_000_001)]{assert!(parse(bad.as_bytes()).is_err());}
 }
 #[test]fn merging_aliases_conflicts_before_writing_and_preserves_local_text_and_tag_union(){
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','i','i.png',1,1)",[]).unwrap();
  let group=Group{paths:vec![],descriptions:["first".into(),"second".into()].into_iter().collect(),tags:["new".into()].into_iter().collect()};let result=merge(&db,"i",&group).unwrap();assert!(result.conflict);assert_eq!(result.description,"");assert_eq!(result.fields,vec!["tags"]);
  db.execute("UPDATE images SET description='local',description_version=7,tags_version=9 WHERE id='i'",[]).unwrap();let group=Group{paths:vec![],descriptions:["foreign".into()].into_iter().collect(),tags:BTreeSet::new()};let result=merge(&db,"i",&group).unwrap();assert!(result.conflict);assert_eq!(result.description,"local");assert!(result.fields.is_empty());assert_eq!((result.description_version,result.tags_version),(7,9));
 }
 #[test]fn candidate_uses_current_path_sha_and_id_and_refuses_changed_bytes(){
  let root=crate::test_support::temp_dir("smart-photo-xmp-import");let path=root.join("photo.png");fs::write(&path,b"own source").unwrap();let sha=compute_file_sha256(&path).unwrap();fs::write(root.join("photo.png.xmp"),document("description","",&sha)).unwrap();let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('current-id','current','photo.png',?1,1,1)",[&sha]).unwrap();db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('current-id',?1,1,1,1)",[path_to_string(&fs::canonicalize(&path).unwrap())]).unwrap();assert_eq!(read_candidate(&db,&path_to_string(&path)).unwrap().0,"current-id");fs::write(&path,b"changed").unwrap();assert!(read_candidate(&db,&path_to_string(&path)).is_err());fs::remove_file(path).unwrap();fs::remove_file(root.join("photo.png.xmp")).unwrap();fs::remove_dir(root).unwrap();
 }
}
