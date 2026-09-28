use dioxus::desktop::{DesktopContext, tao::platform::windows::WindowExtWindows};
use serde_json::{Value, json};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, HWND},
    System::Threading::CreateMutexW,
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongW, LWA_ALPHA, SW_SHOWNORMAL, SetLayeredWindowAttributes,
            SetWindowLongW, WS_EX_LAYERED,
        },
    },
};

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

pub fn known_folders() -> Value {
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Videos, SHGetKnownFolderPath,
        },
    };
    let mut folders = json!({});
    for (name, id) in [
        ("documents", FOLDERID_Documents),
        ("downloads", FOLDERID_Downloads),
        ("videos", FOLDERID_Videos),
    ] {
        let mut pointer = std::ptr::null_mut();
        unsafe {
            if SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut pointer) >= 0
                && !pointer.is_null()
            {
                let mut length = 0;
                while *pointer.add(length) != 0 {
                    length += 1;
                }
                folders[name] = json!(String::from_utf16_lossy(std::slice::from_raw_parts(
                    pointer, length
                )));
            }
            if !pointer.is_null() {
                CoTaskMemFree(pointer.cast());
            }
        }
    }
    folders
}

pub fn set_opacity(window: &DesktopContext, opacity: f64) {
    let hwnd = window.hwnd() as HWND;
    // Only this process's window handle is used, and the opacity is bounded.
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        // Tao regenerates EXSTYLE on show/topmost/focusability changes. Keep its
        // style updates from dropping the per-window alpha between fade frames.
        windows_sys::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(opacity_subclass), 1, 0);
        if style & WS_EX_LAYERED as i32 == 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as i32);
        }
        SetLayeredWindowAttributes(
            hwnd,
            0,
            (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            LWA_ALPHA,
        );
    }
}

