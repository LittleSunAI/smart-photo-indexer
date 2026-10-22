use serde_json::{json,Value};

pub fn available(provider:&str)->bool{
    provider!="openrouter" || serde_json::from_str::<Value>(include_str!("../../src/shared/config/provider-availability.json")).ok().and_then(|v|v["openrouter"].as_bool()).unwrap_or(false)
}
pub fn message()->&'static str{crate::ui_locale_generated::caption(None,"provider.openrouter-unavailable")}
pub fn require(provider:&str)->Result<(),String>{if available(provider){Ok(())}else{Err(message().into())}}
pub fn status(provider:&str)->Option<Value>{(!available(provider)).then(||json!({"ready":false,"available":false,"disabled":true,"message":message()}))}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn release_gate_blocks_only_direct_openrouter(){let flag:Value=serde_json::from_str(include_str!("../../src/shared/config/provider-availability.json")).unwrap();assert_eq!(available("openrouter"),flag["openrouter"].as_bool().unwrap());assert!(available("fal"));assert!(available("local"));assert_eq!(require("openrouter").is_ok(),available("openrouter"));assert!(require("fal").is_ok());if !available("openrouter"){let state=status("openrouter").unwrap();assert_eq!(state["ready"],false);assert_eq!(state["disabled"],true);}assert!(status("fal").is_none());}
}
