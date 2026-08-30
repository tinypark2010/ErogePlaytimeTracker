use crate::{database::Database, tracking::TrackingService};
use chrono::Utc;
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct ScreenshotService {
    requests: Sender<HotkeyRequest>,
}

enum HotkeyRequest {
    Check(String, Sender<Result<(), String>>),
    Update(String, Sender<Result<(), String>>),
}

impl ScreenshotService {
    pub fn start(
        app: AppHandle,
        db: Database,
        tracker: TrackingService,
        root: PathBuf,
        hotkey: String,
    ) -> Self {
        let (requests, receiver) = mpsc::channel::<HotkeyRequest>();
        thread::spawn(move || {
            #[cfg(windows)]
            {
                use windows::Win32::UI::WindowsAndMessaging::{
                    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_HOTKEY,
                };
                let mut current = hotkey;
                let mut registered = if current.trim().is_empty() {
                    false
                } else {
                    match register_hotkey(&current, 1) {
                        Ok(()) => true,
                        Err(e) => {
                            log::error!("screenshot hotkey registration failed: {e:#}");
                            let _ = app.emit(
                                "screenshot-error",
                                "スクリーンショットキーを登録できませんでした。設定を確認してください。",
                            );
                            false
                        }
                    }
                };
                loop {
                    while let Ok(request) = receiver.try_recv() {
                        match request {
                            HotkeyRequest::Check(candidate, response) => {
                                let result = if (candidate == current && registered)
                                    || candidate.trim().is_empty()
                                {
                                    Ok(())
                                } else {
                                    register_hotkey(&candidate, 2).map(|()| unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(None, 2);
                                    })
                                };
                                let _ = response.send(result.map_err(|e| e.to_string()));
                            }
                            HotkeyRequest::Update(candidate, response) => {
                                if candidate == current
                                    && (registered || candidate.trim().is_empty())
                                {
                                    let _ = response.send(Ok(()));
                                    continue;
                                }
                                if registered {
                                    unsafe {
                                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(None, 1);
                                    }
                                }
                                let result = if candidate.trim().is_empty() {
                                    Ok(())
                                } else {
                                    register_hotkey(&candidate, 1)
                                };
                                match result {
                                    Ok(()) => {
                                        current = candidate;
                                        registered = !current.trim().is_empty();
                                        let _ = response.send(Ok(()));
                                    }
                                    Err(e) => {
                                        registered = !current.trim().is_empty()
                                            && register_hotkey(&current, 1).is_ok();
                                        let _ = response.send(Err(e.to_string()));
                                    }
                                }
                            }
                        }
                    }
                    let mut message = MSG::default();
                    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
                        if message.message == WM_HOTKEY {
                            if let Err(e) = capture_focused(&app, &db, &tracker, &root) {
                                log::warn!("screenshot capture skipped or failed: {e:#}");
                                let _ = app.emit("screenshot-error", capture_error_message(&e));
                            }
                        } else {
                            unsafe {
                                let _ = TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(25));
                }
            }
            #[cfg(not(windows))]
            let _ = (app, db, tracker, root, hotkey, receiver);
        });
        Self { requests }
    }

    pub fn set_hotkey(&self, hotkey: String) -> anyhow::Result<()> {
        validate_hotkey(&hotkey)?;
        self.request(|response| HotkeyRequest::Update(hotkey, response))
    }

    pub fn check_hotkey(&self, hotkey: String) -> anyhow::Result<()> {
        validate_hotkey(&hotkey)?;
        self.request(|response| HotkeyRequest::Check(hotkey, response))
    }

    fn request(
        &self,
        make: impl FnOnce(Sender<Result<(), String>>) -> HotkeyRequest,
    ) -> anyhow::Result<()> {
        let (response, receiver) = mpsc::channel();
        self.requests.send(make(response))?;
        receiver
            .recv_timeout(Duration::from_secs(2))?
            .map_err(anyhow::Error::msg)
    }
}

fn capture_error_message(error: &anyhow::Error) -> &'static str {
    if error.to_string() == "フォアグラウンドで計測中のゲームがありません" {
        "フォアグラウンドで計測中のゲームがありません。"
    } else {
        "スクリーンショットを保存できませんでした。しばらくしてからもう一度お試しください。"
    }
}

#[cfg(windows)]
fn register_hotkey(value: &str, id: i32) -> anyhow::Result<()> {
    let (mods, key) = parse_hotkey(value)?;
    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey(None, id, mods, key) }
        .map_err(|_| anyhow::anyhow!("このキーは別のアプリで使用されています"))
}

pub fn validate_hotkey(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        parse_hotkey(value).map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Ok(())
    }
}