unsafe extern "system" fn opacity_subclass(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    id: usize,
    _data: usize,
) -> isize {
    use windows_sys::Win32::UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass},
        WindowsAndMessaging::{STYLESTRUCT, WM_NCDESTROY, WM_STYLECHANGING},
    };
    unsafe {
        if message == WM_STYLECHANGING && wparam as i32 == GWL_EXSTYLE && lparam != 0 {
            // STYLESTRUCT is valid only for this synchronous WM_STYLECHANGING call.
            (*(lparam as *mut STYLESTRUCT)).styleNew |= WS_EX_LAYERED;
        }
        if message == WM_NCDESTROY {
            RemoveWindowSubclass(hwnd, Some(opacity_subclass), id);
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

pub struct Instance(HANDLE);
thread_local! { static INSTANCE: std::cell::RefCell<Option<Instance>> = const { std::cell::RefCell::new(None) }; }
pub fn retain_instance(instance: Instance) {
    INSTANCE.with(|slot| *slot.borrow_mut() = Some(instance));
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
pub fn acquire_instance() -> Result<Option<Instance>, String> {
    let name = wide(format!(
        "Local\\nogirem-dioxus-{}",
        std::env::var("USERNAME").unwrap_or_default()
    ));
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if unsafe { GetLastError() } == 183 {
        unsafe {
            CloseHandle(handle);
        }
        if !std::env::args().any(|arg| arg == "--startup-tray") {
            let app_data = std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?;
            let directory = Path::new(&app_data).join("마비노기 렘 부스터/instance");
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let request = json!({"requestId":format!("{}-{now}", std::process::id()),"requestedAt":now,"requesterPid":std::process::id(),"shouldFocus":true});
            let path = directory.join(format!("focus-request-{}.tmp", std::process::id()));
            std::fs::write(&path, request.to_string()).map_err(|e| e.to_string())?;
            std::fs::rename(path, directory.join("focus-request.json"))
                .map_err(|e| e.to_string())?;
        }
        return Ok(None);
    }
    Ok(Some(Instance(handle)))
}

fn shell_execute(verb: &str, file: &str, parameters: Option<&str>) -> Result<(), String> {
    let verb = wide(verb);
    let file = wide(file);
    let parameters = parameters.map(wide);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        Err(format!(
            "Windows shell operation failed: {}",
            result as isize
        ))
    } else {
        Ok(())
    }
}

pub async fn dispatch(
    method: &str,
    params: &Value,
    parent: Option<DesktopContext>,
) -> Result<Value, String> {
    match method {
        "application.relaunch" => {
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let args = params["args"]
                .as_array()
                .map(|args| {
                    args.iter()
                        .filter_map(Value::as_str)
                        .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            INSTANCE.with(|slot| slot.borrow_mut().take());
            if let Err(error) = shell_execute(
                if params["elevated"] == true {
                    "runas"
                } else {
                    "open"
                },
                &executable.to_string_lossy(),
                Some(&args),
            ) {
                if let Ok(Some(instance)) = acquire_instance() {
                    retain_instance(instance);
                }
                return Err(error);
            }
        }
        "shell.openExternal" => {
            let url = params["path"].as_str().ok_or("Missing URL")?;
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return Err("Only HTTP(S) links may be opened".into());
            }
            shell_execute("open", url, None)?;
        }
        "shell.openPath" => {
            shell_execute("open", params["path"].as_str().ok_or("Missing path")?, None)?;
            return Ok(json!(""));
        }
        "shell.showItemInFolder" => {
            let path = params["path"].as_str().ok_or("Missing path")?;
            if path.contains('"') {
                return Err("Invalid file path".into());
            }
            shell_execute("open", "explorer.exe", Some(&format!("/select,\"{path}\"")))?;
        }
        "dialog.error" => {
            rfd::AsyncMessageDialog::new()
                .set_title(params["title"].as_str().unwrap_or("오류"))
                .set_description(params["content"].as_str().unwrap_or(""))
                .set_level(rfd::MessageLevel::Error)
                .show()
                .await;
        }
        "dialog.showMessageBox" => {
            let options = &params["options"];
            let labels = options["buttons"]
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_else(|| vec!["확인"]);
            let buttons = match labels.as_slice() {
                [a] => rfd::MessageButtons::OkCustom((*a).into()),
                [a, b] => rfd::MessageButtons::OkCancelCustom((*a).into(), (*b).into()),
                [a, b, c] => {
                    rfd::MessageButtons::YesNoCancelCustom((*a).into(), (*b).into(), (*c).into())
                }
                _ => return Err("Unsupported dialog button count".into()),
            };
            let mut dialog = rfd::AsyncMessageDialog::new()
                .set_title(options["title"].as_str().unwrap_or("마비노기 렘 부스터"))
                .set_description(format!(
                    "{}\n\n{}",
                    options["message"].as_str().unwrap_or(""),
                    options["detail"].as_str().unwrap_or("")
                ))
                .set_buttons(buttons);
            if let Some(parent) = parent.as_ref() {
                dialog = dialog.set_parent(&parent.window);
            }
            let result = dialog.show().await;
            let cancel = options["cancelId"]
                .as_u64()
                .unwrap_or(labels.len().saturating_sub(1) as u64);
            let response = match result {
                rfd::MessageDialogResult::Custom(label) => labels
                    .iter()
                    .position(|s| *s == label)
                    .map(|n| n as u64)
                    .unwrap_or(cancel),
                rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => 0,
                rfd::MessageDialogResult::No => 1,
                _ => cancel,
            };
            return Ok(json!({"response":response,"checkboxChecked":false}));
        }
        "dialog.showOpenDialog" | "dialog.showSaveDialog" => {
            let options = &params["options"];
            let mut dialog = rfd::AsyncFileDialog::new()
                .set_title(options["title"].as_str().unwrap_or("파일 선택"));
            if let Some(parent) = parent.as_ref() {
                dialog = dialog.set_parent(&parent.window);
            }
            if let Some(path) = options["defaultPath"].as_str() {
                let path = Path::new(path);
                if path.is_dir() {
                    dialog = dialog.set_directory(path);
                } else {
                    if let Some(parent) = path.parent() {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(name) = path.file_name() {
                        dialog = dialog.set_file_name(name.to_string_lossy());
                    }
                }
            }
            if let Some(filters) = options["filters"].as_array() {
                for filter in filters {
                    let extensions = filter["extensions"]
                        .as_array()
                        .map(|v| v.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                        .unwrap_or_default();
                    dialog =
                        dialog.add_filter(filter["name"].as_str().unwrap_or("Files"), &extensions);
                }
            }
            if method == "dialog.showSaveDialog" {
                let file = dialog.save_file().await;
                return Ok(
                    json!({"canceled":file.is_none(),"filePath":file.map(|f| f.path().to_string_lossy().into_owned())}),
                );
            }
            let folder = options["properties"]
                .as_array()
                .is_some_and(|v| v.contains(&json!("openDirectory")));
            let file = if folder {
                dialog.pick_folder().await
            } else {
                dialog.pick_file().await
            };
            return Ok(
                json!({"canceled":file.is_none(),"filePaths":file.into_iter().map(|f| f.path().to_string_lossy().into_owned()).collect::<Vec<_>>()}),
            );
        }
        _ => return Err(format!("Native method is not implemented: {method}")),
    }
    Ok(Value::Null)
}

// Electron's frameless border/corner options must also be applied to DWM.
pub fn configure_frame(window: &DesktopContext, rounded: bool) {
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
        DwmSetWindowAttribute,
    };
    // Remove the native non-client edge as well: DWM attributes are unavailable on Windows 10.
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SetWindowPos, WS_CAPTION, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_WINDOWEDGE,
    };
    unsafe {
        let hwnd = window.hwnd() as HWND;
        SetWindowLongW(
            hwnd,
            GWL_STYLE,
            GetWindowLongW(hwnd, GWL_STYLE) & !(WS_CAPTION as i32),
        );
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            GetWindowLongW(hwnd, GWL_EXSTYLE)
                & !((WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME) as i32),
        );
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    let border: u32 = 0xfffffffe; // DWMWA_COLOR_NONE: suppress the system's contrasting outline.
    let corner = if rounded {
        DWMWCP_ROUND
    } else {
        DWMWCP_DONOTROUND
    };
    unsafe {
        DwmSetWindowAttribute(
            window.hwnd() as _,
            DWMWA_BORDER_COLOR as u32,
            &border as *const _ as _,
            std::mem::size_of_val(&border) as _,
        );
        DwmSetWindowAttribute(
            window.hwnd() as _,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as _,
            std::mem::size_of_val(&corner) as _,
        );
    }
}

// Hide the composed window before Wry drops its child HWND/WebView surface.
// Otherwise Windows can animate the empty host frame during teardown.
pub fn hide_for_close(window: &DesktopContext) {
    use windows_sys::Win32::{
        Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmFlush, DwmSetWindowAttribute},
        UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
    };
    let disabled: i32 = 1;
    unsafe {
        DwmSetWindowAttribute(
            window.hwnd() as _,
            DWMWA_TRANSITIONS_FORCEDISABLED as _,
            &disabled as *const _ as _,
            std::mem::size_of_val(&disabled) as _,
        );
        ShowWindow(window.hwnd() as _, SW_HIDE);
        DwmFlush();
    }
}

// Elevate only interactive application startup. Read-only contract probes and
// isolated smoke tests can run without UAC; child workers inherit the GUI token.
pub fn ensure_elevated() -> Result<bool, String> {
    use windows_sys::Win32::{Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},System::Threading::{GetCurrentProcess,OpenProcessToken}};
    let mut token=std::ptr::null_mut();
    if unsafe {OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&mut token)}==0 {return Err(std::io::Error::last_os_error().to_string());}
    let mut elevation=TOKEN_ELEVATION{TokenIsElevated:0}; let mut size=0;
    let ok=unsafe {GetTokenInformation(token,TokenElevation,&mut elevation as *mut _ as _,std::mem::size_of::<TOKEN_ELEVATION>() as u32,&mut size)};
    let failure=std::io::Error::last_os_error(); unsafe {CloseHandle(token);}
    if ok==0{return Err(failure.to_string());}
    if elevation.TokenIsElevated!=0{return Ok(true);}
    let args=std::env::args().skip(1).map(|v| quote_argument(&v)).collect::<Vec<_>>().join(" ");
    let exe=std::env::current_exe().map_err(|e|e.to_string())?;
    INSTANCE.with(|slot|slot.borrow_mut().take());
    shell_execute("runas", &exe.to_string_lossy(), Some(&args))?;
    Ok(false)
}
fn quote_argument(value:&str)->String {
    let mut result=String::from("\"");let mut backslashes=0;
    for ch in value.chars(){
        if ch=='\\'{backslashes+=1;continue;}
        if ch=='"'{result.push_str(&"\\".repeat(backslashes*2+1));}else{result.push_str(&"\\".repeat(backslashes));}
        backslashes=0;result.push(ch);
    }
    result.push_str(&"\\".repeat(backslashes*2));result.push('"');result
}
#[cfg(test)]
mod elevation_tests {
    use super::*;
    #[test] fn windows_arguments_preserve_spaces_quotes_and_trailing_slashes(){
        assert_eq!(quote_argument("hello world"),"\"hello world\"");
        assert_eq!(quote_argument("a\"b"),"\"a\\\"b\"");
        assert_eq!(quote_argument("C:\\path\\"),"\"C:\\path\\\\\"");
    }
}

pub fn show_inactive(window:&DesktopContext) {
    // Tao's ignore-cursor setter rewrites WS_CAPTION even for undecorated windows.
    // Strip its regenerated non-client frame after all setters, before first visibility.
    configure_frame(window,false);
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_SHOWNOACTIVATE};
    unsafe {ShowWindow(window.hwnd() as _,SW_SHOWNOACTIVATE);}
}
