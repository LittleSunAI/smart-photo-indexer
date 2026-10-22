//! A single persisted orientation; v1 is normalized on reading, never partially rewritten.
use super::{Draft,EditFrame,EditHistory,Layout,Orientation,Reference,AnnotationReference,Viewport};
use serde::{Deserialize,Serialize,Deserializer,Serializer};
use serde_json::{json,Value};
#[derive(Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
struct Wire {
 version:u32,asset_id:String,orientation:Orientation,tool:String,
 operation:String,profile:String,model_id:Option<String>,prompt:String,
 form_values:serde_json::Map<String,Value>,annotations:Vec<Value>,
 selected_annotation_id:Option<String>,selected_mask_ids:Vec<String>,
 #[serde(default)]annotation_references:std::collections::BTreeMap<String,Vec<AnnotationReference>>,references:Vec<Reference>,viewport:Viewport,layout:Layout,
 #[serde(default)]edit_history:EditHistory,
}
impl<'de> Deserialize<'de> for Draft {
 fn deserialize<D:Deserializer<'de>>(deserializer:D)->Result<Self,D::Error>{
  let mut raw=Value::deserialize(deserializer)?;
  match raw["version"].as_u64(){
   Some(1)=>{
    if raw.get("orientation").is_some()||raw.get("editHistory").is_some(){return Err(serde::de::Error::custom("Смешаны версии состояния редактора"));}
    let rotation=raw["workingTurn"].as_u64().filter(|n|*n<4).ok_or_else(||serde::de::Error::custom("Неверная ориентация v1"))?;
    raw.as_object_mut().ok_or_else(||serde::de::Error::custom("Неверный черновик"))?.remove("workingTurn");
    raw["orientation"]=json!({"rotation":rotation,"mirrored":false});raw["version"]=json!(2);
    raw["layout"]["rightTab"]=json!(match raw["layout"]["rightTab"].as_str(){Some("annotations"|"models")=>"edit",Some("library")=>"sources",_=>return Err(serde::de::Error::custom("Неизвестная вкладка v1"))});
    let mut history=EditHistory::default();
    for (key,frames) in [("__annotationUndo",&mut history.undo),("__annotationRedo",&mut history.redo)]{
     if let Some(values)=raw["formValues"].as_object_mut().and_then(|v|v.remove(key)){
      for annotations in values.as_array().ok_or_else(||serde::de::Error::custom("Неверная история v1"))?{
       frames.push(EditFrame{step_id:None,asset_id:raw["assetId"].as_str().unwrap_or("").to_string(),orientation:None,annotations:serde_json::from_value(annotations.clone()).map_err(serde::de::Error::custom)?,mask_strokes:vec![],mask_annotation_id:None,selected_mask_ids:None,instruction_context:None,annotation_references:Default::default()});
      }
     }
    }
    while history.undo.len()+history.redo.len()>50||serde_json::to_vec(&history).map_err(serde::de::Error::custom)?.len()>256*1024{if !history.undo.is_empty(){history.undo.remove(0);}else if !history.redo.is_empty(){history.redo.remove(0);}else{break;}}
    raw["editHistory"]=serde_json::to_value(history).map_err(serde::de::Error::custom)?;
   },
   Some(2)=>{if raw.get("annotationReferences").is_some_and(|v|v.as_object().is_none_or(|m|!m.is_empty())){return Err(serde::de::Error::custom("Референсы меток требуют формат v3"));}},Some(3)=>{},_=>return Err(serde::de::Error::custom("Неподдерживаемая версия черновика редактора")),
  }
  let w:Wire=serde_json::from_value(raw).map_err(serde::de::Error::custom)?;
  let d=Draft{version:3,annotation_references:w.annotation_references,asset_id:w.asset_id,working_turn:w.orientation.rotation,mirrored:w.orientation.mirrored,tool:w.tool,operation:w.operation,profile:w.profile,model_id:w.model_id,prompt:w.prompt,form_values:w.form_values,annotations:w.annotations,selected_annotation_id:w.selected_annotation_id,selected_mask_ids:w.selected_mask_ids,references:w.references,viewport:w.viewport,layout:w.layout,edit_history:w.edit_history};
  d.validate().map_err(serde::de::Error::custom)?;Ok(d)
 }
}
impl Serialize for Draft {
 fn serialize<S:Serializer>(&self,serializer:S)->Result<S::Ok,S::Error>{
  Wire{version:3,annotation_references:self.annotation_references.clone(),asset_id:self.asset_id.clone(),orientation:Orientation{rotation:self.working_turn,mirrored:self.mirrored},tool:self.tool.clone(),operation:self.operation.clone(),profile:self.profile.clone(),model_id:self.model_id.clone(),prompt:self.prompt.clone(),form_values:self.form_values.clone(),annotations:self.annotations.clone(),selected_annotation_id:self.selected_annotation_id.clone(),selected_mask_ids:self.selected_mask_ids.clone(),references:self.references.clone(),viewport:self.viewport.clone(),layout:self.layout.clone(),edit_history:self.edit_history.clone()}.serialize(serializer)
 }
}
