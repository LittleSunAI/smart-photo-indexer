use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommandError {pub code:String,pub technical:String}
impl CommandError {
 pub fn new(code:&str,technical:impl Into<String>)->Self{Self{code:code.into(),technical:technical.into()}}
}
impl From<String> for CommandError {fn from(value:String)->Self{Self::new("native.operation-failed",value)}}
impl From<&str> for CommandError {fn from(value:&str)->Self{Self::new("native.operation-failed",value)}}
pub fn processing_target_error(error:rusqlite::Error)->CommandError{
 match error {rusqlite::Error::QueryReturnedNoRows=>CommandError::new("queue.target-missing","Processing target no longer exists"),other=>CommandError::from(other.to_string())}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn missing_target_is_not_a_database_failure(){assert_eq!(processing_target_error(rusqlite::Error::QueryReturnedNoRows).code,"queue.target-missing");let sql=processing_target_error(rusqlite::Error::InvalidQuery);assert_eq!(sql.code,"native.operation-failed");assert!(!sql.technical.is_empty());}
 #[test]fn dialog_locale_has_russian_compatible_default(){let caption=crate::ui_locale_generated::caption;assert_eq!(caption(None,"dialog.choose-folder"),"Выбрать папку");assert_eq!(caption(Some("invalid"),"dialog.choose-folder"),caption(Some("ru"),"dialog.choose-folder"));assert_eq!(caption(Some("en"),"dialog.choose-folder"),"Choose folder");assert_eq!(caption(Some("de"),"dialog.choose-folder"),"Ordner auswählen");assert_eq!(caption(Some("fr"),"dialog.choose-folder"),"Choisir un dossier");}
}
