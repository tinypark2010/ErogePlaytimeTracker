mod input;
#[cfg(test)]
mod native_tests;

use crate::{models::ScreenshotOcrRegion, ocr, screenshot, tracking::TrackingService};
use image::DynamicImage;
use input::{Input, Selection};
use std::{
    cell::Cell,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{Dwm::DwmFlush, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Accessibility::*, HiDpi::*, Input::KeyboardAndMouse::VK_ESCAPE, WindowsAndMessaging::*,
        },
    },
    core::w,
};

const WM_SELECTION_INPUT: u32 = WM_APP + 1;
thread_local! { static ACTIVE: Cell<*mut Overlay> = const { Cell::new(std::ptr::null_mut()) }; }

struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn start(app: AppHandle, tracker: TrackingService, busy: Arc<AtomicBool>) {
    if busy.swap(true, Ordering::AcqRel) {
        return;
    }
    std::thread::spawn(move || {
        let _guard = BusyGuard(busy);
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        if let Err(message) = search_focused(&tracker) {
            // A modal message box would steal focus from an exclusive-fullscreen game too.
            let _ = app.emit("game-ocr-error", message);
        }
    });
}

fn search_focused(tracker: &TrackingService) -> Result<(), &'static str> {
    let target = screenshot::focused_target(tracker)
        .map_err(|_| "前面の計測中ゲームを確認できませんでした。ゲーム画面でお試しください。")?;
    let region = select_region(target.hwnd, target.bounds, target.size)
        .map_err(|_| "範囲選択を開始できませんでした。ウィンドウ表示でお試しください。")?;
    let Some(region) = region else {
        return Ok(());
    };
    // Capture only after removing the outline, and never use another game's frame.
    if !target.is_current() {
        return Ok(());
    }
    let frame = screenshot::capture_game(tracker)
        .map_err(|_| "ゲーム画面を読み取れませんでした。ウィンドウ表示でお試しください。")?;
    if frame.target != target {
        return Ok(());
    }
    let result = ocr::recognize_image(DynamicImage::ImageRgba8(frame.image), Some(region))
        .map_err(|e| e.user_message())?;
    let url = search_url(&result.text)
        .ok_or("文字を読み取れませんでした。範囲を変えてお試しください。")?;
    crate::commands::open_external_url(url).map_err(|_| "既定のブラウザを開けませんでした。")
}

fn search_url(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }
    let mut url = url::Url::parse("https://www.google.com/search").expect("static search URL");
    url.query_pairs_mut().append_pair("q", text);
    Some(url.into())
}

struct Overlay {
    hwnd: HWND,
    game: HWND,
    selection: Selection,
    size: (u32, u32),
    hidden_at: Option<Instant>,
}

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}

// All callbacks run on this selection thread, which keeps pumping messages.
// The guards are dropped before capture/OCR so expensive work cannot stall hooks.
#[derive(Default)]
struct Hooks {
    mouse: HHOOK,
    keyboard: HHOOK,
    foreground: HWINEVENTHOOK,
}
impl Hooks {
    unsafe fn install(data: *mut Overlay) -> windows::core::Result<Self> {
        let mut hooks = Self::default();
        unsafe {
            ACTIVE.set(data);
            let instance = GetModuleHandleW(None)?;
            hooks.mouse =
                SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), Some(instance.into()), 0)?;
            hooks.keyboard = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook),
                Some(instance.into()),
                0,
            )?;
            hooks.foreground = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_changed),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hooks.foreground.is_invalid() {
                return Err(windows::core::Error::from_win32());
            }
        }
        Ok(hooks)
    }
}
impl Drop for Hooks {
    fn drop(&mut self) {
        unsafe {
            ACTIVE.set(std::ptr::null_mut());
            if !self.foreground.is_invalid() {
                let _ = UnhookWinEvent(self.foreground);
            }
            if !self.keyboard.is_invalid() {
                let _ = UnhookWindowsHookEx(self.keyboard);
            }
            if !self.mouse.is_invalid() {
                let _ = UnhookWindowsHookEx(self.mouse);
            }
        }
    }
}

