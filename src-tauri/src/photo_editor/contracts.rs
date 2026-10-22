use serde::{Deserialize,Serialize};
use serde_json::{json,Value};

#[derive(Clone,Debug)]
pub struct Draft {
    pub version:u32,pub asset_id:String,pub working_turn:u8,pub tool:String,
    pub operation:String,pub profile:String,pub model_id:Option<String>,pub prompt:String,
    pub form_values:serde_json::Map<String,Value>,pub annotations:Vec<Value>,
    pub selected_annotation_id:Option<String>,pub selected_mask_ids:Vec<String>,
    pub references:Vec<Reference>,pub annotation_references:std::collections::BTreeMap<String,Vec<AnnotationReference>>,pub viewport:Viewport,pub layout:Layout,
    pub mirrored:bool,pub edit_history:EditHistory,
}
#[path="draft_wire.rs"]mod wire;
#[derive(Clone,Debug,Default,Deserialize,Serialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Orientation {pub rotation:u8,pub mirrored:bool}
#[derive(Clone,Debug,Default,Deserialize,Serialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct EditHistory {pub undo:Vec<EditFrame>,pub redo:Vec<EditFrame>}
#[derive(Clone,Debug,Deserialize,Serialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct EditFrame {pub step_id:Option<String>,pub asset_id:String,pub orientation:Option<Orientation>,pub annotations:Vec<Value>,pub mask_strokes:Vec<Value>,pub mask_annotation_id:Option<String>,#[serde(default,skip_serializing_if="Option::is_none")]pub selected_mask_ids:Option<Vec<String>>,#[serde(default,skip_serializing_if="Option::is_none")]pub instruction_context:Option<Value>,#[serde(default)]pub annotation_references:std::collections::BTreeMap<String,Vec<AnnotationReference>>}
#[derive(Clone,Debug,Default,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct AnnotationReference {pub asset_id:String,pub included:bool,#[serde(default,skip_serializing_if="Option::is_none")]pub display_name:Option<String>,#[serde(default,skip_serializing_if="Option::is_none")]pub role_description:Option<String>}
#[derive(Clone,Debug,Deserialize,Serialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Reference {pub asset_id:String,pub role:String}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Viewport {pub zoom:f64,pub pan_x:f64,pub pan_y:f64}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Layout {pub left_width:f64,pub right_width:f64,pub history_height:f64,pub right_tab:String}
pub const OPERATIONS:[&str;11]=["universal-edit","replace-object","remove-object","remove-background","replace-background","outpaint","upscale","relight","recolor","compose","restore"];
impl Draft {
    pub fn new_frame(&self,asset_id:&str)->Self{
        let mut next=Self::initial(asset_id);
        next.operation=self.operation.clone();next.profile=self.profile.clone();next.model_id=self.model_id.clone();next.layout=self.layout.clone();
        let settings=|key:&str|!key.starts_with("__")&&![' ', '\0'].iter().any(|c|key.contains(*c))&&!["prompt","system_prompt","negative_prompt","image_url","image_urls","mask_url","mask_image_url","video_url","audio_url","pdf_url"].contains(&key)||["__choiceId","__schemaSHA","__parameterModes","__outputSizeMode","__lastChoices","__budgetUsd","__overallBudgetUsd","__budgetSince","__concurrency","__preserveColor","__pricingCredential","__pricingRevision"].contains(&key);
        next.form_values=self.form_values.iter().filter(|(key,_)|settings(key)).map(|(key,value)|(key.clone(),value.clone())).collect();
        if let Some(contexts)=self.form_values.get("__contexts").and_then(Value::as_object){
            let cleaned=contexts.iter().map(|(key,context)|{let values=context["formValues"].as_object().map(|values|values.iter().filter(|(name,_)|settings(name)).map(|(name,value)|(name.clone(),value.clone())).collect::<serde_json::Map<String,Value>>()).unwrap_or_default();(key.clone(),json!({"formValues":values}))}).collect::<serde_json::Map<String,Value>>();
            next.form_values.insert("__contexts".into(),json!(cleaned));
        }
        next
    }
    pub fn initial(asset_id:&str)->Self{
        serde_json::from_value(json!({"version":1,"assetId":asset_id,"workingTurn":0,"tool":"select","operation":"universal-edit","profile":"economy","modelId":null,"prompt":"","formValues":{},"annotations":[],"selectedAnnotationId":null,"selectedMaskIds":[],"references":[],"viewport":{"zoom":1.0,"panX":0.0,"panY":0.0},"layout":{"leftWidth":184.0,"rightWidth":316.0,"historyHeight":116.0,"rightTab":"annotations"}})).expect("Static editor draft contract")
    }
    pub fn push_annotation_undo(&mut self){if self.form_values.get("__maskStrokes").and_then(Value::as_array).is_some_and(|s|!s.is_empty())&&!self.edit_history.undo.is_empty(){return;}self.edit_history.undo.push(EditFrame{step_id:None,asset_id:self.asset_id.clone(),orientation:Some(Orientation{rotation:self.working_turn,mirrored:self.mirrored}),annotations:self.annotations.clone(),mask_strokes:vec![],mask_annotation_id:None,selected_mask_ids:Some(self.selected_mask_ids.clone()),instruction_context:self.form_values.get("__instructionContext").cloned(),annotation_references:self.annotation_references.clone()});self.edit_history.redo.clear();while self.edit_history.undo.len()>50||serde_json::to_vec(&self.edit_history).map(|v|v.len()>256*1024).unwrap_or(true){self.edit_history.undo.remove(0);}}