#[cfg(windows)]
fn parse_hotkey(
    value: &str,
) -> anyhow::Result<(
    windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS,
    u32,
)> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    };
    let parts: Vec<_> = value
        .split('+')
        .map(|part| part.trim().to_ascii_uppercase())
        .filter(|part| !part.is_empty())
        .collect();
    anyhow::ensure!(
        !parts.is_empty(),
        "スクリーンショットキーを入力してください"
    );
    let mut mods = MOD_NOREPEAT;
    for modifier in &parts[..parts.len() - 1] {
        mods |= match modifier.as_str() {
            "CTRL" | "CONTROL" => MOD_CONTROL,
            "ALT" => MOD_ALT,
            "SHIFT" => MOD_SHIFT,
            "WIN" | "WINDOWS" => MOD_WIN,
            _ => anyhow::bail!("未対応の修飾キーです: {modifier}"),
        };
    }
    let key_name = parts.last().unwrap();
    let function_key = key_name
        .strip_prefix('F')
        .and_then(|number| number.parse::<u32>().ok())
        .filter(|number| (1..=24).contains(number));
    let key = match key_name.as_str() {
        "PRINTSCREEN" | "PRTSC" => 0x2c,
        "INSERT" => 0x2d,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        name if name.len() == 1 && name.as_bytes()[0].is_ascii_alphanumeric() => {
            name.as_bytes()[0] as u32
        }
        _ if function_key.is_some() => 0x70 + function_key.unwrap() - 1,
        _ => anyhow::bail!("未対応のキーです: {key_name}"),
    };
    Ok((HOT_KEY_MODIFIERS(mods.0), key))
}

#[cfg(windows)]
fn capture_focused(
    app: &AppHandle,
    db: &Database,
    tracker: &TrackingService,
    root: &Path,
) -> anyhow::Result<()> {
    let (game_id, session_id) = tracker
        .focused_game()
        .ok_or_else(|| anyhow::anyhow!("フォアグラウンドで計測中のゲームがありません"))?;
    let hwnd = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    anyhow::ensure!(
        !hwnd.is_invalid(),
        "フォアグラウンドウィンドウを取得できません"
    );
    let (pixels, width, height) = capture_window(hwnd)?;
    let directory = root.join(game_id.to_string());
    std::fs::create_dir_all(&directory)?;
    let captured_at = Utc::now();
    let path = directory.join(format!("{}.png", captured_at.format("%Y%m%d-%H%M%S-%3f")));
    write_png(&path, &pixels, width, height)?;
    if let Err(e) = db.add_screenshot(
        game_id,
        Some(session_id),
        &path.to_string_lossy(),
        &captured_at.to_rfc3339(),
        width as i64,
        height as i64,
    ) {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    let _ = app.emit("screenshot-captured", game_id);
    Ok(())
}

#[cfg(windows)]
fn capture_window(hwnd: windows::Win32::Foundation::HWND) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    use windows::Win32::{
        Foundation::RECT, Graphics::Gdi::*, UI::WindowsAndMessaging::GetClientRect,
    };
    let mut rect = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut rect)?;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    anyhow::ensure!(width > 0 && height > 0, "ゲーム画面のサイズが不正です");
    // GetDC returns a device context whose origin is the client area's top-left.
    // This deliberately excludes the title bar, resize border and DWM shadow.
    let source = unsafe { GetDC(Some(hwnd)) };
    anyhow::ensure!(!source.is_invalid(), "ゲーム画面の描画領域を取得できません");
    let memory = unsafe { CreateCompatibleDC(Some(source)) };
    let bitmap = unsafe { CreateCompatibleBitmap(source, width, height) };
    anyhow::ensure!(
        !memory.is_invalid() && !bitmap.is_invalid(),
        "撮影用バッファを作成できません"
    );
    let previous = unsafe { SelectObject(memory, bitmap.into()) };
    // Pixels outside the virtual desktop may no longer be updated in the
    // compositor surface. Ask the window to repaint itself in that case.
    // Some games do not support PrintWindow, so retain BitBlt as a fallback.
    let printed = !client_is_fully_visible(hwnd, width, height)
        && unsafe {
            windows::Win32::Storage::Xps::PrintWindow(
                hwnd,
                memory,
                windows::Win32::Storage::Xps::PW_CLIENTONLY,
            )
            .as_bool()
        };
    let copied = printed
        || unsafe {
            BitBlt(
                memory,
                0,
                0,
                width,
                height,
                Some(source),
                0,
                0,
                SRCCOPY | CAPTUREBLT,
            )
        }
        .is_ok();
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bgra = vec![0u8; width as usize * height as usize * 4];
    let lines = if copied {
        unsafe {
            GetDIBits(
                memory,
                bitmap,
                0,
                height as u32,
                Some(bgra.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        }
    } else {
        0
    };
    unsafe {
        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(Some(hwnd), source);
    }
    anyhow::ensure!(lines == height, "ゲーム画面を読み取れません");
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    Ok((bgra, width as u32, height as u32))
}

#[cfg(windows)]
fn client_is_fully_visible(
    hwnd: windows::Win32::Foundation::HWND,
    width: i32,
    height: i32,
) -> bool {
    use windows::Win32::{
        Foundation::POINT,
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        },
    };
    let mut origin = POINT::default();
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        return false;
    }
    let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let virtual_right = virtual_left + unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let virtual_bottom = virtual_top + unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    origin.x >= virtual_left
        && origin.y >= virtual_top
        && origin.x + width <= virtual_right
        && origin.y + height <= virtual_bottom
}

#[cfg(windows)]
fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

#[cfg(test)]
mod error_message_tests {
    use super::*;

    #[test]
    fn exposes_only_expected_capture_errors() {
        assert_eq!(
            capture_error_message(&anyhow::anyhow!(
                "フォアグラウンドで計測中のゲームがありません"
            )),
            "フォアグラウンドで計測中のゲームがありません。"
        );
        let message = capture_error_message(&anyhow::anyhow!("private capture detail"));
        assert!(!message.contains("private capture detail"));
    }
}
