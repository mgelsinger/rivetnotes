//! Win10/11 dark-mode glue.
//!
//! Most of the surface this module touches is **undocumented**: uxtheme.dll
//! ordinals 133/135/136 (`AllowDarkModeForWindow`, `SetPreferredAppMode`,
//! `FlushMenuThemes`) and the `WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM`
//! messages. Notepad++, VS Code, Windows Terminal, and many other apps
//! rely on the same set, but Microsoft has never published them, so we
//! resolve everything by ordinal at runtime and tolerate failures.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{BOOL, COLORREF, FALSE, HMODULE, HWND, LPARAM, TRUE};
use windows::Win32::Graphics::Dwm::{
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, HBRUSH, HDC};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, SetWindowTheme};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, HMENU};
use windows::core::{PCSTR, PCWSTR, w};

pub const WM_UAHDRAWMENU: u32 = 0x0091;
pub const WM_UAHDRAWMENUITEM: u32 = 0x0092;

#[repr(i32)]
#[derive(Clone, Copy)]
enum PreferredAppMode {
    #[allow(dead_code)]
    Default = 0,
    AllowDark = 1,
    #[allow(dead_code)]
    ForceDark = 2,
    ForceLight = 3,
}

type AllowDarkModeForWindowFn = unsafe extern "system" fn(HWND, BOOL) -> BOOL;
type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemesFn = unsafe extern "system" fn();

struct Uxtheme {
    allow_dark_for_window: Option<AllowDarkModeForWindowFn>,
    set_preferred_app_mode: Option<SetPreferredAppModeFn>,
    flush_menu_themes: Option<FlushMenuThemesFn>,
}

fn uxtheme() -> &'static Uxtheme {
    static CACHE: OnceLock<Uxtheme> = OnceLock::new();
    CACHE.get_or_init(|| unsafe {
        let module = LoadLibraryW(w!("uxtheme.dll")).unwrap_or(HMODULE(0));
        if module.0 == 0 {
            return Uxtheme {
                allow_dark_for_window: None,
                set_preferred_app_mode: None,
                flush_menu_themes: None,
            };
        }
        let allow = GetProcAddress(module, PCSTR(133 as *const u8));
        let set_mode = GetProcAddress(module, PCSTR(135 as *const u8));
        let flush = GetProcAddress(module, PCSTR(136 as *const u8));
        Uxtheme {
            allow_dark_for_window: allow.map(|p| std::mem::transmute(p)),
            set_preferred_app_mode: set_mode.map(|p| std::mem::transmute(p)),
            flush_menu_themes: flush.map(|p| std::mem::transmute(p)),
        }
    })
}

pub fn set_titlebar_dark(hwnd: HWND, dark: bool) {
    if hwnd.0 == 0 {
        return;
    }
    let value: BOOL = if dark { TRUE } else { FALSE };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &value as *const _ as *const c_void,
            std::mem::size_of::<BOOL>() as u32,
        );
        // Pre-20H1 Win10 used attribute 19 for the same flag.
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(19),
            &value as *const _ as *const c_void,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
}

pub fn allow_dark_for_window(hwnd: HWND, dark: bool) {
    if hwnd.0 == 0 {
        return;
    }
    if let Some(f) = uxtheme().allow_dark_for_window {
        unsafe {
            f(hwnd, if dark { TRUE } else { FALSE });
        }
    }
}

pub fn set_preferred_app_mode(dark: bool) {
    if let Some(f) = uxtheme().set_preferred_app_mode {
        let mode = if dark {
            PreferredAppMode::AllowDark
        } else {
            PreferredAppMode::ForceLight
        };
        unsafe {
            let _ = f(mode as i32);
        }
    }
}

pub fn flush_menu_themes() {
    if let Some(f) = uxtheme().flush_menu_themes {
        unsafe {
            f();
        }
    }
}

pub fn apply_to_window(hwnd: HWND, dark: bool) {
    set_preferred_app_mode(dark);
    allow_dark_for_window(hwnd, dark);
    set_titlebar_dark(hwnd, dark);
    flush_menu_themes();
}

