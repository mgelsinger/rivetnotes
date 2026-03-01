use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use windows::Win32::Foundation::{
    COLORREF, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT,
    WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, HBRUSH, HDC, InvalidateRect, ScreenToClient, SetBkColor,
    SetTextColor,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Controls::{
    ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, NM_RCLICK, NMHDR, SB_SETPARTS,
    SB_SETTEXTW, STATUSCLASSNAMEW, TCHITTESTINFO, TCIF_TEXT, TCITEMW, TCM_DELETEITEM,
    TCM_GETCURSEL, TCM_GETITEMRECT, TCM_HITTEST, TCM_INSERTITEMW, TCM_SETCURSEL, TCM_SETITEMW,
    TCN_SELCHANGE, WC_TABCONTROLW,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, DragAcceptFiles, DragFinish,
    DragQueryFileW, HDROP, SHBrowseForFolderW, SHGetPathFromIDListW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, AppendMenuW, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_PUSHBUTTON, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, CheckMenuItem, CreateAcceleratorTableW, CreateMenu, CreatePopupMenu,
    CreateWindowExW, DefWindowProcW, DestroyAcceleratorTable, DestroyMenu, DestroyWindow,
    DispatchMessageW, ES_AUTOHSCROLL, EnableMenuItem, FALT, FCONTROL, FSHIFT, FVIRTKEY, GCLP_HICON,
    GCLP_HICONSM, GWLP_USERDATA, GetClientRect, GetCursorPos, GetMenu, GetMessageW, GetParent,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, HACCEL, HICON,
    HMENU, HWND_NOTOPMOST, HWND_TOPMOST, ICON_BIG, ICON_SMALL, ICON_SMALL2, IDC_ARROW, IDC_SIZEWE,
    IDI_APPLICATION, IDNO, IDYES, IMAGE_ICON, KillTimer, LB_ADDSTRING, LB_GETCURSEL,
    LB_RESETCONTENT, LB_SETCURSEL, LBN_DBLCLK, LBN_SELCHANGE, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY,
    LR_DEFAULTCOLOR, LR_SHARED, LoadCursorW, LoadIconW, LoadImageW, MB_ICONERROR, MB_ICONWARNING,
    MB_OK, MB_YESNO, MB_YESNOCANCEL, MF_BYCOMMAND, MF_CHECKED, MF_ENABLED, MF_GRAYED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SYSTEM_METRICS_INDEX, SendMessageW, SetClassLongPtrW, SetCursor, SetTimer,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, TranslateAcceleratorW, TranslateMessage, WINDOW_STYLE,
    WM_ACTIVATEAPP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_CREATE, WM_CTLCOLORLISTBOX,
    WM_DESTROY, WM_DROPFILES, WM_GETICON, WM_INITMENUPOPUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_NCDESTROY, WM_NOTIFY, WM_SETCURSOR, WM_SETICON, WM_SIZE,
    WM_TIMER, WNDCLASSEXW, WS_BORDER, WS_CHILD, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW, WS_TABSTOP,
    WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PWSTR;
use windows::core::{HSTRING, PCWSTR, w};

use crate::app::document::{self, Document, Eol, TextEncoding};
use crate::app::session;
use crate::commands::copy_full_path::{
    CopyPathKind, can_copy_directory_path, can_copy_filename, can_copy_full_path,
    copy_directory_path, copy_filename, copy_full_path,
};
use crate::commands::selection::{can_lowercase, can_uppercase};
use crate::editor::scintilla;
use crate::error::{AppError, Result};
use crate::logging;
use crate::platform::clipboard::WinClipboard;
use crate::textops::trim::{trim_edges_spaces_tabs, trim_line_preserve_eol};
use regex::RegexBuilder;

const IDM_FILE_NEW: u16 = 99;
const IDM_FILE_OPEN: u16 = 100;
const IDM_FILE_SAVE: u16 = 101;
const IDM_FILE_SAVE_AS: u16 = 102;
const IDM_FILE_SAVE_AS_UTF8_BOM: u16 = 103;
const IDM_FILE_SAVE_AS_UTF16_LE: u16 = 104;
const IDM_FILE_SAVE_ALL: u16 = 105;
const IDM_FILE_EXIT: u16 = 199;
const IDM_EDIT_UNDO: u16 = 300;
const IDM_EDIT_REDO: u16 = 301;
const IDM_EDIT_CUT: u16 = 302;
const IDM_EDIT_COPY: u16 = 303;
const IDM_EDIT_PASTE: u16 = 304;
const IDM_EDIT_SELECT_ALL: u16 = 305;
const IDM_EDIT_DUPLICATE_LINE: u16 = 306;
const IDM_EDIT_DELETE_LINE: u16 = 307;
const IDM_EDIT_MOVE_LINE_UP: u16 = 308;
const IDM_EDIT_MOVE_LINE_DOWN: u16 = 309;
const IDM_EDIT_INDENT: u16 = 310;
const IDM_EDIT_OUTDENT: u16 = 311;
const CMD_TRIM_LEADING_TRAILING: u16 = 312;
const IDM_EDIT_FIND: u16 = 320;
const IDM_EDIT_FIND_NEXT: u16 = 321;
const IDM_EDIT_FIND_PREV: u16 = 322;
const IDM_EDIT_REPLACE: u16 = 323;
const IDM_EDIT_REPLACE_ALL: u16 = 324;
const IDM_EDIT_FIND_IN_FILES: u16 = 325;
const CMD_TRANSFORM_UPPERCASE: u16 = 326;
const CMD_TRANSFORM_LOWERCASE: u16 = 327;
const CMD_COPY_FULL_PATH: u16 = 328;
const CMD_COPY_FILENAME: u16 = 329;
const CMD_COPY_DIRECTORY_PATH: u16 = 330;
const IDM_VIEW_EDITOR_DARK: u16 = 340;
const IDM_VIEW_TABS_HORIZONTAL: u16 = 341;
const IDM_VIEW_TABS_VERTICAL_LEFT: u16 = 342;
const IDM_VIEW_TABS_VERTICAL_RIGHT: u16 = 343;
const IDM_VIEW_TABS_CYCLE: u16 = 344;
const IDM_VIEW_WORD_WRAP: u16 = 345;
const IDM_VIEW_ALWAYS_ON_TOP: u16 = 346;
const IDM_TAB_CLOSE: u16 = 220;
const IDM_TAB_CLOSE_OTHERS: u16 = 221;
const IDM_TAB_CLOSE_RIGHT: u16 = 222;
const CMD_TAB_DUPLICATE: u16 = 223;
const CMD_TAB_CLOSE_LEFT: u16 = 224;
const CMD_EDITOR_DELETE: u16 = 331;
const IDM_HELP_ABOUT: u16 = 400;

const TIMER_SESSION_ID: usize = 1;
const SESSION_INTERVAL_MS: u32 = 5000;
const TIMER_FIND_RESULTS: usize = 2;
const TIMER_WORD_COUNT: usize = 3;
const WORD_COUNT_INTERVAL_MS: u32 = 250;
const TAB_SPLITTER_WIDTH: i32 = 4;

const SCN_SAVEPOINTREACHED: u32 = 2002;
const SCN_SAVEPOINTLEFT: u32 = 2003;
const SCN_UPDATEUI: u32 = 2007;
const SCN_MODIFIED: u32 = 2008;

const VK_A: u16 = 0x41;
const VK_C: u16 = 0x43;
const VK_D: u16 = 0x44;
const VK_F: u16 = 0x46;
const VK_H: u16 = 0x48;
const VK_L: u16 = 0x4C;
const VK_T: u16 = 0x54;
const VK_V: u16 = 0x56;
const VK_W: u16 = 0x57;
const VK_X: u16 = 0x58;
const VK_Y: u16 = 0x59;
const VK_Z: u16 = 0x5A;
const VK_F3: u16 = 0x72;
const VK_N: u16 = 0x4E;
const VK_S: u16 = 0x53;
const VK_OEM_4: u16 = 0xDB;
const VK_OEM_6: u16 = 0xDD;
const VK_UP: u16 = 0x26;
const VK_DOWN: u16 = 0x28;

const IDC_FIND_TEXT: usize = 5001;
const IDC_REPLACE_TEXT: usize = 5002;
const IDC_MATCH_CASE: usize = 5003;
const IDC_WHOLE_WORD: usize = 5004;
const IDC_REGEX: usize = 5005;
const IDC_WRAP: usize = 5006;
const IDC_FIND_NEXT: usize = 5007;
const IDC_FIND_PREV: usize = 5008;
const IDC_REPLACE: usize = 5009;
const IDC_REPLACE_ALL: usize = 5010;
const IDC_FIND_CLOSE: usize = 5011;
const IDC_FIND_IN_FILES: usize = 5012;

const IDC_FIF_TEXT: usize = 5101;
const IDC_FIF_FOLDER: usize = 5102;
const IDC_FIF_INCLUDE: usize = 5103;
const IDC_FIF_EXCLUDE: usize = 5104;
const IDC_FIF_BROWSE: usize = 5105;
const IDC_FIF_MATCH_CASE: usize = 5106;
const IDC_FIF_WHOLE_WORD: usize = 5107;
const IDC_FIF_REGEX: usize = 5108;
const IDC_FIF_RECURSE: usize = 5109;
const IDC_FIF_FIND: usize = 5110;
const IDC_FIF_CANCEL: usize = 5111;
const IDC_FIF_CLOSE: usize = 5112;
const IDC_FIF_RESULTS: usize = 5113;
const IDC_TAB_LIST: usize = 5201;

const FIND_CLASS: PCWSTR = w!("rivet_find_dialog");
const FIND_FILES_CLASS: PCWSTR = w!("rivet_find_in_files");
const SPLITTER_CLASS: PCWSTR = w!("rivet_tab_splitter");

const SCFIND_MATCHCASE: usize = 0x4;
const SCFIND_WHOLEWORD: usize = 0x2;
const SCFIND_REGEXP: usize = 0x0020_0000;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetFocus(hwnd: HWND) -> HWND;
}

struct DocTab {
    editor: HWND,
    doc: Document,
    display_name: Option<String>,
    dirty: bool,
    wrap_enabled: bool,
    word_count: Option<usize>,
}

struct FindState {
    find_text: String,
    replace_text: String,
    match_case: bool,
    whole_word: bool,
    regex: bool,
    wrap: bool,
}

struct FindDialogState {
    hwnd: HWND,
    find_edit: HWND,
    replace_edit: HWND,
    match_case: HWND,
    whole_word: HWND,
    regex: HWND,
    wrap: HWND,
}

struct FindHit {
    path: PathBuf,
    line: usize,
    text: String,
}

enum FindResult {
    Match(FindHit),
    Done,
}

struct FindInFilesState {
    hwnd: HWND,
    find_edit: HWND,
    folder_edit: HWND,
    include_edit: HWND,
    exclude_edit: HWND,
    match_case: HWND,
    whole_word: HWND,
    regex: HWND,
    recurse: HWND,
    results: HWND,
    cancel: Arc<AtomicBool>,
    receiver: Option<Receiver<FindResult>>,
    running: bool,
    hits: Vec<FindHit>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TabLayout {
    HorizontalTop,
    VerticalLeft,
    VerticalRight,
}

impl TabLayout {
    fn next(self) -> Self {
        match self {
            TabLayout::HorizontalTop => TabLayout::VerticalLeft,
            TabLayout::VerticalLeft => TabLayout::VerticalRight,
            TabLayout::VerticalRight => TabLayout::HorizontalTop,
        }
    }
}

struct AppState {
    tabs: HWND,
    tab_list: HWND,
    tab_splitter: HWND,
    tab_list_brush: HBRUSH,
    tab_list_text: COLORREF,
    tab_list_back: COLORREF,
    status: HWND,
    docs: Vec<DocTab>,
    active: usize,
    editor_dark: bool,
    tab_layout: TabLayout,
    tab_list_width: i32,
    resizing_tabs: bool,
    always_on_top: bool,
    icon_big: HICON,
    icon_small: HICON,
    word_count_pending: bool,
    word_count_timer: bool,
    find_state: FindState,
    find_dialog: Option<FindDialogState>,
    find_in_files: Option<FindInFilesState>,
}

pub fn run() -> Result<()> {
    let start = Instant::now();

    let instance: HINSTANCE = unsafe { GetModuleHandleW(None) }?.into();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    scintilla::register_classes(instance)?;

    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        })
        .ok()?;
    }

    let class_name = w!("rivet_main_window");
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW)? };
    let (icon, icon_sm) = load_main_icons(instance);
    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: instance,
        hCursor: cursor,
        hIcon: icon,
        hIconSm: icon_sm,
        lpszClassName: class_name,
        ..Default::default()
    };

    let atom = unsafe { RegisterClassExW(&wnd_class) };
    if atom == 0 {
        return Err(AppError::win32("RegisterClassExW"));
    }

    register_aux_classes(instance)?;

    let menu = create_menu()?;
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            w!("Rivet"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND(0),
            menu,
            instance,
            None,
        )
    };

    if hwnd.0 == 0 {
        return Err(AppError::win32("CreateWindowExW"));
    }

    set_window_icons(hwnd, instance);

    unsafe {
        ShowWindow(hwnd, SW_SHOW);
    }

    let accel = create_accelerators()?;

    eprintln!("startup_ms={}", start.elapsed().as_millis());

    let result = message_loop(hwnd, accel);
    unsafe {
        let _ = DestroyAcceleratorTable(accel);
    }
    result
}

