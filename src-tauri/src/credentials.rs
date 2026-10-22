use crate::*;
use serde_json::{json,Value};
use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

pub const SCHEMA:&str=r#"
    CREATE TABLE provider_credentials (
        id TEXT PRIMARY KEY, provider TEXT NOT NULL, protected_key BLOB NOT NULL,
        active INTEGER NOT NULL DEFAULT 1, created_at_unix INTEGER NOT NULL
    );
    CREATE INDEX idx_provider_credentials_active ON provider_credentials(provider,active);
"#;
#[derive(Default)]
pub struct CredentialVault { ephemeral:Mutex<HashMap<String,(String,Zeroizing<String>)>> }

#[cfg(windows)]
fn protect(bytes:&[u8],decrypt:bool)->Result<Zeroizing<Vec<u8>>,String>{
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData,CryptUnprotectData,CRYPT_INTEGER_BLOB,CRYPTPROTECT_UI_FORBIDDEN};
    use windows_sys::Win32::Foundation::LocalFree;
    let input=CRYPT_INTEGER_BLOB{cbData:bytes.len().try_into().map_err(|_|"Credential too long.")?,pbData:bytes.as_ptr() as *mut u8};
    let mut output=CRYPT_INTEGER_BLOB{cbData:0,pbData:std::ptr::null_mut()};
    // Windows owns the result allocation; it must be wiped and released on both paths.
    let success=unsafe{
        if decrypt{CryptUnprotectData(&input,std::ptr::null_mut(),std::ptr::null(),std::ptr::null(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}
        else{CryptProtectData(&input,std::ptr::null(),std::ptr::null(),std::ptr::null(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}
    };
    if success==0{return Err("Windows could not protect/unlock this provider credential.".into());}
    let result=unsafe{
        let slice=std::slice::from_raw_parts_mut(output.pbData,output.cbData as usize);
        let result=Zeroizing::new(slice.to_vec());slice.fill(0);LocalFree(output.pbData as _);result
    };
    Ok(result)
}

#[tauri::command]
pub async fn read_clipboard_text()->Result<String,String>{
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(windows)] {
            use windows_sys::Win32::System::DataExchange::{OpenClipboard,CloseClipboard,GetClipboardData};
            use windows_sys::Win32::System::Memory::{GlobalLock,GlobalUnlock,GlobalSize};
            struct ClipboardGuard;impl Drop for ClipboardGuard{fn drop(&mut self){unsafe{CloseClipboard();}}}
            unsafe {
                if OpenClipboard(std::ptr::null_mut())==0{return Err("Буфер обмена занят. Повторите вставку.".into());}
                let _guard=ClipboardGuard;
                let handle=GetClipboardData(13);if handle.is_null(){return Err("В буфере обмена нет текста.".into());}
                let size=GlobalSize(handle);if size==0 || size>16384{return Err("Текст в буфере слишком большой для ключа.".into());}
                let pointer=GlobalLock(handle);if pointer.is_null(){return Err("Не удалось прочитать текст буфера обмена.".into());}
                let chars=std::slice::from_raw_parts(pointer as *const u16,size/2);
                let result=match chars.iter().position(|value|*value==0){Some(end)=>String::from_utf16(&chars[..end]).map_err(|_|"Текст буфера имеет неверную кодировку.".to_string()),None=>Err("Некорректный текст буфера обмена.".into())};
                GlobalUnlock(handle);result
            }
        }
        #[cfg(not(windows))] {Err("Используйте обычную вставку текста в поле ключа.".into())}
    }).await.map_err(|_|"Не удалось прочитать буфер обмена.".to_string())?
}
#[cfg(not(windows))]
fn protect(_bytes:&[u8],_decrypt:bool)->Result<Zeroizing<Vec<u8>>,String>{
    Err("Persistent credentials require an OS credential store on this platform.".into())
}
fn provider_valid(provider:&str)->Result<(),String>{
    if !["fal","openrouter"].contains(&provider){return Err("Unknown credential provider.".into());}Ok(())
}

fn normalize_key(key:&str)->Result<Zeroizing<String>,String>{
    if key.len()>16384{return Err("Ключ слишком длинный.".into());}
    let normalized=Zeroizing::new(key.chars().filter(|c|!c.is_whitespace() && *c!='\u{200b}' && *c!='\u{feff}').collect::<String>());
    if normalized.is_empty(){return Err("Ключ пустой.".into());}
    if normalized.len()>4096{return Err("Ключ слишком длинный.".into());}
    if normalized.chars().any(char::is_control){return Err("Ключ содержит недопустимые символы.".into());}
    Ok(normalized)
}

