use crate::{models::ScreenshotOcrRegion, ocr, screenshot, tracking::TrackingService};
use image::DynamicImage;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::*,
            Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn start(tracker: TrackingService, busy: Arc<AtomicBool>) {
    if busy.swap(true, Ordering::AcqRel) {
        return;
    }
    std::thread::spawn(move || {
        let _guard = BusyGuard(busy);
        // Capture, positioning and mouse coordinates must all use physical pixels.
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        if let Err(message) = search_focused(&tracker) {
            let message: Vec<_> = message.encode_utf16().chain(Some(0)).collect();
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR(message.as_ptr()),
                    w!("ゲーム画面のOCR検索"),
                    MB_OK | MB_ICONINFORMATION | MB_TOPMOST,
                );
            }
        }
    });
}

fn search_focused(tracker: &TrackingService) -> Result<(), &'static str> {
    let frame = screenshot::capture_game(tracker).map_err(|error| {
        if error.to_string() == "フォアグラウンドで計測中のゲームがありません"
        {
            "フォアグラウンドで計測中のゲームがありません。"
        } else {
            "ゲーム画面を読み取れませんでした。ウィンドウ表示でお試しください。"
        }
    })?;
    let region = select_region(&frame.image, frame.origin, frame.hwnd)
        .map_err(|_| "範囲選択を開始できませんでした。ウィンドウ表示でお試しください。")?;
    let Some(region) = region else {
        return Ok(());
    };
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

#[derive(Default)]
struct Selection {
    closing: bool,
    start: Option<(i32, i32)>,
    end: (i32, i32),
    size: (i32, i32),
    pixels: Vec<u8>,
    result: Option<ScreenshotOcrRegion>,
}
impl Selection {
    fn cancel(&mut self) {
        self.closing = true;
        self.start = None;
        self.result = None;
    }
    fn point(&self, param: LPARAM) -> (i32, i32) {
        (
            (param.0 as i16 as i32).clamp(0, self.size.0),
            ((param.0 >> 16) as i16 as i32).clamp(0, self.size.1),
        )
    }
    fn rect(&self) -> Option<RECT> {
        let (x, y) = self.start?;
        Some(RECT {
            left: x.min(self.end.0),
            top: y.min(self.end.1),
            right: x.max(self.end.0),
            bottom: y.max(self.end.1),
        })
    }
    fn region(&self) -> Option<ScreenshotOcrRegion> {
        let rect = self.rect()?;
        if rect.right - rect.left < 2 || rect.bottom - rect.top < 2 {
            return None;
        }
        let x = f64::from(rect.left) / f64::from(self.size.0);
        let y = f64::from(rect.top) / f64::from(self.size.1);
        Some(ScreenshotOcrRegion {
            x,
            y,
            width: f64::from(rect.right) / f64::from(self.size.0) - x,
            height: f64::from(rect.bottom) / f64::from(self.size.1) - y,
        })
    }
}

