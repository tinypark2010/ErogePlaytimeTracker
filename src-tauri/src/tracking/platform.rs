#[cfg(windows)]
mod imp {
    use anyhow::Result;
    use std::{
        collections::HashMap,
        sync::{
            OnceLock,
            mpsc::{self, Receiver, Sender},
        },
        thread,
    };
    use windows::Win32::{
        Foundation::{CloseHandle, HWND},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
        },
        UI::{
            Accessibility::{HWINEVENTHOOK, SetWinEventHook},
            WindowsAndMessaging::{
                EVENT_SYSTEM_FOREGROUND, GetForegroundWindow, GetMessageW,
                GetWindowThreadProcessId, MSG, WINEVENT_OUTOFCONTEXT,
            },
        },
    };
    #[derive(Debug, Clone)]
    pub struct ProcessInfo {
        pub path: Option<String>,
        pub parent_pid: u32,
    }
    pub fn processes() -> Result<HashMap<u32, ProcessInfo>> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? };
        let mut out = HashMap::new();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
            loop {
                let pid = entry.th32ProcessID;
                if pid != 0 {
                    out.insert(
                        pid,
                        ProcessInfo {
                            path: process_path(pid),
                            parent_pid: entry.th32ParentProcessID,
                        },
                    );
                }
                if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                    break;
                }
            }
        }
        unsafe { CloseHandle(snapshot)? };
        Ok(out)
    }
    pub fn process_path(pid: u32) -> Option<String> {
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let result = unsafe {
            QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        unsafe {
            let _ = CloseHandle(h);
        }
        result
            .is_ok()
            .then(|| String::from_utf16_lossy(&buf[..len as usize]))
    }
    pub fn foreground_pid() -> Option<u32> {
        unsafe {
            let h: GetHwnd = GetForegroundWindow();
            if h.0.is_null() {
                return None;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(h, Some(&mut pid));
            (pid != 0).then_some(pid)
        }
    }
    type GetHwnd = HWND;
    static EVENTS: OnceLock<Sender<()>> = OnceLock::new();
    unsafe extern "system" fn foreground_event(
        _: HWINEVENTHOOK,
        _: u32,
        _: HWND,
        _: i32,
        _: i32,
        _: u32,
        _: u32,
    ) {
        if let Some(tx) = EVENTS.get() {
            let _ = tx.send(());
        }
    }
    pub fn foreground_events() -> Receiver<()> {
        let (tx, rx) = mpsc::channel();
        let _ = EVENTS.set(tx);
        thread::spawn(|| unsafe {
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.0.is_null() {
                log::warn!("SetWinEventHook failed; reconciliation remains active");
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        });
        rx
    }
}
#[cfg(not(windows))]
mod imp {
    use anyhow::Result;
    use std::{
        collections::HashMap,
        sync::mpsc::{self, Receiver},
    };
    #[derive(Debug, Clone)]
    pub struct ProcessInfo {
        pub path: Option<String>,
        pub parent_pid: u32,
    }
    pub fn processes() -> Result<HashMap<u32, ProcessInfo>> {
        Ok(HashMap::new())
    }
    pub fn foreground_pid() -> Option<u32> {
        None
    }
    pub fn process_path(_: u32) -> Option<String> {
        None
    }
    pub fn foreground_events() -> Receiver<()> {
        mpsc::channel().1
    }
}
pub use imp::*;