#[tauri::command]
pub async fn save_provider_credential(app:tauri::AppHandle,provider:String,key:String,persistent:Option<bool>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        provider_valid(&provider)?;let submitted=Zeroizing::new(key);let key=normalize_key(&submitted)?;
        let normalized=key.as_str()!=submitted.as_str();
        let nonce=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
        let id=format!("credential-{provider}-{}-{nonce}",std::process::id());
        if persistent.unwrap_or(true){
            let protected=protect(key.as_bytes(),false)?;let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
            tx.execute("UPDATE provider_credentials SET active=0 WHERE provider=?1",[&provider]).map_err(|e|e.to_string())?;
            tx.execute("INSERT INTO provider_credentials(id,provider,protected_key,created_at_unix) VALUES(?1,?2,?3,?4)",params![id,provider,protected.as_slice(),unix_now()]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
        }else{
            app.state::<CredentialVault>().ephemeral.lock().map_err(|_|"Credential store lock failed.")?.insert(id.clone(),(provider.clone(),key));
        }
        Ok(json!({"provider":provider,"credentialRef":id,"stored":true,"persistent":persistent.unwrap_or(true),"normalized":normalized}))
    }).await.map_err(|e|e.to_string())?
}

pub fn unlock(app:&tauri::AppHandle,provider:&str,reference:&str)->Result<Zeroizing<String>,String>{
    provider_valid(provider)?;
    if let Some((stored_provider,key))=app.state::<CredentialVault>().ephemeral.lock().map_err(|_|"Credential store lock failed.")?.get(reference){
        if stored_provider!=provider{return Err("Credential belongs to another provider.".into());}return Ok(Zeroizing::new(key.to_string()));
    }
    let db=open_index_connection(app)?;
    let protected:Vec<u8>=db.query_row("SELECT protected_key FROM provider_credentials WHERE id=?1 AND provider=?2",params![reference,provider],|r|r.get(0)).map_err(|_|"Provider credential is unavailable. Add its key again.".to_string())?;
    let bytes=protect(&protected,true)?;
    let secret=std::str::from_utf8(&bytes).map_err(|_|"Credential encoding is invalid.")?;
    Ok(Zeroizing::new(secret.to_string()))
}

#[tauri::command]
pub async fn get_provider_credentials(app:tauri::AppHandle)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;
        let mut stmt=db.prepare("SELECT provider,id FROM provider_credentials WHERE active=1 ORDER BY created_at_unix DESC,rowid DESC").map_err(|e|e.to_string())?;
        let mut result=Vec::new();
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?{
            let (provider,id)=row.map_err(|e|e.to_string())?;
            let ready=unlock(&app,&provider,&id).is_ok();result.push(json!({"provider":provider,"credentialRef":id,"stored":ready,"persistent":true}));
        }
        for (id,(provider,_)) in app.state::<CredentialVault>().ephemeral.lock().map_err(|_|"Credential store lock failed.")?.iter(){result.push(json!({"provider":provider,"credentialRef":id,"stored":true,"persistent":false}));}
        Ok(json!(result))
    }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]
mod tests{
    #[test]
    fn credentials_are_opaque_and_have_no_provider_format_or_minimum_length(){
        for value in ["x","fal_sk_0123456789abcdef0123456789abcdef","unknown-future-format+/=","короткий"] {
            assert_eq!(super::normalize_key(value).unwrap().as_str(),value);
        }
    }
    #[test]
    fn pasted_key_removes_whitespace_without_leaking_invalid_input(){
        let expected="fal_sk_synthetic-example:secret-part";
        assert_eq!(super::normalize_key(" \nfal_sk_ synthetic-example:\r\nsecret-part\t\u{200b}\u{feff}").unwrap().as_str(),expected);
        assert_eq!(super::normalize_key(expected).unwrap().as_str(),expected);
        assert_eq!(super::normalize_key(" \r\n\t").unwrap_err(),"Ключ пустой.");
        assert_eq!(super::normalize_key("synthetic-key-with\0-control").unwrap_err(),"Ключ содержит недопустимые символы.");
        assert_eq!(super::normalize_key(&"x".repeat(4097)).unwrap_err(),"Ключ слишком длинный.");
    }
    #[cfg(windows)]
    #[test]
    fn credential_is_encrypted_and_round_trips(){
        let plaintext=b"test-only-not-a-service-credential";let encrypted=super::protect(plaintext,false).unwrap();
        assert_ne!(encrypted.as_slice(),plaintext);assert!(!encrypted.windows(plaintext.len()).any(|w|w==plaintext));
        assert_eq!(super::protect(&encrypted,true).unwrap().as_slice(),plaintext);
    }
}