/// Strips visual styles from a control so comctl32 stops drawing its themed
/// background, borders, and selection chrome. We use this on the top tab
/// control so the subclass `WM_PAINT` handler can take full ownership of
/// painting — without this, comctl32's themed paint runs first and we can
/// only paint a 3-px seam at the bottom. **Do not** call this on a
/// `WC_LISTVIEWW` in `LVS_REPORT` mode: it collapses item heights to zero
/// and rows render invisible (see the v0.4.13 → v0.4.14 saga). For ListViews,
/// use [`apply_explorer_theme`] to swap between the light and dark Explorer
/// theme variants instead.
pub fn disable_visual_styles(hwnd: HWND) {
    if hwnd.0 == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowTheme(hwnd, w!(""), w!(""));
    }
}

/// Reverses [`disable_visual_styles`]: restores the control's normal themed
/// (visual-styles) rendering. Passing real null pointers (as opposed to
/// empty-string pointers, which is what disables theming) resets the
/// control to its default theme.
pub fn restore_visual_styles(hwnd: HWND) {
    if hwnd.0 == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowTheme(hwnd, PCWSTR::null(), PCWSTR::null());
    }
}

/// Sets a checkbox/radio-style `BS_AUTOCHECKBOX` control's visual style for
/// the given dark-mode state: disabled (classic rendering, so
/// `WM_CTLCOLORBTN`'s text color actually takes effect) when dark, restored
/// (normal themed rendering) when light. Idempotent — safe to call on every
/// theme change, not just once at creation.
pub fn set_checkbox_dark_mode(hwnd: HWND, dark: bool) {
    if dark {
        disable_visual_styles(hwnd);
    } else {
        restore_visual_styles(hwnd);
    }
}

/// Applies the dark or light Explorer visual style to a control. Used on
/// the vertical tab `ListView` so the system-drawn selection rectangle
/// matches our dark theme instead of flashing the light Explorer chrome on
/// click. Unlike [`disable_visual_styles`], this keeps comctl32's paint
/// paths alive — it just substitutes a different named theme.
pub fn apply_explorer_theme(hwnd: HWND, dark: bool) {
    if hwnd.0 == 0 {
        return;
    }
    let theme: PCWSTR = if dark {
        w!("DarkMode_Explorer")
    } else {
        w!("Explorer")
    };
    unsafe {
        let _ = SetWindowTheme(hwnd, theme, PCWSTR::null());
    }
}

pub fn theme_child_controls(parent: HWND, dark: bool) {
    if parent.0 == 0 {
        return;
    }
    unsafe extern "system" fn cb(child: HWND, lparam: LPARAM) -> BOOL {
        let dark = lparam.0 != 0;
        let theme: PCWSTR = if dark {
            w!("DarkMode_Explorer")
        } else {
            w!("Explorer")
        };
        unsafe {
            let _ = SetWindowTheme(child, theme, PCWSTR::null());
        }
        TRUE
    }
    unsafe {
        let _ = EnumChildWindows(parent, Some(cb), LPARAM(if dark { 1 } else { 0 }));
    }
}

/// Cache of solid brushes keyed by COLORREF, so `WM_CTLCOLOR*` handlers can
/// return a stable HBRUSH per color without leaking. Brushes are created
/// lazily on first use and live for the lifetime of the process.
pub fn cached_solid_brush(color: COLORREF) -> HBRUSH {
    static CACHE: OnceLock<Mutex<HashMap<u32, isize>>> = OnceLock::new();
    let map = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = color.0;
    {
        let guard = map.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(&h) = guard.get(&key) {
            return HBRUSH(h);
        }
    }
    let brush = unsafe { CreateSolidBrush(color) };
    let mut guard = map.lock().unwrap_or_else(|p| p.into_inner());
    guard.insert(key, brush.0);
    brush
}

// Undocumented UAH menu-bar layouts. Lifted from Notepad++'s
// `NppDarkMode.cpp`. The system passes a pointer to these in `lParam` for
// `WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM`.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UahMenu {
    pub hmenu: HMENU,
    pub hdc: HDC,
    pub dw_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UahMenuItemMetrics {
    // Covers the larger arm of the union (rgsizePopup[4] = 4 * 2 * u32).
    pub raw: [u32; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UahMenuPopupMetrics {
    pub rgcx: [u32; 4],
    pub fupdate_max_widths: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UahMenuItem {
    pub i_position: i32,
    pub umim: UahMenuItemMetrics,
    pub umpm: UahMenuPopupMetrics,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UahDrawMenuItem {
    pub dis: DRAWITEMSTRUCT,
    pub um: UahMenu,
    pub umi: UahMenuItem,
}
