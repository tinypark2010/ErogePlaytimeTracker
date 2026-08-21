#[cfg(windows)]
mod imp {
    use anyhow::Result;
    use std::{
        collections::{HashMap, HashSet},
        sync::{
            OnceLock,
            mpsc::{self, Receiver, Sender},
        },
        thread,
    };
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE, HWND, LPARAM},
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                    TH32CS_SNAPPROCESS,
                },
                Threading::{
                    INFINITE, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                    PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW, WaitForSingleObject,
                },
            },
            UI::{
                Accessibility::{HWINEVENTHOOK, SetWinEventHook},
                WindowsAndMessaging::{
                    EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE, EVENT_SYSTEM_FOREGROUND, EnumWindows,
                    GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, IsWindowVisible,
                    MSG, OBJID_WINDOW, WINEVENT_OUTOFCONTEXT,
                },
            },
        },
        core::BOOL,
    };
    #[derive(Debug, Clone)]
    pub struct ProcessInfo {
        pub path: Option<String>,
        pub parent_pid: u32,
    }
    #[derive(Debug, Clone, Copy)]
    pub enum TrackingEvent {
        ForegroundChanged,
        WindowChanged,
        ProcessExited(u32),
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
    unsafe extern "system" fn collect_visible_window(hwnd: HWND, param: LPARAM) -> BOOL {
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            if pid != 0 {
                let pids = unsafe { &mut *(param.0 as *mut HashSet<u32>) };
                pids.insert(pid);
            }
        }
        true.into()
    }
    pub fn visible_window_pids() -> Result<HashSet<u32>> {
        let mut pids = HashSet::new();
        unsafe {
            EnumWindows(
                Some(collect_visible_window),
                LPARAM(&mut pids as *mut HashSet<u32> as isize),
            )?;
        }
        Ok(pids)
    }
    type GetHwnd = HWND;
    static EVENTS: OnceLock<Sender<TrackingEvent>> = OnceLock::new();
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
            let _ = tx.send(TrackingEvent::ForegroundChanged);
        }
    }
    unsafe extern "system" fn window_event(
        _: HWINEVENTHOOK,
        _: u32,
        _: HWND,
        object: i32,
        child: i32,
        _: u32,
        _: u32,
    ) {
        if object == OBJID_WINDOW.0
            && child == 0
            && let Some(tx) = EVENTS.get()
        {
            let _ = tx.send(TrackingEvent::WindowChanged);
        }
    }
    pub fn tracking_events() -> Receiver<TrackingEvent> {
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
            let window_hook = SetWinEventHook(
                EVENT_OBJECT_CREATE,
                EVENT_OBJECT_HIDE,
                None,
                Some(window_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if window_hook.0.is_null() {
                log::warn!("window lifecycle hook failed; reconciliation remains active");
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        });
        rx
    }
    pub fn watch_process_exit(pid: u32) -> bool {
        let Some(events) = EVENTS.get().cloned() else {
            return false;
        };
        let Ok(handle) = (unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }) else {
            return false;
        };
        // HANDLE is not Send in windows-rs; transfer only its integer value and
        // reconstruct ownership inside the waiter thread.
        let handle_value = handle.0 as usize;
        thread::spawn(move || unsafe {
            let handle = HANDLE(handle_value as *mut core::ffi::c_void);
            WaitForSingleObject(handle, INFINITE);
            let _ = CloseHandle(handle);
            let _ = events.send(TrackingEvent::ProcessExited(pid));
        });
        true
    }
}
#[cfg(not(windows))]
mod imp {
    use anyhow::Result;
    use std::{
        collections::{HashMap, HashSet},
        sync::mpsc::{self, Receiver},
    };
    #[derive(Debug, Clone)]
    pub struct ProcessInfo {
        pub path: Option<String>,
        pub parent_pid: u32,
    }
    #[derive(Debug, Clone, Copy)]
    pub enum TrackingEvent {
        ForegroundChanged,
        WindowChanged,
        ProcessExited(u32),
    }
    pub fn processes() -> Result<HashMap<u32, ProcessInfo>> {
        Ok(HashMap::new())
    }
    pub fn foreground_pid() -> Option<u32> {
        None
    }
    pub fn visible_window_pids() -> Result<HashSet<u32>> {
        Ok(HashSet::new())
    }
    pub fn process_path(_: u32) -> Option<String> {
        None
    }
    pub fn tracking_events() -> Receiver<TrackingEvent> {
        mpsc::channel().1
    }
    pub fn watch_process_exit(_: u32) -> bool {
        false
    }
}
pub use imp::*;