pub fn show_error(title: &str, message: &str) {
    logging::log_error(&format!("{title}: {message}"));
    let title = HSTRING::from(title);
    let message = HSTRING::from(message);
    unsafe {
        MessageBoxW(
            HWND(0),
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn create_menu() -> Result<HMENU> {
    unsafe {
        let menu = CreateMenu()?;
        let file_menu = CreatePopupMenu()?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_NEW as usize, w!("New"))?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_OPEN as usize, w!("Open..."))?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_SAVE as usize, w!("Save"))?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_SAVE_ALL as usize,
            w!("Save All"),
        )?;
        AppendMenuW(file_menu, MF_STRING, IDM_TAB_CLOSE as usize, w!("Close"))?;
        AppendMenuW(file_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_SAVE_AS as usize,
            w!("Save As..."),
        )?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_SAVE_AS_UTF8_BOM as usize,
            w!("Save As (UTF-8 BOM)"),
        )?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_SAVE_AS_UTF16_LE as usize,
            w!("Save As (UTF-16 LE)"),
        )?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_EXIT as usize, w!("Exit"))?;
        AppendMenuW(menu, MF_POPUP, file_menu.0 as usize, w!("File"))?;

        let edit_menu = CreatePopupMenu()?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_FIND as usize, w!("Find..."))?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_FIND_NEXT as usize,
            w!("Find Next"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_FIND_PREV as usize,
            w!("Find Previous"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_REPLACE as usize,
            w!("Replace..."),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_REPLACE_ALL as usize,
            w!("Replace All"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_FIND_IN_FILES as usize,
            w!("Find in Files..."),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_UNDO as usize, w!("Undo"))?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_REDO as usize, w!("Redo"))?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_CUT as usize, w!("Cut"))?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_COPY as usize, w!("Copy"))?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_PASTE as usize, w!("Paste"))?;
        let copy_to_clipboard_menu = CreatePopupMenu()?;
        AppendMenuW(
            copy_to_clipboard_menu,
            MF_STRING,
            CMD_COPY_FULL_PATH as usize,
            w!("Copy Full Path"),
        )?;
        AppendMenuW(
            copy_to_clipboard_menu,
            MF_STRING,
            CMD_COPY_FILENAME as usize,
            w!("Copy Filename"),
        )?;
        AppendMenuW(
            copy_to_clipboard_menu,
            MF_STRING,
            CMD_COPY_DIRECTORY_PATH as usize,
            w!("Copy Directory Path"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_POPUP,
            copy_to_clipboard_menu.0 as usize,
            w!("Copy to Clipboard"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_SELECT_ALL as usize,
            w!("Select All"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_DUPLICATE_LINE as usize,
            w!("Duplicate Line"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_DELETE_LINE as usize,
            w!("Delete Line"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_MOVE_LINE_UP as usize,
            w!("Move Line Up"),
        )?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_MOVE_LINE_DOWN as usize,
            w!("Move Line Down"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(edit_menu, MF_STRING, IDM_EDIT_INDENT as usize, w!("Indent"))?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_OUTDENT as usize,
            w!("Outdent"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            CMD_TRIM_LEADING_TRAILING as usize,
            w!("Trim Leading + Trailing Whitespace"),
        )?;
        AppendMenuW(menu, MF_POPUP, edit_menu.0 as usize, w!("Edit"))?;

        let help_menu = CreatePopupMenu()?;
        AppendMenuW(help_menu, MF_STRING, IDM_HELP_ABOUT as usize, w!("About"))?;
        AppendMenuW(menu, MF_POPUP, help_menu.0 as usize, w!("Help"))?;

        Ok(menu)
    }
}

fn register_aux_classes(instance: HINSTANCE) -> Result<()> {
    register_window_class(instance, FIND_CLASS, Some(find_wndproc))?;
    register_window_class(instance, FIND_FILES_CLASS, Some(find_in_files_wndproc))?;
    register_window_class(instance, SPLITTER_CLASS, Some(splitter_wndproc))?;
    Ok(())
}

fn register_window_class(
    instance: HINSTANCE,
    name: PCWSTR,
    proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
) -> Result<()> {
    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: proc,
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        lpszClassName: name,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassExW(&wnd_class) };
    if atom == 0 {
        let err = unsafe { GetLastError() };
        if err != ERROR_CLASS_ALREADY_EXISTS {
            return Err(AppError::win32("RegisterClassExW(aux)"));
        }
    }
    Ok(())
}

#[allow(clippy::manual_dangling_ptr)]
fn load_main_icons(instance: HINSTANCE) -> (HICON, HICON) {
    let resource = PCWSTR(1 as *const u16);
    let default_icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or(HICON(0));
    let big = load_icon_scaled(instance, resource, SM_CXICON, SM_CYICON, default_icon);
    let small = load_icon_scaled(instance, resource, SM_CXSMICON, SM_CYSMICON, default_icon);
    (big, small)
}

fn load_icon_scaled(
    instance: HINSTANCE,
    resource: PCWSTR,
    cx_metric: SYSTEM_METRICS_INDEX,
    cy_metric: SYSTEM_METRICS_INDEX,
    fallback: HICON,
) -> HICON {
    let cx = unsafe { GetSystemMetrics(cx_metric) };
    let cy = unsafe { GetSystemMetrics(cy_metric) };
    let handle = unsafe {
        LoadImageW(
            instance,
            resource,
            IMAGE_ICON,
            cx,
            cy,
            LR_DEFAULTCOLOR | LR_SHARED,
        )
    };
    if let Ok(icon) = handle
        && icon.0 != 0
    {
        return HICON(icon.0);
    }
    unsafe { LoadIconW(instance, resource) }.unwrap_or(fallback)
}

fn set_window_icons(hwnd: HWND, instance: HINSTANCE) {
    let (icon, icon_sm) = load_main_icons(instance);
    if icon.0 != 0 {
        unsafe {
            SetClassLongPtrW(hwnd, GCLP_HICON, icon.0);
        }
        unsafe {
            SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(icon.0));
        }
    }
    if icon_sm.0 != 0 {
        unsafe {
            SetClassLongPtrW(hwnd, GCLP_HICONSM, icon_sm.0);
        }
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_SMALL as usize),
                LPARAM(icon_sm.0),
            );
        }
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_SMALL2 as usize),
                LPARAM(icon_sm.0),
            );
        }
    }
}

fn create_accelerators() -> Result<HACCEL> {
    let accels = [
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_Z,
            cmd: IDM_EDIT_UNDO,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_N,
            cmd: IDM_FILE_NEW,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_F,
            cmd: IDM_EDIT_FIND,
        },
        ACCEL {
            fVirt: FVIRTKEY,
            key: VK_F3,
            cmd: IDM_EDIT_FIND_NEXT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FSHIFT,
            key: VK_F3,
            cmd: IDM_EDIT_FIND_PREV,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_H,
            cmd: IDM_EDIT_REPLACE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_F,
            cmd: IDM_EDIT_FIND_IN_FILES,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_Y,
            cmd: IDM_EDIT_REDO,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_X,
            cmd: IDM_EDIT_CUT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_C,
            cmd: IDM_EDIT_COPY,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_V,
            cmd: IDM_EDIT_PASTE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_A,
            cmd: IDM_EDIT_SELECT_ALL,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_D,
            cmd: IDM_EDIT_DUPLICATE_LINE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_L,
            cmd: IDM_EDIT_DELETE_LINE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FALT,
            key: VK_UP,
            cmd: IDM_EDIT_MOVE_LINE_UP,
        },
        ACCEL {
            fVirt: FVIRTKEY | FALT,
            key: VK_DOWN,
            cmd: IDM_EDIT_MOVE_LINE_DOWN,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_OEM_6,
            cmd: IDM_EDIT_INDENT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_OEM_4,
            cmd: IDM_EDIT_OUTDENT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_T,
            cmd: CMD_TRIM_LEADING_TRAILING,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_S,
            cmd: IDM_FILE_SAVE_ALL,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_W,
            cmd: IDM_TAB_CLOSE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FALT,
            key: VK_T,
            cmd: IDM_VIEW_TABS_CYCLE,
        },
    ];

    let accel = unsafe { CreateAcceleratorTableW(&accels)? };
    Ok(accel)
}

