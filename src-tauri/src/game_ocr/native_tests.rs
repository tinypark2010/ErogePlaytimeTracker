//! Explicit desktop smoke test. Uses only a synthetic game window, never searches Google.
use super::*;
use windows::{
    Win32::{
        Foundation::POINT, System::Threading::GetCurrentThreadId, UI::Input::KeyboardAndMouse::*,
    },
    core::BOOL,
};

const WM_COLOR: u32 = WM_APP + 50;
const WM_QUERY_SELECTION: u32 = WM_APP + 51;
#[derive(Default)]
struct Counts {
    selection_input: u32,
    other_input: u32,
    lost_focus: u32,
    color: COLORREF,
}

unsafe extern "system" fn fixture_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (*(lparam.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
            );
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Counts;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        match message {
            WM_QUERY_SELECTION => ACTIVE.with(|slot| {
                let data = slot.get();
                if data.is_null() {
                    return LRESULT(-2);
                }
                let Some(rect) = (*data).selection.rect() else {
                    return LRESULT(-1);
                };
                LRESULT(match wparam.0 {
                    0 => rect.left,
                    1 => rect.top,
                    2 => rect.right,
                    _ => rect.bottom,
                } as isize)
            }),
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP => {
                (*ptr).selection_input += 1;
                LRESULT(0)
            }
            WM_KEYDOWN | WM_KEYUP => {
                if wparam.0 == VK_ESCAPE.0 as usize {
                    (*ptr).selection_input += 1;
                } else {
                    (*ptr).other_input += 1;
                }
                LRESULT(0)
            }
            WM_KILLFOCUS => {
                (*ptr).lost_focus += 1;
                LRESULT(0)
            }
            WM_COLOR => {
                (*ptr).color = COLORREF(wparam.0 as u32);
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_PAINT => {
                let color = (*ptr).color;
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let brush = CreateSolidBrush(color);
                FillRect(dc, &rect, brush);
                let _ = DeleteObject(brush.into());
                let _ = EndPaint(hwnd, &ps);
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

fn overlay(thread: u32) -> Option<HWND> {
    unsafe extern "system" fn visit(hwnd: HWND, param: LPARAM) -> BOOL {
        unsafe {
            let mut name = [0; 64];
            let length = GetClassNameW(hwnd, &mut name);
            if String::from_utf16_lossy(&name[..length as usize]) == "EptOcrSelection" {
                *(param.0 as *mut Option<HWND>) = Some(hwnd);
                return false.into();
            }
            true.into()
        }
    }
    let mut found = None;
    unsafe {
        let _ = EnumThreadWindows(
            thread,
            Some(visit),
            LPARAM((&mut found as *mut Option<HWND>) as isize),
        );
    }
    found
}

fn wait_for(label: &str, mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(
            Instant::now() < until,
            "native selection smoke test timed out: {label}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn move_to(x: i32, y: i32) {
    unsafe {
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        let event = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: (x - GetSystemMetrics(SM_XVIRTUALSCREEN)) * 65536 / width
                        + 65536 / (2 * width),
                    dy: (y - GetSystemMetrics(SM_YVIRTUALSCREEN)) * 65536 / height
                        + 65536 / (2 * height),
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    ..Default::default()
                },
            },
        };
        assert_eq!(SendInput(&[event], std::mem::size_of::<INPUT>() as i32), 1);
    }
}
fn mouse(flags: MOUSE_EVENT_FLAGS) {
    let event = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    assert_eq!(
        unsafe { SendInput(&[event], std::mem::size_of::<INPUT>() as i32) },
        1
    );
}
fn key(key: VIRTUAL_KEY, up: bool) {
    let event = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    };
    assert_eq!(
        unsafe { SendInput(&[event], std::mem::size_of::<INPUT>() as i32) },
        1
    );
}
fn pixel(x: i32, y: i32) -> COLORREF {
    unsafe {
        let _ = DwmFlush();
        let dc = GetDC(None);
        let color = GetPixel(dc, x, y);
        ReleaseDC(None, dc);
        color
    }
}
fn pump() {
    let until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < until {
        let mut message = MSG::default();
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn focus_fixture(game: HWND) {
    unsafe {
        // Receiving our registered hotkey grants normal foreground activation rights.
        RegisterHotKey(
            None,
            99,
            MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT,
            VK_F11.0 as u32,
        )
        .unwrap();
        let timeout = SetTimer(None, 99, 5000, None);
        assert_ne!(timeout, 0);
        let sender = std::thread::spawn(|| {
            key(VK_CONTROL, false);
            key(VK_SHIFT, false);
            key(VK_F11, false);
            key(VK_F11, true);
            key(VK_SHIFT, true);
            key(VK_CONTROL, true);
        });
        let mut message = MSG::default();
        let mut activated = false;
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            if message.message == WM_HOTKEY && message.wParam.0 == 99 {
                activated = SetForegroundWindow(game).as_bool();
                break;
            }
            if message.message == WM_TIMER && message.wParam.0 == timeout {
                break;
            }
            DispatchMessageW(&message);
        }
        let _ = KillTimer(None, timeout);
        UnregisterHotKey(None, 99).unwrap();
        sender.join().unwrap();
        pump();
        assert!(activated);
        assert_eq!(GetForegroundWindow(), game);
    }
}

#[test]
#[ignore = "requires an interactive Windows desktop; temporarily focuses a synthetic game and sends mouse/Esc input"]
fn native_live_selection_preserves_focus_and_drains_input() {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(None).unwrap();
        let class = w!("EptOcrSyntheticGame");
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(fixture_proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            }),
            0
        );
        let mut counts = Box::<Counts>::default();
        let game = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("OCR selection smoke test"),
                WS_POPUP,
                100,
                100,
                800,
                600,
                None,
                None,
                Some(instance.into()),
                Some((&mut *counts as *mut Counts).cast()),
            )
            .unwrap(),
        );
        SetWindowPos(
            game.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .unwrap();
        pump();
        focus_fixture(game.0);
        counts.selection_input = 0;
        counts.other_input = 0;
        let mut other_counts = Box::<Counts>::default();
        let other = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("OCR focus-loss fixture"),
                WS_POPUP,
                1000,
                100,
                200,
                200,
                None,
                None,
                Some(instance.into()),
                Some((&mut *other_counts as *mut Counts).cast()),
            )
            .unwrap(),
        );
        let thread = GetCurrentThreadId();
        for stage in 0..6 {
            let (bounds, size) = screenshot::client_geometry(game.0).unwrap();
            let game_value = game.0.0 as usize;
            let other_value = other.0.0 as usize;
            let driver = std::thread::spawn(move || {
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                let game = HWND(game_value as *mut _);
                let other = HWND(other_value as *mut _);
                let outcome = std::panic::catch_unwind(|| {
                    wait_for("visible overlay", || {
                        overlay(thread).is_some_and(|hwnd| IsWindowVisible(hwnd).as_bool())
                    });
                    let frame = overlay(thread).unwrap();
                    assert_eq!(GetForegroundWindow(), game);
                    let start = POINT {
                        x: bounds.left + 200,
                        y: bounds.top + 150,
                    };
                    move_to(start.x, start.y);
                    wait_for("cursor reaches drag start", || {
                        let mut cursor = POINT::default();
                        GetCursorPos(&mut cursor).is_ok() && cursor == start
                    });
                    if stage == 1 {
                        mouse(MOUSEEVENTF_RIGHTDOWN);
                        wait_for("right-click hides outline", || {
                            !IsWindowVisible(frame).as_bool()
                        });
                        assert!(
                            IsWindow(Some(frame)).as_bool(),
                            "wait for owned right-button release"
                        );
                        mouse(MOUSEEVENTF_RIGHTUP);
                    } else {
                        mouse(MOUSEEVENTF_LEFTDOWN);
                        wait_for(&format!("drag starts (stage {stage})"), || {
                            SendMessageW(game, WM_QUERY_SELECTION, Some(WPARAM(0)), Some(LPARAM(0)))
                                .0
                                == start.x as isize
                                && SendMessageW(
                                    game,
                                    WM_QUERY_SELECTION,
                                    Some(WPARAM(1)),
                                    Some(LPARAM(0)),
                                )
                                .0 == start.y as isize
                        });
                        move_to(bounds.left + 600, bounds.top + 450);
                        // The outline is visible while the underlying fixture changes color live.
                        PostMessageW(Some(game), WM_COLOR, WPARAM(0x00408020), LPARAM(0)).unwrap();
                        wait_for(&format!("drag coordinates (stage {stage})"), || {
                            (0..4)
                                .map(|axis| {
                                    SendMessageW(
                                        game,
                                        WM_QUERY_SELECTION,
                                        Some(WPARAM(axis)),
                                        Some(LPARAM(0)),
                                    )
                                    .0
                                })
                                .collect::<Vec<_>>()
                                == vec![
                                    start.x as isize,
                                    start.y as isize,
                                    (bounds.left + 600) as isize,
                                    (bounds.top + 450) as isize,
                                ]
                        });
                        wait_for("blue selection outline", || {
                            pixel(start.x, bounds.top + 300) == COLORREF(0x00e6a030)
                        });
                        wait_for("live game pixels", || {
                            pixel(bounds.left + 400, bounds.top + 300) == COLORREF(0x00408020)
                        });
                        key(VK_F24, false);
                        key(VK_F24, true);
                        assert_eq!(GetForegroundWindow(), game);
                        match stage {
                            0 => {
                                key(VK_ESCAPE, false);
                                wait_for("Esc hides outline", || !IsWindowVisible(frame).as_bool());
                                assert!(
                                    IsWindow(Some(frame)).as_bool(),
                                    "wait for owned Esc/left-button releases"
                                );
                                key(VK_ESCAPE, false);
                                key(VK_ESCAPE, true);
                            }
                            3 => {
                                PostMessageW(Some(frame), WM_DISPLAYCHANGE, WPARAM(32), LPARAM(0))
                                    .unwrap();
                                wait_for("display change cancels", || {
                                    !IsWindowVisible(frame).as_bool()
                                });
                            }
                            4 => {
                                SetWindowPos(
                                    game,
                                    None,
                                    bounds.left + 10,
                                    bounds.top,
                                    0,
                                    0,
                                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSIZE,
                                )
                                .unwrap();
                                wait_for("client-area move cancels", || {
                                    !IsWindowVisible(frame).as_bool()
                                });
                            }
                            5 => {
                                let _ = ShowWindow(other, SW_SHOWNOACTIVATE);
                                assert!(SetForegroundWindow(other).as_bool());
                                wait_for("focus loss cancels", || {
                                    !IsWindowVisible(frame).as_bool()
                                });
                            }
                            _ => {}
                        }
                        mouse(MOUSEEVENTF_LEFTUP);
                    }
                    wait_for("overlay destroyed", || overlay(thread).is_none());
                    assert_eq!(GetForegroundWindow(), if stage == 5 { other } else { game });
                    assert_eq!(
                        pixel(bounds.left + 200, bounds.top + 300),
                        COLORREF(0x00408020),
                        "outline is removed before capture"
                    );
                });
                if outcome.is_err() {
                    // Release only synthetic input from this test and close only its own overlay.
                    key(VK_ESCAPE, true);
                    mouse(MOUSEEVENTF_LEFTUP);
                    mouse(MOUSEEVENTF_RIGHTUP);
                    if let Some(frame) = overlay(thread) {
                        let _ = PostMessageW(Some(frame), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
                outcome
            });
            let result = select_region(game.0, bounds, size).unwrap();
            driver.join().unwrap().unwrap();
            pump();
            assert_eq!(
                counts.selection_input, 0,
                "selection input leaked to the game"
            );
            assert_eq!(
                counts.lost_focus,
                u32::from(stage == 5),
                "the overlay activated another window"
            );
            assert_eq!(
                other_counts.selection_input, 0,
                "owned releases leaked after focus loss"
            );
            if stage == 2 {
                let region = result.unwrap();
                assert_eq!(
                    (region.x, region.y, region.width, region.height),
                    (0.25, 0.25, 0.5, 0.5)
                );
            } else {
                assert!(result.is_none());
            }
        }
        assert_eq!(
            counts.other_input + other_counts.other_input,
            10,
            "ordinary keys must pass through"
        );
        assert!(SetForegroundWindow(game.0).as_bool());
        pump();
        key(VK_ESCAPE, false);
        key(VK_ESCAPE, true);
        mouse(MOUSEEVENTF_LEFTDOWN);
        mouse(MOUSEEVENTF_LEFTUP);
        pump();
        assert_eq!(
            counts.selection_input, 4,
            "hooks must be removed after selection"
        );
    }
}
