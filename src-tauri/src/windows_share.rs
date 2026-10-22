use std::{cell::RefCell, collections::HashMap, path::Path};
use windows::{
    core::{factory, HSTRING, Interface, AgileReference},
    ApplicationModel::DataTransfer::{DataTransferManager, DataRequestedEventArgs, DataPackageOperation},
    Foundation::TypedEventHandler,
    Storage::{StorageFile, IStorageItem},
    Win32::{Foundation::HWND, System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED}, UI::Shell::IDataTransferManagerInterop},
};

struct Registration { manager: DataTransferManager, token: i64 }
impl Drop for Registration {
    fn drop(&mut self) { let _ = self.manager.RemoveDataRequested(self.token); }
}
// WinRT managers belong to the window's UI apartment. Retain each handler until
// its next replacement or window destruction, never on a blocking-pool thread.
thread_local! { static REGISTRATIONS: RefCell<HashMap<usize, Registration>> = RefCell::new(HashMap::new()); }

pub fn prepare(path: &Path) -> Result<AgileReference<StorageFile>, String> {
    crate::windows_print::trace("share-prepare");
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| error.to_string())?;
    struct Apartment;
    impl Drop for Apartment { fn drop(&mut self) { unsafe { RoUninitialize() }; } }
    let _apartment = Apartment;
    let canonical = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    let raw = canonical.to_string_lossy();
    let normal = if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") { format!(r"\\{unc}") }
        else { raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string() };
    StorageFile::GetFileFromPathAsync(&HSTRING::from(normal))
        .and_then(|operation| operation.get())
        .and_then(|file| AgileReference::new(&file))
        .map_err(|error| format!("Не удалось подготовить файл для панели «Поделиться»: {error}"))
}

pub fn show(owner: usize, file: AgileReference<StorageFile>, title: String) -> Result<(), String> {
    crate::windows_print::trace("share-ui-enter");
    let result = (|| -> windows::core::Result<()> {
        let interop: IDataTransferManagerInterop = factory::<DataTransferManager, IDataTransferManagerInterop>()?;
        let hwnd = HWND(owner as *mut _);
        let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd) }?;
        REGISTRATIONS.with(|registrations| {
            let mut entries = registrations.borrow_mut();
            entries.retain(|handle, _| unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(*handle as *mut _) } != 0);
            entries.remove(&owner);
        });
        let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(move |_, args| {
            crate::windows_print::trace("share-data-requested");
            let request = args.as_ref().ok_or_else(|| windows::core::Error::from(windows::core::HRESULT(0x80004003u32 as i32)))?.Request()?;
            let data = request.Data()?;
            data.Properties()?.SetTitle(&HSTRING::from(&title))?;
            data.SetRequestedOperation(DataPackageOperation::Copy)?;
            let files: windows_collections::IIterable<IStorageItem> = vec![Some(file.resolve()?.cast()?)].into();
            data.SetStorageItems(&files, true)?;
            crate::windows_print::trace("share-data-ready");
            Ok(())
        });
        let token = manager.DataRequested(&handler)?;
        REGISTRATIONS.with(|registrations| { registrations.borrow_mut().insert(owner, Registration { manager, token }); });
        if let Err(error) = unsafe { interop.ShowShareUIForWindow(hwnd) } {
            release(owner);
            return Err(error);
        }
        crate::windows_print::trace("share-show-returned");
        Ok(())
    })();
    result.map_err(|error| format!("Не удалось открыть панель Windows «Поделиться»: {error}"))
}

pub fn release(owner: usize) {
    REGISTRATIONS.with(|registrations| { registrations.borrow_mut().remove(&owner); });
}
