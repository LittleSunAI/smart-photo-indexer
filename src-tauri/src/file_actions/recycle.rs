//! One explicit recycle-only Shell operation on its own STA. No hard-delete fallback.
use super::{io,types::Fingerprint};
use crate::*;

pub fn recycle(path:&str,expected:&Fingerprint)->Result<serde_json::Value,String>{
    let path=path.to_string();let expected=expected.clone();
    std::thread::spawn(move||sta(&path,&expected)).join().map_err(|_|"Не удалось завершить отправку в корзину.".to_string())?
}
#[cfg(not(windows))]
fn sta(_: &str,_:&Fingerprint)->Result<serde_json::Value,String>{Err("Корзина доступна в Windows.".into())}

#[cfg(windows)]
mod shell {
    use super::*;
    use std::sync::{Arc,Mutex};
    use windows::{core::{implement,Ref,PCWSTR,HRESULT}, Win32::{Foundation::{E_FAIL,E_ACCESSDENIED},
        System::Com::{CoCreateInstance,CoInitializeEx,CoUninitialize,CoTaskMemFree,CLSCTX_INPROC_SERVER,COINIT_APARTMENTTHREADED},
        UI::Shell::*}};
    type WResult=windows::core::Result<()>;
    struct Apartment;
    impl Drop for Apartment {fn drop(&mut self){unsafe{CoUninitialize()};}}
    #[implement(IFileOperationProgressSink)]
    struct Sink { expected:Fingerprint, receipt:Arc<Mutex<Option<serde_json::Value>>> }
    fn display(item:&IShellItem,kind:SIGDN)->windows::core::Result<String>{unsafe{
        let text=item.GetDisplayName(kind)?;let value=text.to_string();CoTaskMemFree(Some(text.0 as _));value.map_err(|_|windows::core::Error::from(E_FAIL))
    }}
    #[allow(non_snake_case,unused_variables)]
    impl IFileOperationProgressSink_Impl for Sink_Impl {
        fn StartOperations(&self)->WResult{Ok(())}
        fn FinishOperations(&self,hr:HRESULT)->WResult{hr.ok()}
        fn PreDeleteItem(&self,flags:u32,item:Ref<'_,IShellItem>)->WResult{
            let item=item.as_ref().ok_or_else(||windows::core::Error::from(E_FAIL))?;
            let path=display(item,SIGDN_FILESYSPATH)?;
            if io::at(Path::new(&path)).ok().flatten().as_ref()!=Some(&self.expected){return Err(E_ACCESSDENIED.into());}
            Ok(())
        }
        fn PostDeleteItem(&self,flags:u32,item:Ref<'_,IShellItem>,hr:HRESULT,new_item:Ref<'_,IShellItem>)->WResult{
            hr.ok()?;
            let recycled=new_item.as_ref().ok_or_else(||windows::core::Error::from(E_FAIL))?;
            let path=display(recycled,SIGDN_FILESYSPATH)?;
            let fingerprint=io::at(Path::new(&path)).ok().flatten().ok_or_else(||windows::core::Error::from(E_FAIL))?;
            if fingerprint.sha256!=self.expected.sha256{return Err(E_FAIL.into());}
            *self.receipt.lock().map_err(|_|windows::core::Error::from(E_FAIL))?=Some(serde_json::json!({
                "recycledPath":path,"identity":fingerprint.identity,"sha256":fingerprint.sha256}));
            Ok(())
        }
        fn PreRenameItem(&self,f:u32,i:Ref<'_,IShellItem>,n:&PCWSTR)->WResult{Err(E_ACCESSDENIED.into())}
        fn PostRenameItem(&self,f:u32,i:Ref<'_,IShellItem>,n:&PCWSTR,h:HRESULT,new:Ref<'_,IShellItem>)->WResult{Err(E_ACCESSDENIED.into())}
        fn PreMoveItem(&self,f:u32,i:Ref<'_,IShellItem>,d:Ref<'_,IShellItem>,n:&PCWSTR)->WResult{Err(E_ACCESSDENIED.into())}
        fn PostMoveItem(&self,f:u32,i:Ref<'_,IShellItem>,d:Ref<'_,IShellItem>,n:&PCWSTR,h:HRESULT,new:Ref<'_,IShellItem>)->WResult{Err(E_ACCESSDENIED.into())}
        fn PreCopyItem(&self,f:u32,i:Ref<'_,IShellItem>,d:Ref<'_,IShellItem>,n:&PCWSTR)->WResult{Err(E_ACCESSDENIED.into())}
        fn PostCopyItem(&self,f:u32,i:Ref<'_,IShellItem>,d:Ref<'_,IShellItem>,n:&PCWSTR,h:HRESULT,new:Ref<'_,IShellItem>)->WResult{Err(E_ACCESSDENIED.into())}
        fn PreNewItem(&self,f:u32,d:Ref<'_,IShellItem>,n:&PCWSTR)->WResult{Err(E_ACCESSDENIED.into())}
        fn PostNewItem(&self,f:u32,d:Ref<'_,IShellItem>,n:&PCWSTR,t:&PCWSTR,a:u32,h:HRESULT,new:Ref<'_,IShellItem>)->WResult{Err(E_ACCESSDENIED.into())}
        fn UpdateProgress(&self,total:u32,done:u32)->WResult{Ok(())}
        fn ResetTimer(&self)->WResult{Ok(())}
        fn PauseTimer(&self)->WResult{Ok(())}
        fn ResumeTimer(&self)->WResult{Ok(())}
    }
    pub(super) fn run(path:&str,expected:&Fingerprint)->windows::core::Result<serde_json::Value>{unsafe{
        CoInitializeEx(None,COINIT_APARTMENTTHREADED).ok()?;let _apartment=Apartment;
        let operation:IFileOperation=CoCreateInstance(&FileOperation,None,CLSCTX_INPROC_SERVER)?;
        operation.SetOperationFlags(FOFX_RECYCLEONDELETE|FOF_SILENT|FOF_NOCONFIRMATION|FOF_NOERRORUI|FOFX_EARLYFAILURE|FOF_WANTNUKEWARNING|FOF_NO_CONNECTED_ELEMENTS)?;
        let path:Vec<u16>=path.encode_utf16().chain(std::iter::once(0)).collect();
        let item:IShellItem=SHCreateItemFromParsingName(PCWSTR(path.as_ptr()),None)?;
        let receipt=Arc::new(Mutex::new(None));
        let sink:IFileOperationProgressSink=Sink{expected:expected.clone(),receipt:receipt.clone()}.into();
        operation.DeleteItem(&item,&sink)?;operation.PerformOperations()?;
        if operation.GetAnyOperationsAborted()?.as_bool(){return Err(E_FAIL.into());}
        let result=receipt.lock().map_err(|_|windows::core::Error::from(E_FAIL))?.clone().ok_or_else(||windows::core::Error::from(E_FAIL));result
    }}
    #[cfg(test)]
    mod tests{
        use super::*;
        #[test]
        fn failed_or_missing_shell_receipt_is_not_success(){
            let receipt=Arc::new(Mutex::new(None));
            let sink:IFileOperationProgressSink=Sink{expected:Fingerprint{identity:"fixture".into(),sha256:"sha".into(),size_bytes:1,modified_ns:"1".into()},receipt:receipt.clone()}.into();
            unsafe{
                assert!(sink.PostDeleteItem(0,None::<&IShellItem>,E_FAIL,None::<&IShellItem>).is_err());
                assert!(sink.PostDeleteItem(0,None::<&IShellItem>,HRESULT(0),None::<&IShellItem>).is_err());
                assert!(sink.FinishOperations(E_FAIL).is_err());
            }
            assert!(receipt.lock().unwrap().is_none());
        }
    }
}
#[cfg(windows)]
fn sta(path:&str,expected:&Fingerprint)->Result<serde_json::Value,String>{
    shell::run(path,expected).map_err(|e|format!("Не удалось отправить фотографию в корзину. Безвозвратное удаление не выполнялось: {e}"))
}

pub fn existing(path:&Path)->Result<(),String>{
    let expected=io::at(path)?.ok_or("Исходная фотография не найдена")?;
    recycle(&path_to_string(path),&expected).map(|_|())
}