    pub fn validate(&self)->Result<(),String>{
        super::geometry::validate_annotations(self)?;super::references::validate(&self.annotations,&self.annotation_references)?;
        if let Some(raw)=self.form_values.get("__instructionContext"){super::instructions::validate(raw)?;}
        if self.version!=3||self.working_turn>3||!OPERATIONS.contains(&self.operation.as_str())||!["select","pan","point","rect","mask","eraser"].contains(&self.tool.as_str())||!["economy","balanced","quality"].contains(&self.profile.as_str()){
            return Err("Неподдерживаемая версия или параметры черновика редактора.".into());
        }
        for name in ["__annotationUndo","__annotationRedo"]{if let Some(value)=self.form_values.get(name){let frames=value.as_array().ok_or("История разметки повреждена")?;if frames.len()>50||value.to_string().len()>256*1024{return Err("История разметки превышает предел.".into());}for frame in frames{let mut copy=self.clone();copy.annotations=serde_json::from_value(frame.clone()).map_err(|_|"История разметки повреждена")?;copy.selected_annotation_id=None;copy.selected_mask_ids.clear();super::geometry::validate_annotations(&copy)?;}}}
        let history=&self.edit_history;
        if history.undo.len()+history.redo.len()>50||serde_json::to_vec(history).map_err(|e|e.to_string())?.len()>256*1024{return Err("История правок превышает допустимые пределы".into());}
        for frame in history.undo.iter().chain(&history.redo){if frame.asset_id!=self.asset_id||frame.orientation.as_ref().is_some_and(|o|o.rotation>3)||frame.step_id.as_ref().is_some_and(|id|id.is_empty()||id.len()>1024){return Err("История правок имеет другую координатную базу".into());}let mut copy=self.clone();copy.annotations=frame.annotations.clone();copy.selected_annotation_id=None;copy.selected_mask_ids.clear();super::geometry::validate_annotations(&copy)?;validate_strokes(&frame.mask_strokes)?;super::references::validate(&frame.annotations,&frame.annotation_references)?;if let Some(raw)=&frame.instruction_context{super::instructions::validate(raw)?;}if let Some(ids)=&frame.selected_mask_ids{copy.selected_mask_ids=ids.clone();super::geometry::validate_annotations(&copy)?;}}
        if let Some(strokes)=self.form_values.get("__maskStrokes"){validate_strokes(strokes.as_array().ok_or("Неверный буфер штрихов")?)?;}
        let v=&self.viewport;let l=&self.layout;
        if !v.zoom.is_finite()||!(0.05..=16.0).contains(&v.zoom)||!v.pan_x.is_finite()||!v.pan_y.is_finite()||v.pan_x.abs()>1e6||v.pan_y.abs()>1e6||!l.left_width.is_finite()||!(140.0..=400.0).contains(&l.left_width)||!l.right_width.is_finite()||!(240.0..=600.0).contains(&l.right_width)||!l.history_height.is_finite()||!(64.0..=400.0).contains(&l.history_height)||!["edit","tasks","sources"].contains(&l.right_tab.as_str()){
            return Err("Неверная геометрия окна редактора.".into());
        }
        if self.asset_id.is_empty()||self.asset_id.len()>160||self.prompt.len()>32768||self.annotations.len()>5000||self.references.len()>16||self.selected_mask_ids.len()>5000||self.references.iter().any(|r|r.asset_id.len()>160||r.role.len()>160){return Err("Черновик превышает допустимые пределы.".into());}
        if serde_json::to_vec(self).map_err(|e|e.to_string())?.len()>1024*1024{return Err("Черновик превышает предел 1 МиБ. Сохранённая версия не изменена.".into());}
        Ok(())
    }
}