fn route_input(input: Input) -> bool {
    let foreground = unsafe { GetForegroundWindow() };
    ACTIVE.with(|slot| unsafe {
        let data = slot.get();
        if data.is_null() {
            return false;
        }
        if foreground != (*data).game {
            (*data).selection.cancel();
        }
        let consumed = (*data).selection.handle(input);
        // Mouse moves are painted by the timer, avoiding a queue per raw event.
        if !matches!(input, Input::Move(_)) || (*data).selection.closing {
            let _ = PostMessageW(Some((*data).hwnd), WM_SELECTION_INPUT, WPARAM(0), LPARAM(0));
        }
        consumed
    })
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code == HC_ACTION as i32 {
            let point = (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt;
            let input = match wparam.0 as u32 {
                WM_MOUSEMOVE => Some(Input::Move(point)),
                WM_LBUTTONDOWN => Some(Input::LeftDown(point)),
                WM_LBUTTONUP => Some(Input::LeftUp(point)),
                WM_RBUTTONDOWN => Some(Input::RightDown(point)),
                WM_RBUTTONUP => Some(Input::RightUp),
                _ => None,
            };
            if input.is_some_and(route_input) {
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        // Inspect only Esc. Other keys (including Alt+Tab and hotkey releases) pass through.
        if code == HC_ACTION as i32
            && (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode == u32::from(VK_ESCAPE.0)
        {
            let input = match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(Input::EscapeDown),
                WM_KEYUP | WM_SYSKEYUP => Some(Input::EscapeUp),
                _ => None,
            };
            if input.is_some_and(route_input) {
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }
}

unsafe extern "system" fn foreground_changed(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    ACTIVE.with(|slot| unsafe {
        let data = slot.get();
        if !data.is_null() && hwnd != (*data).game {
            (*data).selection.cancel();
            let _ = PostMessageW(Some((*data).hwnd), WM_SELECTION_INPUT, WPARAM(0), LPARAM(0));
        }
    });
}

fn select_region(
    game: HWND,
    bounds: RECT,
    size: (u32, u32),
) -> anyhow::Result<Option<ScreenshotOcrRegion>> {
    let result = unsafe {
        static CLASS: OnceLock<u16> = OnceLock::new();
        let instance = GetModuleHandleW(None)?;
        let class = w!("EptOcrSelection");
        let atom = CLASS.get_or_init(|| {
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: class,
                ..Default::default()
            })
        });
        anyhow::ensure!(*atom != 0, "selection class registration failed");
        let mut data = Box::new(Overlay {
            hwnd: HWND::default(),
            game,
            selection: Selection::new(bounds, size),
            size,
            hidden_at: None,
        });
        let ptr = &mut *data as *mut Overlay;
        let window = Window(CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class,
            w!("OCR"),
            WS_POPUP,
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            None,
            None,
            Some(instance.into()),
            Some(ptr.cast()),
        )?);
        (*ptr).hwnd = window.0;
        SetLayeredWindowAttributes(window.0, COLORREF(0), 255, LWA_COLORKEY)?;
        let _hooks = Hooks::install(ptr)?;
        anyhow::ensure!(
            SetTimer(Some(window.0), 1, 20, None) != 0,
            "selection timer failed"
        );
        if game != GetForegroundWindow() {
            return Ok(None);
        }
        // Never activate, capture the mouse, restore focus, or change display mode.
        SetWindowPos(
            window.0,
            Some(HWND_TOPMOST),
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
        refresh_overlay(window.0, ptr);
        let mut message = MSG::default();
        loop {
            let received = GetMessageW(&mut message, None, 0, 0).0;
            anyhow::ensure!(received != -1, "selection message loop failed");
            if received == 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        (*ptr).selection.result.take()
    };
    // Flush removal before CAPTUREBLT, otherwise the outline could enter the OCR crop.
    unsafe {
        let _ = DwmFlush();
    }
    Ok(result)
}

unsafe fn refresh_overlay(hwnd: HWND, data: *mut Overlay) {
    unsafe {
        if GetForegroundWindow() != (*data).game
            || screenshot::client_geometry((*data).game).ok()
                != Some(((*data).selection.bounds, (*data).size))
        {
            (*data).selection.cancel();
        }
        if (*data).selection.closing {
            if (*data).hidden_at.is_none() {
                (*data).hidden_at = Some(Instant::now());
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE | SWP_HIDEWINDOW,
                );
            }
            // Drain the intercepted press/release pairs even though the frame is already hidden.
            // A desktop switch or lost hook must not leave a busy selection thread forever.
            if (*data).selection.drained()
                || (*data)
                    .hidden_at
                    .is_some_and(|at| at.elapsed() >= Duration::from_secs(10))
            {
                let _ = DestroyWindow(hwnd);
            }
        } else {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Overlay;
        if data.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        match message {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
            WM_TIMER | WM_SELECTION_INPUT => {
                refresh_overlay(hwnd, data);
                LRESULT(0)
            }
            WM_DISPLAYCHANGE | WM_DPICHANGED | WM_CLOSE => {
                (*data).selection.cancel();
                refresh_overlay(hwnd, data);
                LRESULT(0)
            }
            WM_PAINT => {
                // Native painting can dispatch hooks; do not borrow live state across it.
                let selection = (*data).selection.clone();
                paint(hwnd, &selection);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

fn paint(hwnd: HWND, selection: &Selection) {
    unsafe {
        let mut paint = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut paint);
        let mut view = RECT {
            right: selection.bounds.right - selection.bounds.left,
            bottom: selection.bounds.bottom - selection.bounds.top,
            ..Default::default()
        };
        // Black is the transparent color key, not a replacement game image.
        FillRect(dc, &view, HBRUSH(GetStockObject(BLACK_BRUSH).0));
        let scale = (GetDpiForWindow(hwnd) as i32).max(96);
        if let Some(rect) = selection.rect() {
            let previous_brush = SelectObject(dc, GetStockObject(HOLLOW_BRUSH));
            for (width, color) in [(5, 0x00202020), (2, 0x00e6a030)] {
                let pen = CreatePen(PS_SOLID, width * scale / 96, COLORREF(color));
                let previous_pen = SelectObject(dc, pen.into());
                let _ = Rectangle(
                    dc,
                    rect.left - selection.bounds.left,
                    rect.top - selection.bounds.top,
                    rect.right - selection.bounds.left,
                    rect.bottom - selection.bounds.top,
                );
                SelectObject(dc, previous_pen);
                let _ = DeleteObject(pen.into());
            }
            SelectObject(dc, previous_brush);
        }
        let mut font = LOGFONTW {
            lfHeight: -(18 * scale / 96),
            ..Default::default()
        };
        for (slot, character) in font
            .lfFaceName
            .iter_mut()
            .zip("Yu Gothic UI".encode_utf16())
        {
            *slot = character;
        }
        let font = CreateFontIndirectW(&font);
        let previous = SelectObject(dc, font.into());
        SetBkColor(dc, COLORREF(0x00202020));
        SetTextColor(dc, COLORREF(0x00ffffff));
        let mut hint: Vec<_> = " ドラッグして範囲を選択 → Google検索 ／ Esc・右クリックで中止 "
            .encode_utf16()
            .collect();
        view.left = 8;
        view.top = 8;
        view.right -= 8;
        DrawTextW(
            dc,
            &mut hint,
            &mut view,
            DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        SelectObject(dc, previous);
        let _ = DeleteObject(font.into());
        let _ = EndPaint(hwnd, &paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_the_entire_query_and_skips_empty_text() {
        assert!(search_url(" \n\t").is_none());
        let text = "漢字 &意味? #1\n二行目 + 読み方";
        let url = url::Url::parse(&search_url(text).unwrap()).unwrap();
        assert_eq!(url.host_str(), Some("www.google.com"));
        assert_eq!(url.path(), "/search");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("q".into(), text.into())]
        );
        assert_eq!(url.fragment(), None);
    }
}