fn select_region(
    image: &image::RgbaImage,
    origin: POINT,
    game: HWND,
) -> anyhow::Result<Option<ScreenshotOcrRegion>> {
    static CLASS: OnceLock<bool> = OnceLock::new();
    let instance = unsafe { GetModuleHandleW(None)? };
    let registered = CLASS.get_or_init(|| unsafe {
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: w!("EptOcrSelection"),
            hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
            ..Default::default()
        }) != 0
    });
    anyhow::ensure!(*registered, "OCR selection window class unavailable");
    let mut pixels = image.as_raw().clone();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let mut selection = Box::new(Selection {
        size: (image.width() as i32, image.height() as i32),
        pixels,
        ..Default::default()
    });
    // The Box stays alive until the window is destroyed. No Rust reference is held
    // across APIs that synchronously re-enter window_proc.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            w!("EptOcrSelection"),
            w!("範囲を選択してGoogle検索"),
            WS_POPUP,
            origin.x,
            origin.y,
            selection.size.0,
            selection.size.1,
            None,
            None,
            Some(instance.into()),
            Some((&mut *selection as *mut Selection).cast()),
        )?
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    if unsafe { GetForegroundWindow() } != hwnd {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        anyhow::bail!("OCR selection could not receive focus");
    }
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    // Error/quit paths must not leave a native window pointing into freed memory.
    if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
    if selection.result.is_some() && unsafe { GetForegroundWindow() }.is_invalid() {
        unsafe {
            let _ = SetForegroundWindow(game);
        }
    }
    Ok(selection.result)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // All state is confined to the overlay thread. Destruction never frees the Box;
    // select_region owns it and waits for WM_QUIT before reading the result.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Selection;
        if data.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        match message {
            WM_PAINT => {
                paint(hwnd, &*data);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_LBUTTONDOWN if !(*data).closing => {
                (*data).start = Some((*data).point(lparam));
                (*data).end = (*data).point(lparam);
                SetCapture(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_MOUSEMOVE if (*data).start.is_some() => {
                (*data).end = (*data).point(lparam);
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_LBUTTONUP if (*data).start.is_some() => {
                (*data).end = (*data).point(lparam);
                (*data).result = (*data).region();
                (*data).start = None;
                let _ = ReleaseCapture();
                if (*data).result.is_some() {
                    (*data).closing = true;
                    let _ = DestroyWindow(hwnd);
                } else {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }
            WM_KEYDOWN | WM_SYSKEYDOWN if wparam.0 == 0x1b => {
                (*data).cancel();
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_KILLFOCUS => {
                // Queue cancellation rather than recursively destroying a window
                // that may already be inside DestroyWindow's focus transition.
                if !(*data).closing {
                    (*data).cancel();
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN | WM_CLOSE => {
                (*data).cancel();
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_CAPTURECHANGED if (*data).start.is_some() => {
                (*data).start = None;
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
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
        let (width, height) = selection.size;
        let info = BITMAPINFO {
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
        StretchDIBits(
            dc,
            0,
            0,
            width,
            height,
            0,
            0,
            width,
            height,
            Some(selection.pixels.as_ptr().cast()),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if let Some(rect) = selection.rect() {
            let brush = CreateSolidBrush(COLORREF(0x00e6a030));
            FrameRect(dc, &rect, brush);
            let _ = DeleteObject(brush.into());
        }
        let mut font = LOGFONTW {
            lfHeight: -(20 * GetDpiForWindow(hwnd) as i32 / 96),
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
        let hint: Vec<_> = " ドラッグして範囲を選択 → Google検索 ／ Esc・右クリックで中止 "
            .encode_utf16()
            .collect();
        let _ = TextOutW(dc, 8, 8, &hint);
        SelectObject(dc, previous);
        let _ = DeleteObject(font.into());
        let _ = EndPaint(hwnd, &paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "interactive Windows overlay and OCR smoke test"]
    fn native_overlay_smoke() {
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/images/library.png");
        let image = image::open(path).unwrap().into_rgba8();
        let game = unsafe { GetForegroundWindow() };
        fn wait_for_hotkey() {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;
            unsafe {
                RegisterHotKey(None, 99, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 0x7a).unwrap();
            }
            println!("Press Ctrl+Shift+F11 to open the test overlay.");
            let mut message = MSG::default();
            while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                if message.message == WM_HOTKEY && message.wParam.0 == 99 {
                    break;
                }
            }
            unsafe {
                UnregisterHotKey(None, 99).unwrap();
            }
        }
        println!("Cancel the first overlay with Escape; select text in the second overlay.");
        wait_for_hotkey();
        assert!(
            select_region(&image, POINT { x: 80, y: 80 }, game)
                .unwrap()
                .is_none()
        );
        wait_for_hotkey();
        let region = select_region(&image, POINT { x: 80, y: 80 }, game)
            .unwrap()
            .unwrap();
        let result = ocr::recognize_image(DynamicImage::ImageRgba8(image), Some(region)).unwrap();
        assert!(search_url(&result.text).is_some());
        // This smoke test constructs the URL without opening a browser or sending text.
    }

    #[test]
    fn encodes_the_entire_query_and_skips_empty_text() {
        assert!(search_url(" \n\t").is_none());
        let text = "難しい漢字\n&?# + 読み方";
        let url = url::Url::parse(&search_url(text).unwrap()).unwrap();
        assert_eq!(url.host_str(), Some("www.google.com"));
        assert_eq!(url.path(), "/search");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("q".into(), text.into())]
        );
        assert!(url.fragment().is_none());
    }

    #[test]
    fn selection_is_direction_independent_and_rejects_clicks() {
        let mut selection = Selection {
            size: (1000, 500),
            start: Some((800, 400)),
            end: (100, 50),
            ..Default::default()
        };
        let region = selection.region().unwrap();
        assert_eq!((region.x, region.y), (0.1, 0.1));
        assert!((region.width - 0.7).abs() < 1e-10);
        assert!((region.height - 0.7).abs() < 1e-10);
        selection.end = (800, 400);
        assert!(selection.region().is_none());
    }

    #[test]
    fn clamps_signed_mouse_coordinates_to_the_image() {
        let selection = Selection {
            size: (100, 50),
            ..Default::default()
        };
        assert_eq!(
            selection.point(LPARAM(((-20i16 as u16 as u32) | (80u32 << 16)) as isize)),
            (0, 50)
        );
    }

    #[test]
    fn cancellation_invalidates_a_draft_before_queued_mouse_up() {
        let mut selection = Selection {
            start: Some((0, 0)),
            end: (100, 100),
            size: (100, 100),
            ..Default::default()
        };
        selection.result = selection.region();
        selection.cancel();
        assert!(selection.closing);
        assert!(selection.start.is_none());
        assert!(selection.region().is_none());
        assert!(selection.result.is_none());
    }
}
