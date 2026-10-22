//! Persisted operation instructions are separate from model parameters and canvas selection.
use super::{contracts::Draft,geometry};
use serde::{Deserialize,Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Operation {pub mode:String,pub target:String,pub texts:BTreeMap<String,String>,pub enabled:Vec<String>,pub global_text:String,pub reviewed:bool}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
pub struct Projection {pub prompt:String,pub annotations:BTreeMap<String,String>}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
pub struct Context {pub version:u32,pub operations:BTreeMap<String,Operation>,pub projection:Projection}
pub fn local(operation:&str)->bool{["universal-edit","replace-object","relight","recolor"].contains(&operation)}
pub fn validate(raw:&Value)->Result<(),String>{
 let c:Context=serde_json::from_value(raw.clone()).map_err(|_|"Повреждены сохранённые инструкции операций")?;
 if c.version!=1||c.operations.len()>11||c.projection.prompt.len()>32768||c.projection.annotations.len()>5000{return Err("Неподдерживаемая версия или размер инструкций".into());}
 for(op,v)in &c.operations{if !super::contracts::OPERATIONS.contains(&op.as_str())||!["single","multi"].contains(&v.mode.as_str())||v.target.is_empty()||v.target.len()>160||v.texts.len()>5001||v.enabled.len()>5001||v.global_text.len()>32768||v.texts.iter().any(|(id,text)|id.is_empty()||id.len()>160||text.len()>32768){return Err("Неверная цель или слишком длинная инструкция".into());}}
 if c.projection.annotations.iter().any(|(id,text)|id.len()>160||text.len()>32768){return Err("Неверная проекция инструкций".into());}Ok(())
}
pub fn tasks(d:&Draft)->Result<(String,Vec<(String,String)>),String>{
 if let Some(raw)=d.form_values.get("__instructionContext"){
  validate(raw)?;let c:Context=serde_json::from_value(raw.clone()).map_err(|_|"Повреждены инструкции")?;
  let Some(op)=c.operations.get(&d.operation)else{return Ok((String::new(),vec![]));};
  if c.projection.prompt!=d.prompt||d.annotations.iter().any(|a|c.projection.annotations.get(a["id"].as_str().unwrap_or("")).is_some_and(|text|Some(text.as_str())!=a["instruction"].as_str())){return Err("Сохранённые варианты инструкции расходятся. Проверьте тексты перед отправкой.".into());}
  let mut out=vec![];let targets=if local(&d.operation){d.annotations.iter().filter(|a|a["included"]==true&&["point","rect"].contains(&a["kind"].as_str().unwrap_or(""))).filter_map(|a|a["id"].as_str().map(String::from)).collect()}else{vec!["image".into()]};let mut seen=std::collections::HashSet::new();
  for target in targets {if !seen.insert(target.clone()){continue;}if target!="image"&&!local(&d.operation){continue;}
   let text=op.texts.get(&target).cloned().unwrap_or_default();
   if text.trim().is_empty(){continue;}
   if target!="image"&&!d.annotations.iter().any(|a|a["id"].as_str()==Some(&target)&&a["included"]==true){return Err("Место задания удалено или исключено. Выберите цель правки.".into());}
   out.push((target,text));
  }
  let global=if local(&d.operation){let mut values=vec![];for text in [op.global_text.as_str(),op.texts.get("image").map(String::as_str).unwrap_or("")]{if !text.trim().is_empty()&&!values.contains(&text){values.push(text);}}values.join("\n")}else{String::new()};
  return Ok((global,out));
 }
 // Legacy requests retain their texts, but unrelated operations never inherit local edits.
 let mut text=d.prompt.clone();let mut out=vec![];
 if local(&d.operation){
  for a in d.annotations.iter().filter(|a|a["included"]==true){let instruction=a["instruction"].as_str().unwrap_or("");if !instruction.trim().is_empty(){out.push((a["id"].as_str().unwrap_or("").to_string(),instruction.into()));}}
  if out.is_empty()&&!text.trim().is_empty(){if let Some(id)=&d.selected_annotation_id{if d.annotations.iter().any(|a|a["id"].as_str()==Some(id)&&a["included"]==true&&["point","rect"].contains(&a["kind"].as_str().unwrap_or(""))){out.push((id.clone(),std::mem::take(&mut text)));}}}
 }Ok((text,out))
}
pub fn plain(d:&Draft)->Result<String,String>{let(global,tasks)=tasks(d)?;Ok(std::iter::once(global).chain(tasks.into_iter().map(|(_,text)|text)).filter(|s|!s.trim().is_empty()).collect::<Vec<_>>().join("\n"))}
pub fn region(d:&Draft,id:&str)->String{
 let Some(a)=d.annotations.iter().find(|a|a["id"].as_str()==Some(id))else{return "весь снимок".into();};
 match a["kind"].as_str(){Some("point")=>{let(x,y)=geometry::point(&a["point"]).unwrap_or((0.,0.));let(x,y)=geometry::orient_point(x,y,d.working_turn,d.mirrored);format!("точка ({:.1}%, {:.1}%)",x*100.,y*100.)},Some("rect")=>{let moved=geometry::transform_annotations(&[a.clone()],&d.asset_id,|x,y|geometry::orient_point(x,y,d.working_turn,d.mirrored));format!("область от ({:.1}%, {:.1}%) до ({:.1}%, {:.1}%)",moved[0]["start"]["x"].as_f64().unwrap_or(0.)*100.,moved[0]["start"]["y"].as_f64().unwrap_or(0.)*100.,moved[0]["end"]["x"].as_f64().unwrap_or(0.)*100.,moved[0]["end"]["y"].as_f64().unwrap_or(0.)*100.)},_=>"отмеченная область (текстовый ориентир, не точная маска)".into()}
}
#[cfg(test)]mod tests{
 use super::*;use serde_json::json;
 fn point_draft()->Draft{let mut d=Draft::initial("a");d.operation="replace-object".into();d.prompt="Добавить мяч".into();d.selected_annotation_id=Some("p".into());d.annotations=vec![json!({"id":"p","assetId":"a","kind":"point","instruction":"","visible":true,"included":true,"point":{"x":0.25,"y":0.6}})];d}
 #[test]fn legacy_single_field_keeps_point_target(){let d=point_draft();let(g,t)=tasks(&d).unwrap();assert!(g.is_empty());assert_eq!(t,vec![("p".into(),"Добавить мяч".into())]);assert_eq!(region(&d,"p"),"точка (25.0%, 60.0%)");}
 #[test]fn background_never_inherits_local_object_instruction(){let mut d=point_draft();d.operation="replace-background".into();d.prompt="Лес".into();d.annotations[0]["instruction"]=json!("Добавить мяч");let(g,t)=tasks(&d).unwrap();assert_eq!(g,"Лес");assert!(t.is_empty());}
 #[test]fn legacy_mode_and_target_do_not_hide_other_included_tasks(){
  let mut d=point_draft();d.annotations[0]["instruction"]=json!("Человек");
  d.annotations.push(json!({"id":"r","kind":"rect","included":true,"visible":false,"instruction":"Лошадь","start":{"x":0.6,"y":0.2},"end":{"x":0.8,"y":0.4}}));
  d.prompt=String::new();
  d.form_values.insert("__instructionContext".into(),json!({"version":1,"operations":{"replace-object":{"mode":"single","target":"missing","texts":{"p":"Человек","r":"Лошадь","missing":"Удалено"},"enabled":[],"globalText":"","reviewed":false}},"projection":{"prompt":"","annotations":{"p":"Человек","r":"Лошадь"}}}));
  let(_,t)=tasks(&d).unwrap();assert_eq!(t,vec![("p".into(),"Человек".into()),("r".into(),"Лошадь".into())]);
  d.annotations[0]["included"]=json!(false);assert_eq!(tasks(&d).unwrap().1,vec![("r".into(),"Лошадь".into())]);
 }
}