fn message_loop(hwnd: HWND, accel: HACCEL) -> Result<()> {
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, HWND(0), 0, 0) };
        if result.0 == -1 {
            return Err(AppError::win32("GetMessageW"));
        }
        if result.0 == 0 {
            break;
        }
        if message.message == WM_MBUTTONUP
            && let Some(state) = get_state(hwnd)
            && message.hwnd == state.tabs
        {
            if let Some(index) = tab_index_at_point(state.tabs, state.docs.len(), message.lParam)
                && let Err(err) = close_tab(hwnd, state, index)
            {
                show_error("Rivet error", &err.to_string());
            }
            continue;
        }
        unsafe {
            if TranslateAcceleratorW(hwnd, accel, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    Ok(())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let instance = match unsafe { GetModuleHandleW(None) } {
                Ok(value) => value.into(),
                Err(_) => {
                    show_error("Rivet error", "Failed to get module handle.");
                    return LRESULT(-1);
                }
            };

            match create_children(hwnd, instance) {
                Ok(state) => {
                    let state_ptr = Box::into_raw(Box::new(state));
                    unsafe {
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
                        DragAcceptFiles(hwnd, true);
                    }
                    set_window_icons(hwnd, instance);
                    LRESULT(0)
                }
                Err(err) => {
                    show_error("Rivet error", &err.to_string());
                    LRESULT(-1)
                }
            }
        }
        WM_GETICON => {
            if let Some(state) = get_state(hwnd) {
                let icon = match wparam.0 as u32 {
                    ICON_BIG => state.icon_big,
                    ICON_SMALL | ICON_SMALL2 => state.icon_small,
                    _ => HICON(0),
                };
                if icon.0 != 0 {
                    return LRESULT(icon.0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_SIZE => {
            if let Some(state) = get_state(hwnd) {
                layout_children(hwnd, state);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(state) = get_state(hwnd)
                && state.resizing_tabs
            {
                let x = lparam_x(lparam);
                let mut rect = windows::Win32::Foundation::RECT::default();
                unsafe {
                    let _ = GetClientRect(hwnd, &mut rect);
                }
                let width = rect.right - rect.left;
                let desired = match state.tab_layout {
                    TabLayout::VerticalLeft => x,
                    TabLayout::VerticalRight => width - x - TAB_SPLITTER_WIDTH,
                    TabLayout::HorizontalTop => state.tab_list_width,
                };
                if state.tab_layout != TabLayout::HorizontalTop {
                    state.tab_list_width = clamp_tab_list_width(state, desired, width);
                    layout_children(hwnd, state);
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_LBUTTONUP => {
            if let Some(state) = get_state(hwnd)
                && state.resizing_tabs
            {
                state.resizing_tabs = false;
                unsafe {
                    let _ = ReleaseCapture();
                }
            }
            LRESULT(0)
        }
        WM_CONTEXTMENU => {
            let source = HWND(wparam.0 as isize);
            if let Some(state) = get_state(hwnd)
                && doc_index_by_hwnd(state, source).is_some()
            {
                let (x, y) = context_menu_position(lparam);
                if let Some(command_id) = show_editor_context_menu(hwnd, source, x, y) {
                    unsafe {
                        SendMessageW(hwnd, WM_COMMAND, WPARAM(command_id as usize), LPARAM(0));
                    }
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_INITMENUPOPUP => {
            if let Some(state) = get_state(hwnd) {
                update_copy_path_menu(hwnd, state);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let command_id = (wparam.0 & 0xffff) as u16;
            if lparam.0 != 0 {
                let notify = hiword(wparam.0);
                if command_id as usize == IDC_TAB_LIST && notify == LBN_SELCHANGE as u16 {
                    if let Some(state) = get_state(hwnd) {
                        let index = unsafe {
                            SendMessageW(state.tab_list, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0
                        };
                        if index >= 0 {
                            select_tab(hwnd, state, index as usize);
                        }
                    }
                    return LRESULT(0);
                }
            }
            match command_id {
                IDM_FILE_OPEN => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = open_from_dialog(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_UNDO => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::undo(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_FIND => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = show_find_dialog(hwnd, state, true)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_FIND_NEXT => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = perform_find_next(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_FIND_PREV => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = perform_find_prev(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_REPLACE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = show_find_dialog(hwnd, state, false)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_REPLACE_ALL => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = perform_replace_all(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_FIND_IN_FILES => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = show_find_in_files_dialog(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_REDO => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::redo(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_CUT => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::cut(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_COPY => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::copy(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_PASTE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::paste(editor);
                    }
                    LRESULT(0)
                }
                CMD_EDITOR_DELETE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::clear(editor);
                    }
                    LRESULT(0)
                }
                CMD_COPY_FULL_PATH => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) =
                            copy_path_to_clipboard(hwnd, state, CopyPathKind::FullPath)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                CMD_COPY_FILENAME => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) =
                            copy_path_to_clipboard(hwnd, state, CopyPathKind::FileName)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                CMD_COPY_DIRECTORY_PATH => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) =
                            copy_path_to_clipboard(hwnd, state, CopyPathKind::DirectoryPath)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_EDIT_SELECT_ALL => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::select_all(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_DUPLICATE_LINE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::duplicate_line(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_DELETE_LINE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::delete_line(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_MOVE_LINE_UP => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::move_line_up(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_MOVE_LINE_DOWN => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::move_line_down(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_INDENT => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::indent_selection(editor);
                    }
                    LRESULT(0)
                }
                IDM_EDIT_OUTDENT => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::outdent_selection(editor);
                    }
                    LRESULT(0)
                }
                CMD_TRANSFORM_UPPERCASE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        transform_selection_case(editor, true);
                    }
                    LRESULT(0)
                }
                CMD_TRANSFORM_LOWERCASE => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        transform_selection_case(editor, false);
                    }
                    LRESULT(0)
                }
                CMD_TRIM_LEADING_TRAILING => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        trim_leading_and_trailing_whitespace(editor);
                    }
                    LRESULT(0)
                }
                IDM_FILE_NEW => {
                    if let Some(state) = get_state(hwnd) {
                        let instance = match module_instance() {
                            Ok(instance) => instance,
                            Err(err) => {
                                show_error("Rivet error", &err.to_string());
                                return LRESULT(0);
                            }
                        };
                        if let Err(err) = create_empty_tab(hwnd, instance, state) {
                            show_error("Rivet error", &err.to_string());
                        }
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_HORIZONTAL => {
                    if let Some(state) = get_state(hwnd) {
                        set_tab_layout(hwnd, state, TabLayout::HorizontalTop);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_VERTICAL_LEFT => {
                    if let Some(state) = get_state(hwnd) {
                        set_tab_layout(hwnd, state, TabLayout::VerticalLeft);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_VERTICAL_RIGHT => {
                    if let Some(state) = get_state(hwnd) {
                        set_tab_layout(hwnd, state, TabLayout::VerticalRight);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_CYCLE => {
                    if let Some(state) = get_state(hwnd) {
                        let next = state.tab_layout.next();
                        set_tab_layout(hwnd, state, next);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_WORD_WRAP => {
                    if let Some(state) = get_state(hwnd) {
                        toggle_word_wrap(hwnd, state);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_ALWAYS_ON_TOP => {
                    if let Some(state) = get_state(hwnd) {
                        let enabled = !state.always_on_top;
                        set_always_on_top(hwnd, state, enabled);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_EDITOR_DARK => {
                    if let Some(state) = get_state(hwnd) {
                        let enabled = !state.editor_dark;
                        set_editor_dark_mode(hwnd, state, enabled);
                    }
                    LRESULT(0)
                }
                IDM_FILE_SAVE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = save_document(hwnd, state, None, false)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_SAVE_ALL => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = save_all_documents(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_SAVE_AS => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = save_document(hwnd, state, Some(TextEncoding::Utf8), true)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_SAVE_AS_UTF8_BOM => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) =
                            save_document(hwnd, state, Some(TextEncoding::Utf8Bom), true)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_SAVE_AS_UTF16_LE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) =
                            save_document(hwnd, state, Some(TextEncoding::Utf16Le), true)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_TAB_CLOSE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = close_tab(hwnd, state, state.active)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_TAB_CLOSE_OTHERS => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = close_other_tabs(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_TAB_CLOSE_RIGHT => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = close_tabs_to_right(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                CMD_TAB_CLOSE_LEFT => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = close_tabs_to_left(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                CMD_TAB_DUPLICATE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = duplicate_active_tab(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_EXIT => {
                    if let Some(state) = get_state(hwnd)
                        && confirm_close_all(hwnd, state).unwrap_or(true)
                    {
                        let _ = unsafe { DestroyWindow(hwnd) };
                    }
                    LRESULT(0)
                }
                IDM_HELP_ABOUT => {
                    show_error("Rivet", "Rivet is starting up.");
                    LRESULT(0)
                }
                _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        WM_CTLCOLORLISTBOX => {
            if let Some(state) = get_state(hwnd) {
                let target = HWND(lparam.0);
                if target == state.tab_list {
                    let hdc = HDC(wparam.0 as isize);
                    unsafe {
                        SetTextColor(hdc, state.tab_list_text);
                        SetBkColor(hdc, state.tab_list_back);
                    }
                    return LRESULT(state.tab_list_brush.0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_NOTIFY => {
            let nmhdr = unsafe { &*(lparam.0 as *const NMHDR) };
            if nmhdr.hwndFrom != HWND(0) {
                if let Some(state) = get_state(hwnd)
                    && nmhdr.hwndFrom == state.tabs
                    && nmhdr.code == NM_RCLICK
                {
                    if let Some((index, x, y)) = tab_hit_test_at_cursor(state.tabs) {
                        select_tab(hwnd, state, index);
                        if let Some(command_id) = show_tab_context_menu(hwnd, state, index, x, y) {
                            unsafe {
                                SendMessageW(
                                    hwnd,
                                    WM_COMMAND,
                                    WPARAM(command_id as usize),
                                    LPARAM(0),
                                );
                            }
                        }
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == TCN_SELCHANGE {
                    if let Some(state) = get_state(hwnd) {
                        let index = unsafe {
                            SendMessageW(state.tabs, TCM_GETCURSEL, WPARAM(0), LPARAM(0)).0
                        } as i32;
                        if index >= 0 {
                            select_tab(hwnd, state, index as usize);
                        }
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_SAVEPOINTLEFT || nmhdr.code == SCN_SAVEPOINTREACHED {
                    if let Some(state) = get_state(hwnd)
                        && let Some(index) = doc_index_by_hwnd(state, nmhdr.hwndFrom)
                    {
                        let is_dirty = nmhdr.code == SCN_SAVEPOINTLEFT;
                        set_dirty(state, index, is_dirty);
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_UPDATEUI {
                    if let Some(state) = get_state(hwnd) {
                        update_status_position(state);
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_MODIFIED {
                    if let Some(state) = get_state(hwnd) {
                        schedule_word_count(hwnd, state);
                    }
                    return LRESULT(0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_TIMER => {
            if wparam.0 == TIMER_SESSION_ID
                && let Some(state) = get_state(hwnd)
            {
                let _ = save_session_checkpoint(state);
            } else if wparam.0 == TIMER_FIND_RESULTS
                && let Some(state) = get_state(hwnd)
            {
                poll_find_results(hwnd, state);
            } else if wparam.0 == TIMER_WORD_COUNT
                && let Some(state) = get_state(hwnd)
            {
                handle_word_count_timer(hwnd, state);
            }
            LRESULT(0)
        }
        WM_DROPFILES => {
            if let Some(state) = get_state(hwnd) {
                let hdrop = HDROP(wparam.0 as isize);
                if let Err(err) = open_from_drop(hwnd, state, hdrop) {
                    show_error("Rivet error", &err.to_string());
                }
            }
            LRESULT(0)
        }
        WM_ACTIVATEAPP => {
            if wparam.0 != 0
                && let Some(state) = get_state(hwnd)
                && let Err(err) = check_external_change(hwnd, state)
            {
                show_error("Rivet error", &err.to_string());
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            if let Some(state) = get_state(hwnd)
                && confirm_close_all(hwnd, state).unwrap_or(true)
            {
                let _ = unsafe { DestroyWindow(hwnd) };
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = get_state(hwnd) {
                let _ = save_session_checkpoint(state);
            }
            unsafe {
                let _ = KillTimer(hwnd, TIMER_SESSION_ID);
                let _ = KillTimer(hwnd, TIMER_FIND_RESULTS);
                let _ = KillTimer(hwnd, TIMER_WORD_COUNT);
            }
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
            if !state_ptr.is_null() {
                unsafe {
                    destroy_tab_list_brush(&mut *state_ptr);
                    drop(Box::from_raw(state_ptr));
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn create_children(hwnd: HWND, instance: HINSTANCE) -> Result<AppState> {
    let (icon_big, icon_small) = load_main_icons(instance);
    let tabs = unsafe {
        CreateWindowExW(
            Default::default(),
            WC_TABCONTROLW,
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            hwnd,
            HMENU(1),
            instance,
            None,
        )
    };
    if tabs.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(TabControl)"));
    }

    let tab_list = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("LISTBOX"),
            PCWSTR::null(),
            window_style(
                WS_CHILD.0
                    | WS_BORDER.0
                    | WS_VSCROLL.0
                    | WS_TABSTOP.0
                    | LBS_NOTIFY as u32
                    | LBS_NOINTEGRALHEIGHT as u32,
            ),
            0,
            0,
            0,
            0,
            hwnd,
            menu_id(IDC_TAB_LIST),
            instance,
            None,
        )
    };
    if tab_list.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(TabList)"));
    }

    let tab_splitter = unsafe {
        CreateWindowExW(
            Default::default(),
            SPLITTER_CLASS,
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            hwnd,
            HMENU(0),
            instance,
            None,
        )
    };
    if tab_splitter.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(TabSplitter)"));
    }

    let status = unsafe {
        CreateWindowExW(
            Default::default(),
            STATUSCLASSNAMEW,
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            hwnd,
            HMENU(2),
            instance,
            None,
        )
    };
    if status.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(StatusBar)"));
    }

    let (tab_list_text, tab_list_back) = tab_list_colors(true);
    let tab_list_brush = unsafe { CreateSolidBrush(tab_list_back) };
    if tab_list_brush.0 == 0 {
        return Err(AppError::win32("CreateSolidBrush(TabList)"));
    }

    let state = AppState {
        tabs,
        tab_list,
        tab_splitter,
        tab_list_brush,
        tab_list_text,
        tab_list_back,
        status,
        docs: Vec::new(),
        active: 0,
        editor_dark: true,
        tab_layout: TabLayout::HorizontalTop,
        tab_list_width: scale_for_dpi(hwnd, 200),
        resizing_tabs: false,
        always_on_top: false,
        icon_big,
        icon_small,
        word_count_pending: false,
        word_count_timer: false,
        find_state: FindState {
            find_text: String::new(),
            replace_text: String::new(),
            match_case: false,
            whole_word: false,
            regex: false,
            wrap: true,
        },
        find_dialog: None,
        find_in_files: None,
    };

    let mut state = restore_session(hwnd, state)?;
    if state.docs.is_empty() {
        create_empty_tab(hwnd, instance, &mut state)?;
    }

    let show_vertical = if state.tab_layout == TabLayout::HorizontalTop {
        SW_HIDE
    } else {
        SW_SHOW
    };
    unsafe {
        ShowWindow(state.tab_list, show_vertical);
        ShowWindow(state.tab_splitter, show_vertical);
    }

    let active = state.active.min(state.docs.len().saturating_sub(1));
    select_tab(hwnd, &mut state, active);
    layout_children(hwnd, &mut state);
    update_status(&state);
    update_tab_layout_menu(hwnd, state.tab_layout);
    update_wrap_menu(hwnd, &state);
    update_editor_dark_menu(hwnd, state.editor_dark);
    update_always_on_top_menu(hwnd, &state);

    unsafe {
        let _ = SetTimer(hwnd, TIMER_SESSION_ID, SESSION_INTERVAL_MS, None);
    }

    Ok(state)
}

fn layout_children(hwnd: HWND, state: &mut AppState) {
    let mut rect = windows::Win32::Foundation::RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
        SendMessageW(state.status, WM_SIZE, WPARAM(0), LPARAM(0));
    }

    let mut status_rect = windows::Win32::Foundation::RECT::default();
    unsafe {
        let _ =
            windows::Win32::UI::WindowsAndMessaging::GetWindowRect(state.status, &mut status_rect);
    }

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let status_height = status_rect.bottom - status_rect.top;
    let tab_height = match state.tab_layout {
        TabLayout::HorizontalTop => tab_bar_height(state).max(0),
        TabLayout::VerticalLeft | TabLayout::VerticalRight => 0,
    };
    let list_width = match state.tab_layout {
        TabLayout::HorizontalTop => 0,
        TabLayout::VerticalLeft | TabLayout::VerticalRight => {
            let adjusted = clamp_tab_list_width(state, state.tab_list_width, width);
            state.tab_list_width = adjusted;
            adjusted
        }
    };
    let editor_height = (height - status_height - tab_height).max(0);
    let editor_width = match state.tab_layout {
        TabLayout::HorizontalTop => width.max(0),
        TabLayout::VerticalLeft | TabLayout::VerticalRight => {
            (width - list_width - TAB_SPLITTER_WIDTH).max(0)
        }
    };
    let (list_left, splitter_left, editor_left) = match state.tab_layout {
        TabLayout::HorizontalTop => (0, 0, 0),
        TabLayout::VerticalLeft => (0, list_width, list_width + TAB_SPLITTER_WIDTH),
        TabLayout::VerticalRight => {
            let editor_left = 0;
            let splitter_left = editor_width;
            let list_left = editor_width + TAB_SPLITTER_WIDTH;
            (list_left, splitter_left, editor_left)
        }
    };

    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
            state.tabs, 0, 0, width, tab_height, true,
        );
        let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
            state.tab_list,
            list_left,
            0,
            list_width,
            height - status_height,
            true,
        );
        let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
            state.tab_splitter,
            splitter_left,
            0,
            TAB_SPLITTER_WIDTH,
            height - status_height,
            true,
        );
        let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
            state.status,
            0,
            height - status_height,
            width,
            status_height,
            true,
        );
        for doc in &state.docs {
            let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
                doc.editor,
                editor_left,
                tab_height,
                editor_width,
                editor_height,
                true,
            );
        }
    }
    update_status_parts(state);
}

fn open_from_dialog(hwnd: HWND, state: &mut AppState) -> Result<()> {
    if let Some(path) = open_file_dialog(hwnd)? {
        open_path_new_tab(hwnd, state, path, None, None, None, None)?;
    }
    Ok(())
}

fn open_from_drop(hwnd: HWND, state: &mut AppState, hdrop: HDROP) -> Result<()> {
    let count = unsafe { DragQueryFileW(hdrop, 0xFFFF_FFFF, None) };
    if count == 0 {
        unsafe {
            DragFinish(hdrop);
        }
        return Ok(());
    }

    let len = unsafe { DragQueryFileW(hdrop, 0, None) };
    let mut buffer = vec![0u16; (len + 1) as usize];
    let copied = unsafe { DragQueryFileW(hdrop, 0, Some(buffer.as_mut_slice())) };
    unsafe {
        DragFinish(hdrop);
    }

    if copied == 0 {
        return Err(AppError::new("Failed to read dropped file path."));
    }

    let path = PathBuf::from(wide_to_string(&buffer)?);
    open_path_new_tab(hwnd, state, path, None, None, None, None)?;
    Ok(())
}

fn open_path_new_tab(
    hwnd: HWND,
    state: &mut AppState,
    path: PathBuf,
    caret: Option<usize>,
    wrap: Option<bool>,
    encoding: Option<TextEncoding>,
    eol: Option<Eol>,
) -> Result<()> {
    let instance = module_instance()?;
    let mut doc_tab = create_doc_from_path(hwnd, instance, path)?;
    if let Some(encoding) = encoding {
        doc_tab.doc.encoding = encoding;
    }
    if let Some(eol) = eol {
        doc_tab.doc.eol = eol;
        scintilla::set_eol_mode(doc_tab.editor, eol);
    }
    if let Some(wrap) = wrap {
        doc_tab.wrap_enabled = wrap;
        let enable = wrap && !doc_tab.doc.large_file_mode;
        scintilla::set_wrap_enabled(doc_tab.editor, enable);
    }

    apply_syntax_for_doc(&doc_tab, state.editor_dark);
    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    if let Some(caret) = caret {
        let editor = state.docs[index].editor;
        scintilla::goto_pos(editor, caret);
    }
    Ok(())
}

fn save_document(
    hwnd: HWND,
    state: &mut AppState,
    encoding_override: Option<TextEncoding>,
    force_save_as: bool,
) -> Result<()> {
    let index = state.active;
    let _ = save_document_at(hwnd, state, index, encoding_override, force_save_as)?;
    Ok(())
}

fn save_all_documents(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let mut index = 0usize;
    while index < state.docs.len() {
        let dirty = state
            .docs
            .get(index)
            .map(|doc_tab| scintilla::is_modified(doc_tab.editor))
            .unwrap_or(false);
        if dirty {
            match save_document_at(hwnd, state, index, None, false)? {
                true => {}
                false => return Ok(()),
            }
        }
        index += 1;
    }
    Ok(())
}

fn save_document_at(
    hwnd: HWND,
    state: &mut AppState,
    index: usize,
    encoding_override: Option<TextEncoding>,
    force_save_as: bool,
) -> Result<bool> {
    let doc_tab = state
        .docs
        .get_mut(index)
        .ok_or_else(|| AppError::new("No active document."))?;
    let encoding = encoding_override.unwrap_or(doc_tab.doc.encoding);
    let path = if force_save_as || doc_tab.doc.path.is_none() {
        match save_file_dialog(hwnd)? {
            Some(path) => path,
            None => return Ok(false),
        }
    } else {
        doc_tab
            .doc
            .path
            .clone()
            .ok_or_else(|| AppError::new("No file path available."))?
    };

    let text = scintilla::get_text(doc_tab.editor)?;
    let normalized = document::normalize_eol(&text, doc_tab.doc.eol);
    let bytes = document::encode_text(&normalized, encoding)?;

    std::fs::write(&path, &bytes)
        .map_err(|err| AppError::new(format!("Failed to write file: {err}")))?;

    let stamp = document::FileStamp::from_path(&path)?;
    doc_tab.doc.path = Some(path);
    doc_tab.display_name = None;
    doc_tab
        .doc
        .update_after_save(encoding, doc_tab.doc.eol, stamp);
    doc_tab.doc.large_file_mode = document::is_large_file(
        doc_tab
            .doc
            .stamp
            .as_ref()
            .map(|stamp| stamp.size)
            .unwrap_or(0),
    );
    doc_tab.word_count = if doc_tab.doc.large_file_mode {
        None
    } else {
        Some(count_words(&text))
    };
    doc_tab.wrap_enabled = doc_tab.wrap_enabled && !doc_tab.doc.large_file_mode;
    scintilla::set_wrap_enabled(doc_tab.editor, doc_tab.wrap_enabled);
    apply_syntax_for_doc(doc_tab, state.editor_dark);
    scintilla::set_savepoint(doc_tab.editor);
    doc_tab.dirty = false;
    update_tab_text(state, index);
    update_title(hwnd, state);
    update_status(state);
    Ok(true)
}

fn check_external_change(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let index = state.active;
    let path = match state
        .docs
        .get(index)
        .and_then(|doc_tab| doc_tab.doc.path.clone())
    {
        Some(path) => path,
        None => return Ok(()),
    };
    let stamp = state
        .docs
        .get(index)
        .and_then(|doc_tab| doc_tab.doc.stamp.clone());

    if let Some(new_stamp) = document::check_stamp(&path, &stamp)? {
        if prompt_reload(hwnd) {
            {
                let doc_tab = match state.docs.get_mut(index) {
                    Some(doc_tab) => doc_tab,
                    None => return Ok(()),
                };
                load_file_into_doc(doc_tab, &path)?;
                apply_syntax_for_doc(doc_tab, state.editor_dark);
            }
            update_tab_text(state, index);
            update_title(hwnd, state);
            update_status(state);
        } else if let Some(doc_tab) = state.docs.get_mut(index) {
            doc_tab.doc.stamp = Some(new_stamp);
        }
    }

    Ok(())
}

fn prompt_reload(hwnd: HWND) -> bool {
    let title = HSTRING::from("File changed on disk");
    let message = HSTRING::from("The file has changed on disk. Reload?");
    let result = unsafe {
        MessageBoxW(
            hwnd,
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_YESNO | MB_ICONWARNING,
        )
    };
    result == IDYES
}

fn update_title(hwnd: HWND, state: &AppState) {
    let mut title = String::from("Rivet");
    if let Some(doc_tab) = state.docs.get(state.active) {
        if let Some(path) = &doc_tab.doc.path
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
        {
            title = format!("Rivet - {name}");
        }
        if doc_tab.doc.large_file_mode {
            title.push_str(" [Large File Mode]");
        }
    }
    let title = HSTRING::from(title);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR::from_raw(title.as_ptr()));
    }
}

fn update_status(state: &AppState) {
    update_status_text(state);
    update_status_position(state);
}

fn update_status_text(state: &AppState) {
    let mut text = String::from("Ready");
    if let Some(doc_tab) = state.docs.get(state.active) {
        if let Some(path) = &doc_tab.doc.path {
            text = path.display().to_string();
        }
        text.push_str(" | ");
        text.push_str(encoding_label(doc_tab.doc.encoding));
        text.push_str(" | ");
        text.push_str(eol_label(doc_tab.doc.eol));
        if doc_tab.doc.large_file_mode {
            text.push_str(" | Large File Mode");
        }
    }

    let text = HSTRING::from(text);
    unsafe {
        SendMessageW(
            state.status,
            SB_SETTEXTW,
            WPARAM(0),
            LPARAM(text.as_ptr() as isize),
        );
    }
}

fn update_status_position(state: &AppState) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut words = String::from("0");
    if let Some(doc_tab) = state.docs.get(state.active) {
        let pos = scintilla::get_current_pos(doc_tab.editor);
        line = scintilla::line_from_position(doc_tab.editor, pos).saturating_add(1);
        col = scintilla::get_column(doc_tab.editor, pos).saturating_add(1);
        words = match doc_tab.word_count {
            Some(count) => count.to_string(),
            None => "n/a".to_string(),
        };
    }

    let text = HSTRING::from(format!("Ln {line}, Col {col} | Words: {words}"));
    unsafe {
        SendMessageW(
            state.status,
            SB_SETTEXTW,
            WPARAM(1),
            LPARAM(text.as_ptr() as isize),
        );
    }
}

fn update_status_parts(state: &AppState) {
    let mut rect = windows::Win32::Foundation::RECT::default();
    unsafe {
        let _ = GetClientRect(state.status, &mut rect);
    }
    let width = rect.right - rect.left;
    let right_width = scale_for_dpi(state.status, 240);
    let right_edge = (width - right_width).max(0);
    let parts = [right_edge, -1];
    unsafe {
        SendMessageW(
            state.status,
            SB_SETPARTS,
            WPARAM(parts.len()),
            LPARAM(parts.as_ptr() as isize),
        );
    }
}

fn schedule_word_count(hwnd: HWND, state: &mut AppState) {
    if state
        .docs
        .get(state.active)
        .is_some_and(|doc_tab| !doc_tab.doc.large_file_mode)
    {
        state.word_count_pending = true;
    }
    if !state.word_count_timer && state.word_count_pending {
        unsafe {
            let _ = SetTimer(hwnd, TIMER_WORD_COUNT, WORD_COUNT_INTERVAL_MS, None);
        }
        state.word_count_timer = true;
    }
}

fn handle_word_count_timer(hwnd: HWND, state: &mut AppState) {
    if state.word_count_pending {
        state.word_count_pending = false;
        update_word_count(state);
        update_status_position(state);
    }
    if state.word_count_timer && !state.word_count_pending {
        unsafe {
            let _ = KillTimer(hwnd, TIMER_WORD_COUNT);
        }
        state.word_count_timer = false;
    }
}

fn update_word_count(state: &mut AppState) {
    let index = state.active;
    let doc_tab = match state.docs.get_mut(index) {
        Some(doc_tab) => doc_tab,
        None => return,
    };
    if doc_tab.doc.large_file_mode {
        doc_tab.word_count = None;
        return;
    }
    let text = match scintilla::get_text(doc_tab.editor) {
        Ok(text) => text,
        Err(_) => return,
    };
    doc_tab.word_count = Some(count_words(&text));
}

fn active_editor(state: &AppState) -> Option<HWND> {
    state.docs.get(state.active).map(|doc_tab| doc_tab.editor)
}

fn current_document_path(state: &AppState) -> Option<&Path> {
    state
        .docs
        .get(state.active)
        .and_then(|doc_tab| doc_tab.doc.path.as_deref())
}

fn transform_selection_case(editor: HWND, uppercase: bool) {
    let start = scintilla::selection_start(editor);
    let end = scintilla::selection_end(editor);
    let can_transform = if uppercase {
        can_uppercase(start as i64, end as i64)
    } else {
        can_lowercase(start as i64, end as i64)
    };
    if !can_transform {
        return;
    }
    if uppercase {
        scintilla::uppercase_selection(editor);
    } else {
        scintilla::lowercase_selection(editor);
    }
}

fn trim_leading_and_trailing_whitespace(editor: HWND) {
    let sel_start = scintilla::selection_start(editor);
    let sel_end = scintilla::selection_end(editor);
    let line_count = scintilla::line_count(editor);
    if line_count == 0 {
        return;
    }

    let (start_line, end_line) = if sel_start != sel_end {
        let start_line = scintilla::line_from_position(editor, sel_start);
        let mut end_line = scintilla::line_from_position(editor, sel_end);
        if sel_end > sel_start {
            let line_start = scintilla::position_from_line(editor, end_line);
            if sel_end == line_start && end_line > start_line {
                end_line = end_line.saturating_sub(1);
            }
        }
        (start_line, end_line)
    } else {
        (0, line_count.saturating_sub(1))
    };

    if start_line > end_line {
        return;
    }

    scintilla::begin_undo_action(editor);
    for line in (start_line..=end_line).rev() {
        trim_line_whitespace(editor, line);
    }
    scintilla::end_undo_action(editor);
}

fn trim_line_whitespace(editor: HWND, line: usize) {
    let line_start = scintilla::position_from_line(editor, line);
    let line_end = scintilla::line_end_position(editor, line);
    if line_end <= line_start {
        return;
    }

    let mut raw = Vec::with_capacity(line_end - line_start);
    for pos in line_start..line_end {
        raw.push(scintilla::char_at(editor, pos));
    }
    let line_text = String::from_utf8_lossy(&raw);
    if trim_line_preserve_eol(&line_text) == line_text {
        return;
    }
    let (trim_left, trim_right) = trim_edges_spaces_tabs(&line_text);

    if trim_right > 0 {
        scintilla::set_target_range(editor, line_end - trim_right, line_end);
        scintilla::replace_target_empty(editor);
    }
    if trim_left > 0 {
        scintilla::set_target_range(editor, line_start, line_start + trim_left);
        scintilla::replace_target_empty(editor);
    }
}

fn copy_path_to_clipboard(hwnd: HWND, state: &AppState, kind: CopyPathKind) -> Result<()> {
    let mut clipboard = WinClipboard::new(hwnd);
    let _ = match kind {
        CopyPathKind::FullPath => copy_full_path(current_document_path(state), &mut clipboard),
        CopyPathKind::FileName => copy_filename(current_document_path(state), &mut clipboard),
        CopyPathKind::DirectoryPath => {
            copy_directory_path(current_document_path(state), &mut clipboard)
        }
    }
    .map_err(|err| AppError::new(format!("Copy path failed: {err}")))?;
    Ok(())
}

fn show_find_dialog(hwnd: HWND, state: &mut AppState, find_only: bool) -> Result<()> {
    if let Some(dialog) = &state.find_dialog {
        unsafe {
            ShowWindow(dialog.hwnd, SW_SHOW);
        }
        apply_find_state_to_dialog(state)?;
        return Ok(());
    }

    let instance = module_instance()?;
    let title = if find_only {
        w!("Find")
    } else {
        w!("Find / Replace")
    };
    let width = scale_for_dpi(hwnd, 460);
    let height = scale_for_dpi(hwnd, 240);
    let hwnd_dialog = unsafe {
        CreateWindowExW(
            Default::default(),
            FIND_CLASS,
            title,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            hwnd,
            HMENU(0),
            instance,
            Some(hwnd.0 as *const std::ffi::c_void),
        )
    };

    if hwnd_dialog.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(FindDialog)"));
    }

    Ok(())
}

fn show_find_in_files_dialog(hwnd: HWND, state: &mut AppState) -> Result<()> {
    if let Some(dialog) = &state.find_in_files {
        unsafe {
            ShowWindow(dialog.hwnd, SW_SHOW);
        }
        return Ok(());
    }

    let instance = module_instance()?;
    let width = scale_for_dpi(hwnd, 680);
    let height = scale_for_dpi(hwnd, 420);
    let hwnd_dialog = unsafe {
        CreateWindowExW(
            Default::default(),
            FIND_FILES_CLASS,
            w!("Find in Files"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            hwnd,
            HMENU(0),
            instance,
            Some(hwnd.0 as *const std::ffi::c_void),
        )
    };

    if hwnd_dialog.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(FindInFiles)"));
    }

    Ok(())
}

fn perform_find_next(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_find_state_from_dialog(state)?;
    if state.find_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, true)?;
        return Ok(());
    }
    if let Some(editor) = active_editor(state) {
        find_in_editor(editor, &state.find_state, true);
    }
    Ok(())
}

fn perform_find_prev(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_find_state_from_dialog(state)?;
    if state.find_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, true)?;
        return Ok(());
    }
    if let Some(editor) = active_editor(state) {
        find_in_editor(editor, &state.find_state, false);
    }
    Ok(())
}

fn perform_replace(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_find_state_from_dialog(state)?;
    if state.find_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, false)?;
        return Ok(());
    }
    if let Some(editor) = active_editor(state) {
        replace_in_editor(editor, &state.find_state);
    }
    Ok(())
}

fn perform_replace_all(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_find_state_from_dialog(state)?;
    if state.find_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, false)?;
        return Ok(());
    }
    if let Some(editor) = active_editor(state) {
        replace_all_in_editor(editor, &state.find_state);
    }
    Ok(())
}

fn sync_find_state_from_dialog(state: &mut AppState) -> Result<()> {
    if let Some(dialog) = &state.find_dialog {
        state.find_state.find_text = get_window_text(dialog.find_edit)?;
        state.find_state.replace_text = get_window_text(dialog.replace_edit)?;
        state.find_state.match_case = is_checked(dialog.match_case);
        state.find_state.whole_word = is_checked(dialog.whole_word);
        state.find_state.regex = is_checked(dialog.regex);
        state.find_state.wrap = is_checked(dialog.wrap);
    }
    Ok(())
}

fn apply_find_state_to_dialog(state: &AppState) -> Result<()> {
    if let Some(dialog) = &state.find_dialog {
        set_window_text(dialog.find_edit, &state.find_state.find_text);
        set_window_text(dialog.replace_edit, &state.find_state.replace_text);
        set_checked(dialog.match_case, state.find_state.match_case);
        set_checked(dialog.whole_word, state.find_state.whole_word);
        set_checked(dialog.regex, state.find_state.regex);
        set_checked(dialog.wrap, state.find_state.wrap);
    }
    Ok(())
}

fn find_in_editor(editor: HWND, state: &FindState, forward: bool) {
    let flags = search_flags(state);
    if forward {
        let _ = scintilla::search_next(editor, &state.find_text, flags, state.wrap);
    } else {
        let _ = scintilla::search_prev(editor, &state.find_text, flags, state.wrap);
    }
}

fn replace_in_editor(editor: HWND, state: &FindState) {
    let flags = search_flags(state);
    if scintilla::search_next(editor, &state.find_text, flags, state.wrap) {
        scintilla::replace_selection(editor, &state.replace_text);
    }
}

fn replace_all_in_editor(editor: HWND, state: &FindState) {
    let flags = search_flags(state);
    scintilla::set_selection(editor, 0, 0);
    while scintilla::search_next(editor, &state.find_text, flags, false) {
        scintilla::replace_selection(editor, &state.replace_text);
    }
}

fn search_flags(state: &FindState) -> usize {
    let mut flags = 0;
    if state.match_case {
        flags |= SCFIND_MATCHCASE;
    }
    if state.whole_word {
        flags |= SCFIND_WHOLEWORD;
    }
    if state.regex {
        flags |= SCFIND_REGEXP;
    }
    flags
}

fn is_checked(hwnd: HWND) -> bool {
    let value = unsafe { SendMessageW(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as u32 };
    value != 0
}

fn set_checked(hwnd: HWND, checked: bool) {
    let value = if checked { 1 } else { 0 };
    unsafe {
        SendMessageW(hwnd, BM_SETCHECK, WPARAM(value), LPARAM(0));
    }
}

fn set_window_text(hwnd: HWND, text: &str) {
    let text = HSTRING::from(text);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR::from_raw(text.as_ptr()));
    }
}

fn get_window_text(hwnd: HWND) -> Result<String> {
    let len = unsafe { GetWindowTextLengthW(hwnd) } as usize;
    let mut buffer = vec![0u16; len + 1];
    unsafe {
        let _ = GetWindowTextW(hwnd, buffer.as_mut_slice());
    }
    wide_to_string(&buffer)
}

fn start_find_in_files(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let dialog = match state.find_in_files.as_mut() {
        Some(dialog) => dialog,
        None => return Ok(()),
    };

    let find_text = get_window_text(dialog.find_edit)?;
    if find_text.trim().is_empty() {
        return Ok(());
    }
    let folder_text = get_window_text(dialog.folder_edit)?;
    if folder_text.trim().is_empty() {
        return Err(AppError::new("Folder is required for Find in Files."));
    }

    let include_text = get_window_text(dialog.include_edit)?;
    let exclude_text = get_window_text(dialog.exclude_edit)?;

    let options = FindInFilesOptions {
        find_text,
        folder: PathBuf::from(folder_text),
        include: parse_patterns(&include_text),
        exclude: parse_patterns(&exclude_text),
        match_case: is_checked(dialog.match_case),
        whole_word: is_checked(dialog.whole_word),
        regex: is_checked(dialog.regex),
        recurse: is_checked(dialog.recurse),
    };

    dialog.hits.clear();
    unsafe {
        SendMessageW(dialog.results, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
    }

    let (tx, rx) = mpsc::channel();
    dialog.receiver = Some(rx);
    dialog.cancel.store(false, Ordering::SeqCst);
    dialog.running = true;

    let cancel = dialog.cancel.clone();
    std::thread::spawn(move || {
        run_find_in_files(options, tx, cancel);
    });

    unsafe {
        let _ = SetTimer(hwnd, TIMER_FIND_RESULTS, 100, None);
    }

    Ok(())
}

fn cancel_find_in_files(state: &mut AppState) {
    if let Some(dialog) = &mut state.find_in_files {
        dialog.cancel.store(true, Ordering::SeqCst);
    }
}

fn poll_find_results(hwnd: HWND, state: &mut AppState) {
    let dialog = match state.find_in_files.as_mut() {
        Some(dialog) => dialog,
        None => return,
    };
    let receiver = match dialog.receiver.as_mut() {
        Some(receiver) => receiver,
        None => return,
    };

    loop {
        match receiver.try_recv() {
            Ok(FindResult::Match(hit)) => {
                let label = format!("{}({}): {}", hit.path.display(), hit.line, hit.text);
                dialog.hits.push(hit);
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                unsafe {
                    SendMessageW(
                        dialog.results,
                        LB_ADDSTRING,
                        WPARAM(0),
                        LPARAM(wide.as_ptr() as isize),
                    );
                }
            }
            Ok(FindResult::Done) => {
                dialog.running = false;
                dialog.receiver = None;
                unsafe {
                    let _ = KillTimer(hwnd, TIMER_FIND_RESULTS);
                }
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                dialog.running = false;
                dialog.receiver = None;
                unsafe {
                    let _ = KillTimer(hwnd, TIMER_FIND_RESULTS);
                }
                break;
            }
        }
    }
}

fn open_find_result(hwnd: HWND, state: &mut AppState, index: usize) {
    let (path, line) = match state
        .find_in_files
        .as_ref()
        .and_then(|dialog| dialog.hits.get(index))
    {
        Some(hit) => (hit.path.clone(), hit.line),
        None => return,
    };

    if open_path_new_tab(hwnd, state, path, None, None, None, None).is_ok()
        && let Some(editor) = active_editor(state)
    {
        scintilla::goto_line(editor, line.saturating_sub(1));
    }
}

fn browse_for_folder(owner: HWND) -> Option<PathBuf> {
    let mut buffer = [0u16; 260];
    let info = BROWSEINFOW {
        hwndOwner: owner,
        lpszTitle: w!("Select folder"),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        ..Default::default()
    };

    let pidl = unsafe { SHBrowseForFolderW(&info) };
    if pidl.is_null() {
        return None;
    }
    let ok = unsafe { SHGetPathFromIDListW(pidl, &mut buffer) };
    unsafe {
        CoTaskMemFree(Some(pidl as _));
    }
    if !ok.as_bool() {
        return None;
    }
    wide_to_string(&buffer).ok().map(PathBuf::from)
}

#[derive(Clone)]
struct FindInFilesOptions {
    find_text: String,
    folder: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    match_case: bool,
    whole_word: bool,
    regex: bool,
    recurse: bool,
}

fn run_find_in_files(
    options: FindInFilesOptions,
    sender: mpsc::Sender<FindResult>,
    cancel: Arc<AtomicBool>,
) {
    let regex = if options.regex {
        let pattern = if options.whole_word {
            format!(r"\b(?:{})\b", options.find_text)
        } else {
            options.find_text.clone()
        };
        RegexBuilder::new(&pattern)
            .case_insensitive(!options.match_case)
            .build()
            .ok()
    } else {
        None
    };

    let mut stack = vec![options.folder.clone()];
    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                if options.recurse {
                    stack.push(path);
                }
                continue;
            }
            if !matches_patterns(&options.include, &path, true) {
                continue;
            }
            if matches_patterns(&options.exclude, &path, false) {
                continue;
            }
            search_file(&path, &options, &regex, &sender, &cancel);
        }
    }

    let _ = sender.send(FindResult::Done);
}

fn search_file(
    path: &PathBuf,
    options: &FindInFilesOptions,
    regex: &Option<regex::Regex>,
    sender: &mpsc::Sender<FindResult>,
    cancel: &Arc<AtomicBool>,
) {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();
    let mut line_no = 1usize;
    let mut first_line = true;

    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        buffer.clear();
        let read = match reader.read_until(b'\n', &mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }
        if first_line {
            first_line = false;
            if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
                buffer.drain(0..3);
            }
        }
        let line = String::from_utf8_lossy(&buffer);
        let line_trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if line_matches(line_trimmed, options, regex) {
            let preview = trim_preview(line_trimmed);
            let _ = sender.send(FindResult::Match(FindHit {
                path: path.clone(),
                line: line_no,
                text: preview,
            }));
        }
        line_no += 1;
    }
}

fn line_matches(line: &str, options: &FindInFilesOptions, regex: &Option<regex::Regex>) -> bool {
    if let Some(re) = regex {
        return re.is_match(line);
    }

    if options.match_case {
        match_substring(line, &options.find_text, options.whole_word)
    } else {
        let line_lower = line.to_lowercase();
        let needle_lower = options.find_text.to_lowercase();
        match_substring(&line_lower, &needle_lower, options.whole_word)
    }
}

fn match_substring(line: &str, needle: &str, whole_word: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    if !whole_word {
        return line.contains(needle);
    }
    for (idx, _) in line.match_indices(needle) {
        let before = line[..idx].chars().next_back();
        let after = line[idx + needle.len()..].chars().next();
        let before_ok = before.is_none_or(|ch| !is_word_char(ch));
        let after_ok = after.is_none_or(|ch| !is_word_char(ch));
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn count_words(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_word = false;
    for ch in text.chars() {
        if is_word_char(ch) {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    count
}

fn trim_preview(line: &str) -> String {
    let mut out = line.trim().to_string();
    if out.len() > 200 {
        out.truncate(200);
        out.push_str("...");
    }
    out
}

fn parse_patterns(value: &str) -> Vec<String> {
    value
        .split(&[';', ','][..])
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn matches_patterns(patterns: &[String], path: &Path, include_if_empty: bool) -> bool {
    if patterns.is_empty() {
        return include_if_empty;
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    patterns.iter().any(|pattern| wildcard_match(pattern, name))
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            if p[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p[i - 1] == '?' || p[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }
    dp[p.len()][t.len()]
}

fn module_instance() -> Result<HINSTANCE> {
    let instance: HINSTANCE = unsafe { GetModuleHandleW(None) }?.into();
    Ok(instance)
}

fn create_editor(parent: HWND, instance: HINSTANCE) -> Result<HWND> {
    let editor = scintilla::create_window(parent, instance)?;
    scintilla::initialize(editor);
    Ok(editor)
}

fn apply_syntax_for_doc(doc_tab: &DocTab, dark: bool) {
    let lexer = lexer_for_doc(&doc_tab.doc);
    scintilla::apply_lexer(doc_tab.editor, lexer, dark);
}

fn lexer_for_doc(doc: &Document) -> scintilla::LexerKind {
    if doc.large_file_mode {
        return scintilla::LexerKind::Null;
    }
    let ext = doc
        .path
        .as_ref()
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    match ext.as_deref() {
        Some("c") | Some("h") | Some("cc") | Some("cpp") | Some("cxx") | Some("hpp")
        | Some("hxx") | Some("hh") => scintilla::LexerKind::Cpp,
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") | Some("ts") | Some("tsx") => {
            scintilla::LexerKind::JavaScript
        }
        Some("json") | Some("jsonc") => scintilla::LexerKind::Json,
        Some("yaml") | Some("yml") => scintilla::LexerKind::Yaml,
        Some("ps1") | Some("psm1") | Some("psd1") => scintilla::LexerKind::PowerShell,
        Some("py") | Some("pyw") => scintilla::LexerKind::Python,
        Some("html") | Some("htm") => scintilla::LexerKind::Html,
        Some("xml") | Some("xaml") => scintilla::LexerKind::Xml,
        Some("css") => scintilla::LexerKind::Css,
        Some("ini") | Some("cfg") | Some("conf") | Some("properties") => {
            scintilla::LexerKind::Properties
        }
        _ => scintilla::LexerKind::Null,
    }
}

fn create_doc_from_path(parent: HWND, instance: HINSTANCE, path: PathBuf) -> Result<DocTab> {
    let editor = create_editor(parent, instance)?;
    let mut doc_tab = DocTab {
        editor,
        doc: Document::new_empty(),
        display_name: None,
        dirty: false,
        wrap_enabled: true,
        word_count: Some(0),
    };
    load_file_into_doc(&mut doc_tab, &path)?;
    Ok(doc_tab)
}

fn load_file_into_doc(doc_tab: &mut DocTab, path: &PathBuf) -> Result<()> {
    let bytes =
        std::fs::read(path).map_err(|err| AppError::new(format!("Failed to read file: {err}")))?;
    let (text, encoding) = document::decode_bytes(&bytes)?;
    let eol = document::detect_eol(&text);
    let stamp = document::FileStamp::from_path(path)?;
    let large_file_mode = document::is_large_file(stamp.size);

    scintilla::set_text(doc_tab.editor, &text)?;
    scintilla::set_eol_mode(doc_tab.editor, eol);

    doc_tab.wrap_enabled = doc_tab.wrap_enabled && !large_file_mode;
    scintilla::set_wrap_enabled(doc_tab.editor, doc_tab.wrap_enabled);
    scintilla::set_savepoint(doc_tab.editor);

    doc_tab
        .doc
        .update_from_load(path.clone(), encoding, eol, stamp, large_file_mode);
    doc_tab.display_name = None;
    doc_tab.dirty = false;
    doc_tab.word_count = if large_file_mode {
        None
    } else {
        Some(count_words(&text))
    };
    Ok(())
}

fn create_empty_tab(hwnd: HWND, instance: HINSTANCE, state: &mut AppState) -> Result<()> {
    let editor = create_editor(hwnd, instance)?;
    let doc_tab = DocTab {
        editor,
        doc: Document::new_empty(),
        display_name: None,
        dirty: false,
        wrap_enabled: true,
        word_count: Some(0),
    };
    scintilla::set_eol_mode(editor, doc_tab.doc.eol);
    scintilla::set_wrap_enabled(editor, true);
    scintilla::set_savepoint(editor);
    apply_syntax_for_doc(&doc_tab, state.editor_dark);

    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    Ok(())
}

fn duplicate_active_tab(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let source = state
        .docs
        .get(state.active)
        .ok_or_else(|| AppError::new("No active document."))?;
    let text = scintilla::get_text(source.editor)?;
    let instance = module_instance()?;
    let editor = create_editor(hwnd, instance)?;
    scintilla::set_text(editor, &text)?;
    scintilla::set_eol_mode(editor, source.doc.eol);
    scintilla::set_wrap_enabled(editor, source.wrap_enabled);

    let mut doc = Document::new_empty();
    doc.encoding = source.doc.encoding;
    doc.eol = source.doc.eol;
    doc.large_file_mode = document::is_large_file(text.len() as u64);

    let copy_name = format!("Copy of {}", tab_base_name(source));
    let doc_tab = DocTab {
        editor,
        doc,
        display_name: Some(copy_name),
        dirty: true,
        wrap_enabled: source.wrap_enabled && !source.doc.large_file_mode,
        word_count: if source.doc.large_file_mode {
            None
        } else {
            Some(count_words(&text))
        },
    };
    let lexer = lexer_for_doc(&source.doc);
    scintilla::apply_lexer(doc_tab.editor, lexer, state.editor_dark);

    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    Ok(())
}

fn tab_base_name(doc_tab: &DocTab) -> String {
    if let Some(display_name) = &doc_tab.display_name {
        display_name.clone()
    } else if let Some(path) = &doc_tab.doc.path {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_string()
    } else {
        "Untitled".to_string()
    }
}

fn tab_title(doc_tab: &DocTab) -> String {
    let mut title = tab_base_name(doc_tab);
    if doc_tab.dirty {
        title = format!("• {title}");
    }
    title
}

fn add_tab(state: &mut AppState, title: &str, doc_tab: DocTab) -> Result<usize> {
    let index = state.docs.len();
    insert_tab_item(state.tabs, index, title)?;
    state.docs.push(doc_tab);
    rebuild_tab_list(state);
    Ok(index)
}

fn insert_tab_item(tabs: HWND, index: usize, title: &str) -> Result<()> {
    let mut buffer: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let mut item = TCITEMW {
        mask: TCIF_TEXT,
        pszText: PWSTR(buffer.as_mut_ptr()),
        cchTextMax: buffer.len() as i32,
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            tabs,
            TCM_INSERTITEMW,
            WPARAM(index),
            LPARAM(&mut item as *mut TCITEMW as isize),
        );
    }
    Ok(())
}

fn update_tab_text(state: &mut AppState, index: usize) {
    if let Some(doc_tab) = state.docs.get(index) {
        let title = tab_title(doc_tab);
        let mut buffer: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            pszText: PWSTR(buffer.as_mut_ptr()),
            cchTextMax: buffer.len() as i32,
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                state.tabs,
                TCM_SETITEMW,
                WPARAM(index),
                LPARAM(&mut item as *mut TCITEMW as isize),
            );
        }
        rebuild_tab_list(state);
    }
}

fn rebuild_tab_list(state: &mut AppState) {
    unsafe {
        SendMessageW(state.tab_list, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
    }
    for doc_tab in &state.docs {
        let title = tab_title(doc_tab);
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            SendMessageW(
                state.tab_list,
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide.as_ptr() as isize),
            );
        }
    }
    set_tab_list_selection(state, state.active);
}

fn set_tab_list_selection(state: &AppState, index: usize) {
    if state.docs.is_empty() || index >= state.docs.len() {
        return;
    }
    unsafe {
        SendMessageW(state.tab_list, LB_SETCURSEL, WPARAM(index), LPARAM(0));
    }
}

fn select_tab(hwnd: HWND, state: &mut AppState, index: usize) {
    if index >= state.docs.len() {
        return;
    }

    state.active = index;
    let dirty = state
        .docs
        .get(index)
        .map(|doc_tab| scintilla::is_modified(doc_tab.editor))
        .unwrap_or(false);
    if let Some(doc_tab) = state.docs.get_mut(index) {
        doc_tab.dirty = dirty;
    }
    update_tab_text(state, index);
    unsafe {
        SendMessageW(state.tabs, TCM_SETCURSEL, WPARAM(index), LPARAM(0));
    }
    set_tab_list_selection(state, index);

    for (i, doc) in state.docs.iter().enumerate() {
        let show = if i == index { SW_SHOW } else { SW_HIDE };
        unsafe {
            ShowWindow(doc.editor, show);
        }
    }
    if let Some(doc_tab) = state.docs.get(index) {
        unsafe {
            SetFocus(doc_tab.editor);
        }
    }

    update_title(hwnd, state);
    update_status(state);
    update_wrap_menu(hwnd, state);
    update_copy_path_menu(hwnd, state);
    layout_children(hwnd, state);
}

fn tab_bar_height(state: &AppState) -> i32 {
    let min_height = scale_for_dpi(state.tabs, 26);
    if state.docs.is_empty() {
        return min_height;
    }
    let mut rect = windows::Win32::Foundation::RECT::default();
    let result = unsafe {
        SendMessageW(
            state.tabs,
            TCM_GETITEMRECT,
            WPARAM(0),
            LPARAM(&mut rect as *mut _ as isize),
        )
    };
    if result.0 == 0 {
        min_height
    } else {
        (rect.bottom - rect.top).max(min_height)
    }
}

fn tab_index_at_point(tabs: HWND, count: usize, point: LPARAM) -> Option<usize> {
    let x = lparam_x(point);
    let y = lparam_y(point);
    let mut rect = windows::Win32::Foundation::RECT::default();
    for index in 0..count {
        let result = unsafe {
            SendMessageW(
                tabs,
                TCM_GETITEMRECT,
                WPARAM(index),
                LPARAM(&mut rect as *mut _ as isize),
            )
        };
        if result.0 != 0 && x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
            return Some(index);
        }
    }
    None
}

fn doc_index_by_hwnd(state: &AppState, hwnd: HWND) -> Option<usize> {
    state.docs.iter().position(|doc| doc.editor == hwnd)
}

fn set_dirty(state: &mut AppState, index: usize, dirty: bool) {
    if let Some(doc_tab) = state.docs.get_mut(index)
        && doc_tab.dirty != dirty
    {
        doc_tab.dirty = dirty;
        update_tab_text(state, index);
    }
}

fn confirm_close_all(hwnd: HWND, state: &mut AppState) -> Result<bool> {
    for index in 0..state.docs.len() {
        let doc_tab = &state.docs[index];
        if doc_tab.dirty {
            match prompt_save_changes(hwnd, doc_tab) {
                SaveChoice::Yes => match save_document_at(hwnd, state, index, None, false)? {
                    true => {}
                    false => return Ok(false),
                },
                SaveChoice::No => {}
                SaveChoice::Cancel => return Ok(false),
            }
        }
    }
    Ok(true)
}

fn close_tab(hwnd: HWND, state: &mut AppState, index: usize) -> Result<bool> {
    if index >= state.docs.len() {
        return Ok(true);
    }
    let should_close = {
        let doc_tab = &state.docs[index];
        if doc_tab.dirty {
            match prompt_save_changes(hwnd, doc_tab) {
                SaveChoice::Yes => match save_document_at(hwnd, state, index, None, false) {
                    Ok(true) => true,
                    Ok(false) => return Ok(false),
                    Err(err) => {
                        show_error("Rivet error", &err.to_string());
                        return Ok(false);
                    }
                },
                SaveChoice::No => true,
                SaveChoice::Cancel => false,
            }
        } else {
            true
        }
    };

    if !should_close {
        return Ok(false);
    }

    let doc = state.docs.remove(index);
    unsafe {
        let _ = DestroyWindow(doc.editor);
        SendMessageW(state.tabs, TCM_DELETEITEM, WPARAM(index), LPARAM(0));
    }

    if state.active > index {
        state.active = state.active.saturating_sub(1);
    }

    if state.docs.is_empty() {
        let instance = module_instance()?;
        create_empty_tab(hwnd, instance, state)?;
    }

    let new_index = state.active.min(state.docs.len().saturating_sub(1));
    select_tab(hwnd, state, new_index);
    Ok(true)
}

fn close_other_tabs(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let active = state.active;
    let mut index = state.docs.len();
    while index > 0 {
        index -= 1;
        if index != active && !close_tab(hwnd, state, index)? {
            return Ok(());
        }
    }
    Ok(())
}

fn close_tabs_to_left(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let active = state.active;
    if active == 0 || state.docs.len() <= 1 {
        return Ok(());
    }
    let mut index = active;
    while index > 0 {
        index -= 1;
        if !close_tab(hwnd, state, index)? {
            return Ok(());
        }
    }
    Ok(())
}

fn close_tabs_to_right(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let active = state.active;
    if active + 1 >= state.docs.len() {
        return Ok(());
    }
    let index = active + 1;
    while index < state.docs.len() {
        if !close_tab(hwnd, state, index)? {
            return Ok(());
        }
    }
    Ok(())
}

enum SaveChoice {
    Yes,
    No,
    Cancel,
}

fn prompt_save_changes(hwnd: HWND, doc_tab: &DocTab) -> SaveChoice {
    let name = doc_tab
        .doc
        .path
        .as_ref()
        .and_then(|path| path.file_name().and_then(|name| name.to_str()))
        .unwrap_or("Untitled");
    let message = HSTRING::from(format!("Save changes to {name}?"));
    let title = HSTRING::from("Unsaved changes");
    let result = unsafe {
        MessageBoxW(
            hwnd,
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_YESNOCANCEL | MB_ICONWARNING,
        )
    };
    match result {
        IDYES => SaveChoice::Yes,
        IDNO => SaveChoice::No,
        _ => SaveChoice::Cancel,
    }
}

fn restore_session(hwnd: HWND, mut state: AppState) -> Result<AppState> {
    let session = match session::load_session() {
        Ok(session) => session,
        Err(_) => return Ok(state),
    };

    for entry in session.entries {
        if open_path_new_tab(
            hwnd,
            &mut state,
            entry.path,
            Some(entry.caret),
            Some(entry.wrap),
            Some(entry.encoding),
            Some(entry.eol),
        )
        .is_err()
        {
            continue;
        }
    }

    if !state.docs.is_empty() {
        state.active = session.active.min(state.docs.len().saturating_sub(1));
    }

    Ok(state)
}

fn save_session_checkpoint(state: &AppState) -> Result<()> {
    let mut entries = Vec::new();
    let mut active_entry = 0usize;
    let mut entry_index = 0usize;
    for (doc_index, doc_tab) in state.docs.iter().enumerate() {
        if let Some(path) = &doc_tab.doc.path {
            let caret = scintilla::get_current_pos(doc_tab.editor);
            entries.push(session::SessionEntry {
                path: path.clone(),
                caret,
                encoding: doc_tab.doc.encoding,
                eol: doc_tab.doc.eol,
                wrap: doc_tab.wrap_enabled,
            });
            if doc_index == state.active {
                active_entry = entry_index;
            }
            entry_index += 1;
        }
    }

    let data = session::SessionData {
        active: active_entry,
        entries,
    };
    session::save_session(&data)
}

fn encoding_label(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "UTF-8",
        TextEncoding::Utf8Bom => "UTF-8 BOM",
        TextEncoding::Utf16Le => "UTF-16 LE",
        TextEncoding::Utf16Be => "UTF-16 BE",
    }
}

fn eol_label(eol: Eol) -> &'static str {
    match eol {
        Eol::Crlf => "CRLF",
        Eol::Lf => "LF",
    }
}

fn open_file_dialog(hwnd: HWND) -> Result<Option<PathBuf>> {
    let mut buffer = vec![0u16; 1024];
    let filter = w!("All Files\0*.*\0\0");

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        ..Default::default()
    };

    let result = unsafe { GetOpenFileNameW(&mut ofn) };
    if result.as_bool() {
        let path = PathBuf::from(wide_to_string(&buffer)?);
        return Ok(Some(path));
    }

    let error = unsafe { CommDlgExtendedError() };
    if error.0 == 0 {
        return Ok(None);
    }

    Err(AppError::new(format!(
        "GetOpenFileNameW failed with error code {}.",
        error.0
    )))
}

fn save_file_dialog(hwnd: HWND) -> Result<Option<PathBuf>> {
    let mut buffer = vec![0u16; 1024];
    let filter = w!("All Files\0*.*\0\0");

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT,
        ..Default::default()
    };

    let result = unsafe { GetSaveFileNameW(&mut ofn) };
    if result.as_bool() {
        let path = PathBuf::from(wide_to_string(&buffer)?);
        return Ok(Some(path));
    }

    let error = unsafe { CommDlgExtendedError() };
    if error.0 == 0 {
        return Ok(None);
    }

    Err(AppError::new(format!(
        "GetSaveFileNameW failed with error code {}.",
        error.0
    )))
}

fn wide_to_string(buffer: &[u16]) -> Result<String> {
    let len = buffer
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..len])
        .map_err(|err| AppError::new(format!("Failed to decode UTF-16 string from dialog: {err}")))
}

fn get_state(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *ptr })
    }
}

fn clamp_tab_list_width(state: &AppState, desired: i32, client_width: i32) -> i32 {
    let min_width = scale_for_dpi(state.tab_list, 24);
    let min_editor = scale_for_dpi(state.tab_list, 200);
    let max_width = (client_width - min_editor - TAB_SPLITTER_WIDTH).max(min_width);
    desired.clamp(min_width, max_width)
}

fn color_ref(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

fn tab_list_colors(dark: bool) -> (COLORREF, COLORREF) {
    if dark {
        (color_ref(212, 212, 212), color_ref(30, 30, 30))
    } else {
        (color_ref(32, 32, 32), color_ref(255, 255, 255))
    }
}

fn loword(value: usize) -> u16 {
    (value & 0xffff) as u16
}

fn hiword(value: usize) -> u16 {
    ((value >> 16) & 0xffff) as u16
}

fn lparam_x(lparam: LPARAM) -> i32 {
    (lparam.0 & 0xffff) as u16 as i16 as i32
}

fn lparam_y(lparam: LPARAM) -> i32 {
    ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32
}

fn context_menu_position(lparam: LPARAM) -> (i32, i32) {
    if lparam.0 == -1 {
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            (point.x, point.y)
        } else {
            (0, 0)
        }
    } else {
        (lparam_x(lparam), lparam_y(lparam))
    }
}

fn tab_hit_test_at_cursor(tabs: HWND) -> Option<(usize, i32, i32)> {
    let mut screen = POINT::default();
    if unsafe { GetCursorPos(&mut screen) }.is_err() {
        return None;
    }
    let mut client = screen;
    unsafe {
        let _ = ScreenToClient(tabs, &mut client);
    }
    let mut hit = TCHITTESTINFO {
        pt: client,
        ..Default::default()
    };
    let index = unsafe {
        SendMessageW(
            tabs,
            TCM_HITTEST,
            WPARAM(0),
            LPARAM(&mut hit as *mut TCHITTESTINFO as isize),
        )
    }
    .0 as i32;
    if index < 0 {
        None
    } else {
        Some((index as usize, screen.x, screen.y))
    }
}

fn show_tab_context_menu(
    hwnd: HWND,
    state: &AppState,
    index: usize,
    x: i32,
    y: i32,
) -> Option<u16> {
    let doc = state.docs.get(index)?;
    let is_dirty = scintilla::is_modified(doc.editor);
    let has_path = doc.doc.path.is_some();
    let only_one = state.docs.len() <= 1;
    let is_first = index == 0;
    let is_last = index + 1 >= state.docs.len();

    let menu = unsafe { CreatePopupMenu().ok()? };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, IDM_FILE_SAVE as usize, w!("Save"));
        let _ = AppendMenuW(menu, MF_STRING, IDM_FILE_SAVE_AS as usize, w!("Save As..."));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TAB_DUPLICATE as usize,
            w!("Duplicate Tab"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, IDM_TAB_CLOSE as usize, w!("Close"));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TAB_CLOSE_OTHERS as usize,
            w!("Close Others"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TAB_CLOSE_LEFT as usize,
            w!("Close Tabs to the Left"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TAB_CLOSE_RIGHT as usize,
            w!("Close Tabs to the Right"),
        );

        let save_flags = if is_dirty || !has_path {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let _ = EnableMenuItem(menu, IDM_FILE_SAVE as u32, save_flags);

        let others_flags = if only_one {
            MF_BYCOMMAND | MF_GRAYED
        } else {
            MF_BYCOMMAND | MF_ENABLED
        };
        let _ = EnableMenuItem(menu, IDM_TAB_CLOSE_OTHERS as u32, others_flags);

        let left_flags = if only_one || is_first {
            MF_BYCOMMAND | MF_GRAYED
        } else {
            MF_BYCOMMAND | MF_ENABLED
        };
        let right_flags = if only_one || is_last {
            MF_BYCOMMAND | MF_GRAYED
        } else {
            MF_BYCOMMAND | MF_ENABLED
        };
        let _ = EnableMenuItem(menu, CMD_TAB_CLOSE_LEFT as u32, left_flags);
        let _ = EnableMenuItem(menu, IDM_TAB_CLOSE_RIGHT as u32, right_flags);
    }

    let selected = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            x,
            y,
            0,
            hwnd,
            None,
        )
    };
    unsafe {
        let _ = DestroyMenu(menu);
    }
    if selected.0 > 0 {
        Some(selected.0 as u16)
    } else {
        None
    }
}

fn show_editor_context_menu(hwnd: HWND, editor: HWND, x: i32, y: i32) -> Option<u16> {
    let menu = unsafe { CreatePopupMenu().ok()? };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, IDM_EDIT_UNDO as usize, w!("Undo"));
        let _ = AppendMenuW(menu, MF_STRING, IDM_EDIT_REDO as usize, w!("Redo"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, IDM_EDIT_CUT as usize, w!("Cut"));
        let _ = AppendMenuW(menu, MF_STRING, IDM_EDIT_COPY as usize, w!("Copy"));
        let _ = AppendMenuW(menu, MF_STRING, IDM_EDIT_PASTE as usize, w!("Paste"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_EDITOR_DELETE as usize, w!("Delete"));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_EDIT_SELECT_ALL as usize,
            w!("Select All"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRANSFORM_UPPERCASE as usize,
            w!("Uppercase"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRANSFORM_LOWERCASE as usize,
            w!("Lowercase"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRIM_LEADING_TRAILING as usize,
            w!("Trim Leading + Trailing Whitespace"),
        );
        let has_selection = !scintilla::selection_empty(editor);
        let sel_start = scintilla::selection_start(editor) as i64;
        let sel_end = scintilla::selection_end(editor) as i64;
        let undo_flags = if scintilla::can_undo(editor) {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let redo_flags = if scintilla::can_redo(editor) {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let cut_copy_delete_flags = if has_selection {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let paste_flags = if scintilla::can_paste(editor) {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let upper_flags = if can_uppercase(sel_start, sel_end) {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let lower_flags = if can_lowercase(sel_start, sel_end) {
            MF_BYCOMMAND | MF_ENABLED
        } else {
            MF_BYCOMMAND | MF_GRAYED
        };
        let _ = EnableMenuItem(menu, IDM_EDIT_UNDO as u32, undo_flags);
        let _ = EnableMenuItem(menu, IDM_EDIT_REDO as u32, redo_flags);
        let _ = EnableMenuItem(menu, IDM_EDIT_CUT as u32, cut_copy_delete_flags);
        let _ = EnableMenuItem(menu, IDM_EDIT_COPY as u32, cut_copy_delete_flags);
        let _ = EnableMenuItem(menu, CMD_EDITOR_DELETE as u32, cut_copy_delete_flags);
        let _ = EnableMenuItem(menu, IDM_EDIT_PASTE as u32, paste_flags);
        let _ = EnableMenuItem(menu, CMD_TRANSFORM_UPPERCASE as u32, upper_flags);
        let _ = EnableMenuItem(menu, CMD_TRANSFORM_LOWERCASE as u32, lower_flags);
    }
    let selected = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            x,
            y,
            0,
            hwnd,
            None,
        )
    };
    unsafe {
        let _ = DestroyMenu(menu);
    }
    if selected.0 > 0 {
        Some(selected.0 as u16)
    } else {
        None
    }
}

fn menu_id(id: usize) -> HMENU {
    HMENU(id as isize)
}

fn window_style(value: u32) -> WINDOW_STYLE {
    WINDOW_STYLE(value)
}

fn scale_for_dpi(hwnd: HWND, value: i32) -> i32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) } as i32;
    value.saturating_mul(dpi).div_euclid(96)
}

fn set_editor_dark_mode(hwnd: HWND, state: &mut AppState, enabled: bool) {
    state.editor_dark = enabled;
    update_editor_dark_menu(hwnd, enabled);
    for doc_tab in &state.docs {
        apply_syntax_for_doc(doc_tab, enabled);
    }
    if let Err(err) = update_tab_list_theme(state, enabled) {
        logging::log_error(&format!("Failed to update tab list theme: {err}"));
    }
    unsafe {
        InvalidateRect(state.tab_list, None, true);
    }
}

fn set_tab_layout(hwnd: HWND, state: &mut AppState, layout: TabLayout) {
    state.tab_layout = layout;
    update_tab_layout_menu(hwnd, layout);
    if layout == TabLayout::HorizontalTop && state.resizing_tabs {
        state.resizing_tabs = false;
        unsafe {
            let _ = ReleaseCapture();
        }
    }
    let (show_tabs, show_list) = match layout {
        TabLayout::HorizontalTop => (SW_SHOW, SW_HIDE),
        TabLayout::VerticalLeft | TabLayout::VerticalRight => (SW_HIDE, SW_SHOW),
    };
    unsafe {
        ShowWindow(state.tabs, show_tabs);
        ShowWindow(state.tab_list, show_list);
        let splitter_show = if layout == TabLayout::HorizontalTop {
            SW_HIDE
        } else {
            SW_SHOW
        };
        ShowWindow(state.tab_splitter, splitter_show);
    }
    set_tab_list_selection(state, state.active);
    layout_children(hwnd, state);
}

fn toggle_word_wrap(hwnd: HWND, state: &mut AppState) {
    let enabled = state
        .docs
        .get(state.active)
        .map(|doc_tab| !doc_tab.wrap_enabled)
        .unwrap_or(true);
    set_word_wrap(hwnd, state, enabled);
}

fn set_word_wrap(hwnd: HWND, state: &mut AppState, enabled: bool) {
    if let Some(doc_tab) = state.docs.get_mut(state.active) {
        doc_tab.wrap_enabled = enabled;
        let apply = enabled && !doc_tab.doc.large_file_mode;
        scintilla::set_wrap_enabled(doc_tab.editor, apply);
    }
    update_wrap_menu(hwnd, state);
}

fn set_always_on_top(hwnd: HWND, state: &mut AppState, enabled: bool) {
    state.always_on_top = enabled;
    update_always_on_top_menu(hwnd, state);
    let insert_after = if enabled {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

fn update_editor_dark_menu(hwnd: HWND, enabled: bool) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let flag = if enabled { MF_CHECKED } else { MF_UNCHECKED };
    let flags = (MF_BYCOMMAND | flag).0;
    unsafe {
        CheckMenuItem(menu, IDM_VIEW_EDITOR_DARK as u32, flags);
    }
}

fn update_tab_list_theme(state: &mut AppState, dark: bool) -> Result<()> {
    let (text, back) = tab_list_colors(dark);
    let brush = unsafe { CreateSolidBrush(back) };
    if brush.0 == 0 {
        return Err(AppError::win32("CreateSolidBrush(TabList)"));
    }
    if state.tab_list_brush.0 != 0 {
        unsafe {
            let _ = DeleteObject(state.tab_list_brush);
        }
    }
    state.tab_list_brush = brush;
    state.tab_list_text = text;
    state.tab_list_back = back;
    Ok(())
}

fn destroy_tab_list_brush(state: &mut AppState) {
    if state.tab_list_brush.0 != 0 {
        unsafe {
            let _ = DeleteObject(state.tab_list_brush);
        }
        state.tab_list_brush = HBRUSH(0);
    }
}

fn update_tab_layout_menu(hwnd: HWND, layout: TabLayout) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let (horizontal, left, right) = match layout {
        TabLayout::HorizontalTop => (true, false, false),
        TabLayout::VerticalLeft => (false, true, false),
        TabLayout::VerticalRight => (false, false, true),
    };
    set_menu_check(menu, IDM_VIEW_TABS_HORIZONTAL, horizontal);
    set_menu_check(menu, IDM_VIEW_TABS_VERTICAL_LEFT, left);
    set_menu_check(menu, IDM_VIEW_TABS_VERTICAL_RIGHT, right);
}

fn update_wrap_menu(hwnd: HWND, state: &AppState) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let checked = state
        .docs
        .get(state.active)
        .map(|doc_tab| doc_tab.wrap_enabled)
        .unwrap_or(true);
    set_menu_check(menu, IDM_VIEW_WORD_WRAP, checked);
}

fn update_copy_path_menu(hwnd: HWND, state: &AppState) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let path = current_document_path(state);
    let full_state = if can_copy_full_path(path) {
        MF_ENABLED
    } else {
        MF_GRAYED
    };
    let file_state = if can_copy_filename(path) {
        MF_ENABLED
    } else {
        MF_GRAYED
    };
    let dir_state = if can_copy_directory_path(path) {
        MF_ENABLED
    } else {
        MF_GRAYED
    };
    unsafe {
        EnableMenuItem(menu, CMD_COPY_FULL_PATH as u32, MF_BYCOMMAND | full_state);
        EnableMenuItem(menu, CMD_COPY_FILENAME as u32, MF_BYCOMMAND | file_state);
        EnableMenuItem(
            menu,
            CMD_COPY_DIRECTORY_PATH as u32,
            MF_BYCOMMAND | dir_state,
        );
    }
}

fn update_always_on_top_menu(hwnd: HWND, state: &AppState) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    set_menu_check(menu, IDM_VIEW_ALWAYS_ON_TOP, state.always_on_top);
}

fn set_menu_check(menu: HMENU, id: u16, checked: bool) {
    let flag = if checked { MF_CHECKED } else { MF_UNCHECKED };
    let flags = (MF_BYCOMMAND | flag).0;
    unsafe {
        CheckMenuItem(menu, id as u32, flags);
    }
}

unsafe extern "system" fn splitter_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            let parent = unsafe { GetParent(hwnd) };
            if parent.0 != 0
                && let Some(state) = get_state(parent)
            {
                state.resizing_tabs = true;
                unsafe {
                    let _ = SetCapture(parent);
                }
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            if let Ok(cursor) = unsafe { LoadCursorW(None, IDC_SIZEWE) } {
                unsafe {
                    SetCursor(cursor);
                }
                return LRESULT(1);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn find_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let createstruct = unsafe {
                &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW)
            };
            let main_hwnd = HWND(createstruct.lpCreateParams as isize);
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, main_hwnd.0);
            }

            let instance: HINSTANCE = match unsafe { GetModuleHandleW(None) } {
                Ok(value) => value.into(),
                Err(_) => return LRESULT(-1),
            };
            let edit_style = window_style(
                WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32 | WS_TABSTOP.0,
            );
            let check_style =
                window_style(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX as u32 | WS_TABSTOP.0);
            let button_style =
                window_style(WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32 | WS_TABSTOP.0);
            let scale = |value: i32| scale_for_dpi(hwnd, value);

            let (
                find_label,
                find_edit,
                replace_label,
                replace_edit,
                match_case,
                whole_word,
                regex,
                wrap,
                find_next,
                find_prev,
                replace_btn,
                replace_all,
                find_in_files,
                close_btn,
            ) = unsafe {
                let find_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Find:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(12),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let find_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    edit_style,
                    scale(90),
                    scale(10),
                    scale(250),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIND_TEXT),
                    instance,
                    None,
                );
                let replace_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Replace:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(42),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let replace_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    edit_style,
                    scale(90),
                    scale(40),
                    scale(250),
                    scale(22),
                    hwnd,
                    menu_id(IDC_REPLACE_TEXT),
                    instance,
                    None,
                );

                let match_case = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Match case"),
                    check_style,
                    scale(10),
                    scale(70),
                    scale(110),
                    scale(20),
                    hwnd,
                    menu_id(IDC_MATCH_CASE),
                    instance,
                    None,
                );
                let whole_word = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Whole word"),
                    check_style,
                    scale(130),
                    scale(70),
                    scale(110),
                    scale(20),
                    hwnd,
                    menu_id(IDC_WHOLE_WORD),
                    instance,
                    None,
                );
                let regex = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Regex"),
                    check_style,
                    scale(250),
                    scale(70),
                    scale(80),
                    scale(20),
                    hwnd,
                    menu_id(IDC_REGEX),
                    instance,
                    None,
                );
                let wrap = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Wrap"),
                    check_style,
                    scale(330),
                    scale(70),
                    scale(80),
                    scale(20),
                    hwnd,
                    menu_id(IDC_WRAP),
                    instance,
                    None,
                );

                let find_next = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Find Next"),
                    button_style,
                    scale(360),
                    scale(10),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIND_NEXT),
                    instance,
                    None,
                );
                let find_prev = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Find Prev"),
                    button_style,
                    scale(360),
                    scale(40),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIND_PREV),
                    instance,
                    None,
                );
                let replace_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Replace"),
                    button_style,
                    scale(360),
                    scale(70),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_REPLACE),
                    instance,
                    None,
                );
                let replace_all = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Replace All"),
                    button_style,
                    scale(360),
                    scale(100),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_REPLACE_ALL),
                    instance,
                    None,
                );
                let find_in_files = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Find in Files"),
                    button_style,
                    scale(10),
                    scale(100),
                    scale(120),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIND_IN_FILES),
                    instance,
                    None,
                );
                let close_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Close"),
                    button_style,
                    scale(360),
                    scale(130),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIND_CLOSE),
                    instance,
                    None,
                );
                (
                    find_label,
                    find_edit,
                    replace_label,
                    replace_edit,
                    match_case,
                    whole_word,
                    regex,
                    wrap,
                    find_next,
                    find_prev,
                    replace_btn,
                    replace_all,
                    find_in_files,
                    close_btn,
                )
            };

            let _ = find_label;
            let _ = replace_label;
            let _ = find_next;
            let _ = find_prev;
            let _ = replace_btn;
            let _ = replace_all;
            let _ = find_in_files;
            let _ = close_btn;

            if let Some(state) = get_state(main_hwnd) {
                state.find_dialog = Some(FindDialogState {
                    hwnd,
                    find_edit,
                    replace_edit,
                    match_case,
                    whole_word,
                    regex,
                    wrap,
                });
                let _ = apply_find_state_to_dialog(state);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                let id = loword(wparam.0) as usize;
                match id {
                    IDC_FIND_NEXT => {
                        let _ = perform_find_next(main_hwnd, state);
                    }
                    IDC_FIND_PREV => {
                        let _ = perform_find_prev(main_hwnd, state);
                    }
                    IDC_REPLACE => {
                        let _ = perform_replace(main_hwnd, state);
                    }
                    IDC_REPLACE_ALL => {
                        let _ = perform_replace_all(main_hwnd, state);
                    }
                    IDC_FIND_IN_FILES => {
                        let _ = show_find_in_files_dialog(main_hwnd, state);
                    }
                    IDC_FIND_CLOSE => unsafe {
                        ShowWindow(hwnd, SW_HIDE);
                    },
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                state.find_dialog = None;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn find_in_files_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let createstruct = unsafe {
                &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW)
            };
            let main_hwnd = HWND(createstruct.lpCreateParams as isize);
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, main_hwnd.0);
            }

            let instance: HINSTANCE = match unsafe { GetModuleHandleW(None) } {
                Ok(value) => value.into(),
                Err(_) => return LRESULT(-1),
            };
            let edit_style = window_style(
                WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32 | WS_TABSTOP.0,
            );
            let check_style =
                window_style(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX as u32 | WS_TABSTOP.0);
            let button_style =
                window_style(WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32 | WS_TABSTOP.0);
            let list_style = window_style(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_BORDER.0
                    | WS_VSCROLL.0
                    | LBS_NOTIFY as u32
                    | LBS_NOINTEGRALHEIGHT as u32,
            );
            let scale = |value: i32| scale_for_dpi(hwnd, value);

            let (
                find_label,
                find_edit,
                folder_label,
                folder_edit,
                browse_btn,
                include_label,
                include_edit,
                exclude_label,
                exclude_edit,
                match_case,
                whole_word,
                regex,
                recurse,
                find_btn,
                cancel_btn,
                close_btn,
                results,
            ) = unsafe {
                let find_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Find:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(12),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let find_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    edit_style,
                    scale(90),
                    scale(10),
                    scale(320),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIF_TEXT),
                    instance,
                    None,
                );
                let folder_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Folder:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(42),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let folder_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    edit_style,
                    scale(90),
                    scale(40),
                    scale(320),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIF_FOLDER),
                    instance,
                    None,
                );
                let browse_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Browse..."),
                    button_style,
                    scale(420),
                    scale(40),
                    scale(90),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIF_BROWSE),
                    instance,
                    None,
                );
                let include_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Include:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(72),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let include_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    w!("*.*"),
                    edit_style,
                    scale(90),
                    scale(70),
                    scale(320),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIF_INCLUDE),
                    instance,
                    None,
                );
                let exclude_label = CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Exclude:"),
                    WS_CHILD | WS_VISIBLE,
                    scale(10),
                    scale(102),
                    scale(80),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                );
                let exclude_edit = CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    edit_style,
                    scale(90),
                    scale(100),
                    scale(320),
                    scale(22),
                    hwnd,
                    menu_id(IDC_FIF_EXCLUDE),
                    instance,
                    None,
                );

                let match_case = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Match case"),
                    check_style,
                    scale(420),
                    scale(10),
                    scale(120),
                    scale(20),
                    hwnd,
                    menu_id(IDC_FIF_MATCH_CASE),
                    instance,
                    None,
                );
                let whole_word = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Whole word"),
                    check_style,
                    scale(420),
                    scale(70),
                    scale(120),
                    scale(20),
                    hwnd,
                    menu_id(IDC_FIF_WHOLE_WORD),
                    instance,
                    None,
                );
                let regex = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Regex"),
                    check_style,
                    scale(420),
                    scale(100),
                    scale(120),
                    scale(20),
                    hwnd,
                    menu_id(IDC_FIF_REGEX),
                    instance,
                    None,
                );
                let recurse = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Recurse"),
                    check_style,
                    scale(420),
                    scale(130),
                    scale(120),
                    scale(20),
                    hwnd,
                    menu_id(IDC_FIF_RECURSE),
                    instance,
                    None,
                );

                let find_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Find"),
                    button_style,
                    scale(550),
                    scale(10),
                    scale(90),
                    scale(24),
                    hwnd,
                    menu_id(IDC_FIF_FIND),
                    instance,
                    None,
                );
                let cancel_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Cancel"),
                    button_style,
                    scale(550),
                    scale(40),
                    scale(90),
                    scale(24),
                    hwnd,
                    menu_id(IDC_FIF_CANCEL),
                    instance,
                    None,
                );
                let close_btn = CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Close"),
                    button_style,
                    scale(550),
                    scale(70),
                    scale(90),
                    scale(24),
                    hwnd,
                    menu_id(IDC_FIF_CLOSE),
                    instance,
                    None,
                );

                let results = CreateWindowExW(
                    Default::default(),
                    w!("ListBox"),
                    PCWSTR::null(),
                    list_style,
                    scale(10),
                    scale(160),
                    scale(640),
                    scale(210),
                    hwnd,
                    menu_id(IDC_FIF_RESULTS),
                    instance,
                    None,
                );
                (
                    find_label,
                    find_edit,
                    folder_label,
                    folder_edit,
                    browse_btn,
                    include_label,
                    include_edit,
                    exclude_label,
                    exclude_edit,
                    match_case,
                    whole_word,
                    regex,
                    recurse,
                    find_btn,
                    cancel_btn,
                    close_btn,
                    results,
                )
            };

            let _ = find_label;
            let _ = folder_label;
            let _ = include_label;
            let _ = exclude_label;
            let _ = browse_btn;
            let _ = find_btn;
            let _ = cancel_btn;
            let _ = close_btn;

            if let Some(state) = get_state(main_hwnd) {
                state.find_in_files = Some(FindInFilesState {
                    hwnd,
                    find_edit,
                    folder_edit,
                    include_edit,
                    exclude_edit,
                    match_case,
                    whole_word,
                    regex,
                    recurse,
                    results,
                    cancel: Arc::new(AtomicBool::new(false)),
                    receiver: None,
                    running: false,
                    hits: Vec::new(),
                });
            }

            LRESULT(0)
        }
        WM_COMMAND => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                let id = loword(wparam.0) as usize;
                let code = hiword(wparam.0) as u32;
                match id {
                    IDC_FIF_FIND => {
                        let _ = start_find_in_files(main_hwnd, state);
                    }
                    IDC_FIF_CANCEL => {
                        cancel_find_in_files(state);
                    }
                    IDC_FIF_CLOSE => unsafe {
                        ShowWindow(hwnd, SW_HIDE);
                    },
                    IDC_FIF_BROWSE => {
                        if let Some(path) = browse_for_folder(hwnd)
                            && let Some(dialog) = &state.find_in_files
                        {
                            set_window_text(dialog.folder_edit, &path.display().to_string());
                        }
                    }
                    IDC_FIF_RESULTS => {
                        if code == LBN_DBLCLK
                            && let Some(dialog) = &mut state.find_in_files
                        {
                            let index = unsafe {
                                SendMessageW(dialog.results, LB_GETCURSEL, WPARAM(0), LPARAM(0))
                            }
                            .0 as isize;
                            if index >= 0 {
                                open_find_result(main_hwnd, state, index as usize);
                            }
                        }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                state.find_in_files = None;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_options(find_text: &str) -> FindInFilesOptions {
        FindInFilesOptions {
            find_text: find_text.to_string(),
            folder: PathBuf::from("C:\\"),
            include: Vec::new(),
            exclude: Vec::new(),
            match_case: false,
            whole_word: false,
            regex: false,
            recurse: true,
        }
    }

    #[test]
    fn tab_layout_cycles() {
        assert_eq!(TabLayout::HorizontalTop.next(), TabLayout::VerticalLeft);
        assert_eq!(TabLayout::VerticalLeft.next(), TabLayout::VerticalRight);
        assert_eq!(TabLayout::VerticalRight.next(), TabLayout::HorizontalTop);
    }

    #[test]
    fn parse_patterns_splits_and_trims() {
        let patterns = parse_patterns(" *.rs; *.txt, ,foo ");
        assert_eq!(patterns, vec!["*.rs", "*.txt", "foo"]);
    }

    #[test]
    fn wildcard_match_basic() {
        assert!(wildcard_match("*.txt", "notes.txt"));
        assert!(wildcard_match("r?vet.*", "rivet.log"));
        assert!(!wildcard_match("*.rs", "main.c"));
    }

    #[test]
    fn matches_patterns_respects_empty_list() {
        let path = Path::new("C:\\notes\\file.txt");
        assert!(matches_patterns(&[], path, true));
        assert!(!matches_patterns(&[], path, false));
    }

    #[test]
    fn match_substring_whole_word() {
        assert!(match_substring("hello world", "hello", true));
        assert!(!match_substring("hello_world", "hello", true));
        assert!(match_substring("hello_world", "hello", false));
    }

    #[test]
    fn count_words_basic() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("hello_world"), 1);
        assert_eq!(count_words("one-two"), 2);
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn trim_preview_limits_length() {
        let long = "a".repeat(205);
        let trimmed = trim_preview(&long);
        assert_eq!(trimmed.len(), 203);
        assert!(trimmed.ends_with("..."));
        assert_eq!(trim_preview("  hello  "), "hello");
    }

    #[test]
    fn line_matches_case_rules() {
        let mut options = make_options("Test");
        assert!(line_matches("this is test", &options, &None));
        options.match_case = true;
        assert!(!line_matches("this is test", &options, &None));
        options.find_text = "test".to_string();
        options.match_case = false;
        options.whole_word = true;
        assert!(!line_matches("testing", &options, &None));
        assert!(line_matches("test case", &options, &None));
    }

    #[test]
    fn line_matches_regex() {
        let options = make_options("ignored");
        let regex = regex::Regex::new(r"t.st").unwrap();
        assert!(line_matches("test", &options, &Some(regex)));
    }

    #[test]
    fn lexer_for_doc_extension_and_large_file() {
        let mut doc = Document::new_empty();
        doc.path = Some(PathBuf::from("C:\\notes\\file.py"));
        assert_eq!(lexer_for_doc(&doc), scintilla::LexerKind::Python);

        doc.large_file_mode = true;
        assert_eq!(lexer_for_doc(&doc), scintilla::LexerKind::Null);
    }
}
