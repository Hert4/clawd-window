use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
#[cfg(windows)]
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetShellWindow, GetSystemMetrics, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, IsIconic, IsWindowVisible, GWL_EXSTYLE, SM_CXSCREEN, SM_CYSCREEN,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

#[derive(Clone, Copy, Debug)]
pub struct WinRect {
    pub hwnd: isize,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub type SharedWindows = Arc<RwLock<Vec<WinRect>>>;

pub fn new_shared_windows() -> SharedWindows {
    Arc::new(RwLock::new(Vec::new()))
}

#[cfg(windows)]
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
    };
    result.is_ok() && cloaked != 0
}

#[cfg(windows)]
extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let list_ptr = lparam.0 as *mut Vec<WinRect>;
        let list = &mut *list_ptr;

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        if IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        if hwnd == GetShellWindow() {
            return BOOL(1);
        }
        if is_cloaked(hwnd) {
            return BOOL(1);
        }
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if ex_style & (WS_EX_TOOLWINDOW.0 as i32) != 0 {
            return BOOL(1);
        }
        if ex_style & (WS_EX_NOACTIVATE.0 as i32) != 0 {
            return BOOL(1);
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return BOOL(1);
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return BOOL(1);
        }

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 200 || h < 100 {
            return BOOL(1);
        }
        if rect.left < -10000 || rect.top < -10000 {
            return BOOL(1);
        }
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        if w >= screen_w - 20 && h >= screen_h - 20 {
            return BOOL(1);
        }

        list.push(WinRect {
            hwnd: hwnd.0 as isize,
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        });
        BOOL(1)
    }
}

#[cfg(windows)]
pub fn enumerate_windows(self_hwnd: isize) -> Vec<WinRect> {
    let mut list: Vec<WinRect> = Vec::with_capacity(64);
    let lparam = LPARAM(&mut list as *mut _ as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_proc), lparam);
    }
    list.retain(|w| w.hwnd != self_hwnd);
    list
}

#[cfg(not(windows))]
pub fn enumerate_windows(_self_hwnd: isize) -> Vec<WinRect> {
    Vec::new()
}

pub fn spawn_window_poller(shared: SharedWindows, self_hwnd: isize) {
    tauri::async_runtime::spawn(async move {
        loop {
            let list = enumerate_windows(self_hwnd);
            *shared.write() = list;
            sleep(Duration::from_millis(100)).await;
        }
    });
}
