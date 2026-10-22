//! Print the bounded, oriented sRGB image through a real Windows printer dialog.
use crate::*;
use windows_sys::Win32::{Foundation::GlobalFree,Graphics::Gdi::*,Storage::Xps::*,System::Com::*,UI::Controls::Dialogs::*};

struct Printer(PRINTDLGW);
pub fn trace(stage:&str){if std::env::var("SMART_PHOTO_NATIVE_OS_TRACE").as_deref()==Ok("1"){eprintln!("native-print:{stage}");}}
static PRINT_GATE:std::sync::Mutex<()>=std::sync::Mutex::new(());
impl Drop for Printer {
    fn drop(&mut self) { unsafe {
        if !self.0.hDC.is_null() { DeleteDC(self.0.hDC); }
        if !self.0.hDevMode.is_null() { GlobalFree(self.0.hDevMode); }
        if !self.0.hDevNames.is_null() { GlobalFree(self.0.hDevNames); }
    } }
}
pub fn print(path:&Path,owner:usize)->Result<bool,String>{
    let _gate=PRINT_GATE.try_lock().map_err(|_|"Сначала закройте открытый диалог печати.")?;
    let path=path.to_path_buf();
    std::thread::Builder::new().name("smart-photo-printer".into()).spawn(move||print_in_sta(&path,owner))
        .map_err(|error|error.to_string())?.join().map_err(|_|"Работник печати завершился с ошибкой.".to_string())?
}
struct ComApartment;
impl Drop for ComApartment{fn drop(&mut self){unsafe{CoUninitialize();}}}
// A supported dialog hook keeps the common dialog inside this process, avoiding
// the separate WinUI print broker and its stalled modal proxy on affected Windows installations.
unsafe extern "system" fn print_hook(_:windows_sys::Win32::Foundation::HWND,_:u32,_:usize,_:isize)->usize{0}
fn print_in_sta(path:&Path,owner:usize)->Result<bool,String>{
    trace("worker-entry");
    let initialized=unsafe{CoInitializeEx(std::ptr::null(),(COINIT_APARTMENTTHREADED|COINIT_DISABLE_OLE1DDE) as u32)};
    if initialized<0{return Err(format!("Не удалось подготовить системный диалог печати (HRESULT {initialized})."));}
    let _apartment=ComApartment;
    trace("com-initialized");
    let expected=compute_file_sha256(path)?;
    let image=file_safety::decode_bounded(path)?;
    let prepared=image.thumbnail(4096,4096).to_rgba8();drop(image);
    trace("pixels-prepared");
    if compute_file_sha256(path)?!=expected{return Err("Файл изменился во время подготовки печати.".into());}
    let (width,height)=prepared.dimensions();let mut pixels=prepared.into_raw();
    // GDI BI_RGB uses BGR. Composite alpha onto white paper, without another image buffer.
    for pixel in pixels.chunks_exact_mut(4){let alpha=u32::from(pixel[3]);for value in pixel.iter_mut().take(3){*value=((u32::from(*value)*alpha+255*(255-alpha)+127)/255) as u8;}pixel.swap(0,2);pixel[3]=255;}
    let mut printer=Printer(unsafe{std::mem::zeroed()});
    printer.0.lStructSize=std::mem::size_of::<PRINTDLGW>() as u32;
    printer.0.hwndOwner=owner as _;
    printer.0.Flags=PD_RETURNDC|PD_NOSELECTION|PD_NOPAGENUMS|PD_HIDEPRINTTOFILE|PD_USEDEVMODECOPIESANDCOLLATE|PD_ENABLEPRINTHOOK;
    printer.0.lpfnPrintHook=Some(print_hook);
    printer.0.nCopies=1;
    trace("dialog-opening");
    if unsafe{PrintDlgW(&mut printer.0)}==0{
        let code=unsafe{CommDlgExtendedError()};if code==0{return Ok(false);}
        return Err(format!("Не удалось открыть системный диалог печати (код {code}). Проверьте доступность принтера."));
    }
    trace("dialog-confirmed");
    let dc=printer.0.hDC;if dc.is_null(){return Err("Windows не вернула устройство печати.".into());}
    let paper_width=unsafe{GetDeviceCaps(dc,HORZRES as i32)};let paper_height=unsafe{GetDeviceCaps(dc,VERTRES as i32)};
    if paper_width<=0||paper_height<=0{return Err("Принтер вернул неверный размер страницы.".into());}
    let dpi_x=unsafe{GetDeviceCaps(dc,LOGPIXELSX as i32)};let dpi_y=unsafe{GetDeviceCaps(dc,LOGPIXELSY as i32)};
    if dpi_x<=0||dpi_y<=0{return Err("Принтер вернул неверное разрешение страницы.".into());}
    let scale=(f64::from(paper_width)/(f64::from(width)*f64::from(dpi_x))).min(f64::from(paper_height)/(f64::from(height)*f64::from(dpi_y)));
    let dest_width=(f64::from(width)*scale*f64::from(dpi_x)).round() as i32;let dest_height=(f64::from(height)*scale*f64::from(dpi_y)).round() as i32;
    let title=wide_null_from_os_str(path.file_name().unwrap_or(path.as_os_str()));
    let document=DOCINFOW{cbSize:std::mem::size_of::<DOCINFOW>() as i32,lpszDocName:title.as_ptr(),lpszOutput:std::ptr::null(),lpszDatatype:std::ptr::null(),fwType:0};
    if unsafe{StartDocW(dc,&document)}<=0{return Err("Печать отменена или Windows не смогла начать документ.".into());}
    let result=(||{
        if unsafe{StartPage(dc)}<=0{return Err("Не удалось начать страницу печати.".to_string());}
        let info=BITMAPINFO{bmiHeader:BITMAPINFOHEADER{biSize:std::mem::size_of::<BITMAPINFOHEADER>() as u32,biWidth:width as i32,biHeight:-(height as i32),biPlanes:1,biBitCount:32,biCompression:BI_RGB,..unsafe{std::mem::zeroed()}},bmiColors:unsafe{std::mem::zeroed()}};
        unsafe{SetStretchBltMode(dc,HALFTONE);SetBrushOrgEx(dc,0,0,std::ptr::null_mut());}
        let copied=unsafe{StretchDIBits(dc,(paper_width-dest_width)/2,(paper_height-dest_height)/2,dest_width,dest_height,0,0,width as i32,height as i32,pixels.as_ptr().cast(),&info,DIB_RGB_COLORS,SRCCOPY)};
        if copied==0 || copied==-1{return Err("Не удалось передать изображение принтеру.".into());}
        if unsafe{EndPage(dc)}<=0 || unsafe{EndDoc(dc)}<=0{return Err("Windows не смогла завершить печать документа.".into());}
        Ok(true)
    })();if result.is_err(){unsafe{AbortDoc(dc);}}result
}