fn validate_strokes(values:&[Value])->Result<(),String>{
 if values.len()>512{return Err("Слишком много штрихов маски".into());}let mut total=0;
 for value in values{let s:super::masks::Stroke=serde_json::from_value(value.clone()).map_err(|_|"Неверный штрих маски")?;total+=s.points.len();if !s.radius.is_finite()||!(0.5..=512.).contains(&s.radius)||s.points.is_empty()||total>20000{return Err("Штрихи маски превышают допустимые пределы".into());}for p in s.points{super::geometry::point(&json!({"x":p.x,"y":p.y}))?;}}
 Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn new_frames_keep_model_settings_and_clear_every_edit_owner(){
        let mut d=Draft::initial("old");d.operation="replace-object".into();d.model_id=Some("model".into());d.prompt="old edit".into();d.working_turn=2;d.mirrored=true;d.tool="rect".into();d.layout.history_height=160.0;d.viewport.zoom=2.0;
        d.form_values=serde_json::from_value(json!({"image_size":"auto_2K","quality":"high","prompt":"old","system_prompt":"old","negative_prompt":"old","image_urls":["old"],"__parameterModes":{"quality":"explicit"},"__outputSizeMode":"model","__instructionContext":{"old":true},"__maskStrokes":[[1]],"__consentScope":"old","__contexts":{"context":{"prompt":"old","formValues":{"quality":"low","prompt":"old","system_prompt":"old","negative_prompt":"old","__instructionContext":{"old":true}}}}})).unwrap();
        let next=d.new_frame("new");assert_eq!(next.asset_id,"new");assert_eq!(next.operation,d.operation);assert_eq!(next.model_id,d.model_id);assert_eq!(next.layout.history_height,160.0);assert_eq!(next.viewport.zoom,1.0);assert_eq!(next.tool,"select");assert_eq!(next.working_turn,0);assert!(!next.mirrored);assert!(next.prompt.is_empty());assert!(next.annotations.is_empty());assert!(next.annotation_references.is_empty());assert!(next.edit_history.undo.is_empty());assert_eq!(next.form_values["image_size"],json!("auto_2K"));assert_eq!(next.form_values["quality"],json!("high"));
        for name in ["prompt","system_prompt","negative_prompt","image_urls","__instructionContext","__maskStrokes","__consentScope"]{assert!(!next.form_values.contains_key(name));}
        assert_eq!(next.form_values["__contexts"]["context"],json!({"formValues":{"quality":"low"}}));
    }
    #[test]fn legacy_drafts_migrate_without_inventing_history_orientation(){
        let mut v=serde_json::to_value(Draft::initial("asset")).unwrap();v["version"]=json!(1);v.as_object_mut().unwrap().remove("orientation");v.as_object_mut().unwrap().remove("editHistory");v["workingTurn"]=json!(3);v["layout"]["rightTab"]=json!("models");v["formValues"]["__annotationUndo"]=json!([[]]);
        let d:Draft=serde_json::from_value(v).unwrap();assert_eq!(d.version,3);assert_eq!(d.working_turn,3);assert!(!d.mirrored);assert_eq!(d.layout.right_tab,"edit");assert!(d.edit_history.undo[0].orientation.is_none());assert!(!d.form_values.contains_key("__annotationUndo"));let raw=serde_json::to_value(d).unwrap();assert!(raw.get("workingTurn").is_none());assert_eq!(raw["orientation"],json!({"rotation":3,"mirrored":false}));
    }
    #[test]fn v2_requires_canonical_orientation_and_rejects_future_versions(){let d=Draft::initial("asset");let raw=serde_json::to_value(&d).unwrap();let mut bad=raw.clone();bad["version"]=json!(4);assert!(serde_json::from_value::<Draft>(bad).is_err());let mut bad=raw.clone();bad["workingTurn"]=json!(0);assert!(serde_json::from_value::<Draft>(bad).is_err());let mut bad=raw;bad["orientation"]["rotation"]=json!(4);assert!(serde_json::from_value::<Draft>(bad).is_err());}
    #[test]fn unknown_fields_and_invalid_geometry_are_rejected(){
        let d=Draft::initial("asset");d.validate().unwrap();
        let mut raw=serde_json::to_value(&d).unwrap();raw["providerKey"]=json!("must never be accepted");assert!(serde_json::from_value::<Draft>(raw).is_err());
        let mut bad=d.clone();bad.working_turn=4;assert!(bad.validate().is_err());bad=d.clone();bad.viewport.zoom=f64::INFINITY;assert!(bad.validate().is_err());bad=d;bad.prompt="x".repeat(32769);assert!(bad.validate().is_err());
    }
}
