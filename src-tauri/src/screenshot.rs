use crate::{database::Database, tracking::TrackingService};
use chrono::Utc;
use std::{fs::File, io::BufWriter, path::Path};
use tauri::{AppHandle, Emitter};

pub(crate) fn capture_error_message(error: &anyhow::Error) -> &'static str {
    if error.to_string() == "フォアグラウンドで計測中のゲームがありません" {
        "フォアグラウンドで計測中のゲームがありません。"
    } else {
        "スクリーンショットを保存できませんでした。しばらくしてからもう一度お試しください。"
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameCaptureTarget {
    pub hwnd: windows::Win32::Foundation::HWND,
    pub bounds: windows::Win32::Foundation::RECT,
    pub size: (u32, u32),
    game_id: i64,
    session_id: i64,
}

#[cfg(windows)]
impl GameCaptureTarget {
    pub fn is_current(&self) -> bool {
        self.hwnd == unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }
            && client_geometry(self.hwnd).is_ok_and(|geometry| geometry == (self.bounds, self.size))
    }
}

#[cfg(windows)]
pub(crate) fn client_geometry(
    hwnd: windows::Win32::Foundation::HWND,
) -> anyhow::Result<(windows::Win32::Foundation::RECT, (u32, u32))> {
    use windows::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::GetClientRect,
    };
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect)? };
    let size = (rect.right - rect.left, rect.bottom - rect.top);
    anyhow::ensure!(size.0 > 0 && size.1 > 0, "ゲーム画面のサイズが不正です");
    let mut start = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut end = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    anyhow::ensure!(
        unsafe { ClientToScreen(hwnd, &mut start) }.as_bool()
            && unsafe { ClientToScreen(hwnd, &mut end) }.as_bool(),
        "ゲーム画面の位置を取得できません"
    );
    Ok((
        RECT {
            left: start.x,
            top: start.y,
            right: end.x,
            bottom: end.y,
        },
        (size.0 as u32, size.1 as u32),
    ))
}

#[cfg(windows)]
pub fn focused_target(tracker: &TrackingService) -> anyhow::Result<GameCaptureTarget> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let hwnd = unsafe { GetForegroundWindow() };
    let (game_id, session_id) = tracker
        .focused_game()
        .ok_or_else(|| anyhow::anyhow!("フォアグラウンドで計測中のゲームがありません"))?;
    anyhow::ensure!(
        !hwnd.is_invalid() && hwnd == unsafe { GetForegroundWindow() },
        "フォアグラウンドで計測中のゲームがありません"
    );
    let (bounds, size) = client_geometry(hwnd)?;
    let target = GameCaptureTarget {
        hwnd,
        bounds,
        size,
        game_id,
        session_id,
    };
    anyhow::ensure!(
        target.is_current() && tracker.focused_game() == Some((game_id, session_id)),
        "取得中にゲーム画面が切り替わりました"
    );
    Ok(target)
}

#[cfg(windows)]
pub struct CapturedGame {
    pub target: GameCaptureTarget,
    pub image: image::RgbaImage,
}

#[cfg(windows)]
pub fn capture_game(tracker: &TrackingService) -> anyhow::Result<CapturedGame> {
    let target = focused_target(tracker)?;
    let (pixels, width, height) = capture_window(target.hwnd)?;
    anyhow::ensure!(
        focused_target(tracker)? == target && target.size == (width, height),
        "取得中にゲーム画面が切り替わりました"
    );
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow::anyhow!("ゲーム画面を読み取れません"))?;
    Ok(CapturedGame { target, image })
}

#[cfg(windows)]
pub fn capture_focused(
    app: &AppHandle,
    db: &Database,
    tracker: &TrackingService,
    root: &Path,
) -> anyhow::Result<()> {
    let CapturedGame { target, image } = capture_game(tracker)?;
    let GameCaptureTarget {
        game_id,
        session_id,
        ..
    } = target;
    let (width, height) = image.dimensions();
    let directory = root.join(game_id.to_string());
    std::fs::create_dir_all(&directory)?;
    let captured_at = Utc::now();
    let path = directory.join(format!("{}.png", captured_at.format("%Y%m%d-%H%M%S-%3f")));
    write_png(&path, image.as_raw(), width, height)?;
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
