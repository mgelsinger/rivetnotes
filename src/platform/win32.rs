use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{
    BOOL, COLORREF, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_HIDEPREFIX,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint, FillRect, HBRUSH,
    HDC, HGDIOBJ, InvalidateRect, LineTo, MONITOR_DEFAULTTONULL, MonitorFromRect, MoveToEx,
    PAINTSTRUCT, PS_SOLID, ScreenToClient, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Controls::{
    CDDS_ITEMPOSTPAINT, CDDS_ITEMPREPAINT, CDDS_PREPAINT, CDRF_DODEFAULT, CDRF_NEWFONT,
    CDRF_NOTIFYITEMDRAW, CDRF_NOTIFYPOSTPAINT, ICC_LISTVIEW_CLASSES, ICC_WIN95_CLASSES,
    INITCOMMONCONTROLSEX, InitCommonControlsEx, LIST_VIEW_ITEM_STATE_FLAGS, LVCF_WIDTH, LVCOLUMNW,
    LVHITTESTINFO, LVIF_PARAM, LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS,
    LVM_GETITEMRECT, LVM_HITTEST, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETBKCOLOR,
    LVM_SETCOLUMNWIDTH, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE, LVM_SETTEXTBKCOLOR,
    LVM_SETTEXTCOLOR, LVN_ITEMCHANGED, LVS_EX_DOUBLEBUFFER, LVS_EX_FULLROWSELECT,
    LVS_NOCOLUMNHEADER, LVS_REPORT, LVS_SHOWSELALWAYS, LVS_SINGLESEL, NM_CUSTOMDRAW, NM_RCLICK,
    NMHDR, NMLISTVIEW, NMLVCUSTOMDRAW, ODS_HOTLIGHT, ODS_INACTIVE, ODS_NOACCEL, ODS_SELECTED,
    SB_GETPARTS, SB_GETRECT, SB_GETTEXTLENGTHW, SB_GETTEXTW, SB_SETPARTS, SB_SETTEXTW,
    STATUSCLASSNAMEW, TCHITTESTINFO, TCIF_TEXT, TCITEMW, TCM_DELETEITEM, TCM_GETCURSEL,
    TCM_GETITEMCOUNT, TCM_GETITEMRECT, TCM_HITTEST, TCM_INSERTITEMW, TCM_SETCURSEL, TCM_SETITEMW,
    TCM_SETMINTABWIDTH, TCM_SETPADDING, TCN_SELCHANGE, WC_LISTVIEWW, WC_TABCONTROLW,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_MENU, VK_RETURN,
};
use windows::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, DefSubclassProc, DragAcceptFiles,
    DragFinish, DragQueryFileW, HDROP, RemoveWindowSubclass, SHBrowseForFolderW,
    SHGetPathFromIDListW, SetWindowSubclass, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, AppendMenuW, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_PUSHBUTTON,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CheckMenuItem, CreateAcceleratorTableW, CreateMenu,
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DeleteMenu, DestroyAcceleratorTable,
    DestroyMenu, DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL, ES_NUMBER, EnableMenuItem, FALT,
    FCONTROL, FSHIFT, FVIRTKEY, GCLP_HICON, GCLP_HICONSM, GWLP_USERDATA, GetClientRect,
    GetCursorPos, GetMenu, GetMenuBarInfo, GetMenuItemCount, GetMenuItemInfoW, GetMessageW,
    GetParent, GetSubMenu, GetSystemMetrics, GetWindowLongPtrW, GetWindowPlacement, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, HACCEL, HICON, HMENU, HWND_NOTOPMOST, HWND_TOPMOST,
    ICON_BIG, ICON_SMALL, ICON_SMALL2, IDC_ARROW, IDC_SIZEWE, IDI_APPLICATION, IDNO, IDYES,
    IMAGE_ICON, IsIconic, KillTimer, LB_ADDSTRING, LB_GETCURSEL, LB_RESETCONTENT, LBN_DBLCLK,
    LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LR_DEFAULTCOLOR, LR_SHARED, LoadCursorW, LoadIconW,
    LoadImageW, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO, MB_YESNOCANCEL,
    MENUBARINFO, MENUITEMINFOW, MF_BYCOMMAND, MF_BYPOSITION, MF_CHECKED, MF_ENABLED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MIIM_STRING, MSG, MessageBoxW, OBJID_MENU,
    PostQuitMessage, RegisterClassExW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SW_HIDE,
    SW_RESTORE, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SYSTEM_METRICS_INDEX, SendMessageW,
    SetClassLongPtrW, SetCursor, SetTimer, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos,
    SetWindowTextW, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateAcceleratorW, TranslateMessage, WINDOW_STYLE, WINDOWPLACEMENT, WINDOWPLACEMENT_FLAGS,
    WM_ACTIVATEAPP, WM_CAPTURECHANGED, WM_CHAR, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_COPYDATA,
    WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DROPFILES, WM_ERASEBKGND, WM_GETFONT, WM_GETICON,
    WM_INITMENUPOPUP, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONUP, WM_MOUSEMOVE,
    WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETCURSOR, WM_SETICON, WM_SIZE, WM_TIMER, WNDCLASSEXW,
    WPF_RESTORETOMAXIMIZED, WS_BORDER, WS_CAPTION, WS_CHILD, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW,
    WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PWSTR;
use windows::core::{HSTRING, PCWSTR, w};

use crate::app::document::{self, Document, Eol, TextEncoding};
use crate::app::session;
use crate::app::settings::{self, MAX_RECENT_FILES, TabPlacement, UiSettings};
use crate::commands::copy_full_path::{
    CopyPathKind, can_copy_directory_path, can_copy_filename, can_copy_full_path,
    copy_directory_path, copy_filename, copy_full_path,
};
use crate::commands::selection::{can_lowercase, can_uppercase};
use crate::editor::markdown;
use crate::editor::scintilla;
use crate::error::{AppError, Result};
use crate::logging;
use crate::platform::clipboard::{Clipboard, WinClipboard};
use crate::platform::dark_mode;
use crate::platform::single_instance;
use crate::textops::trim::{trim_edges_spaces_tabs, trim_line_preserve_eol};
use regex::RegexBuilder;

const IDM_FILE_NEW: u16 = 99;
const IDM_FILE_OPEN: u16 = 100;
const IDM_FILE_SAVE: u16 = 101;
const IDM_FILE_SAVE_AS: u16 = 102;
const IDM_FILE_SAVE_AS_UTF8_BOM: u16 = 103;
const IDM_FILE_SAVE_AS_UTF16_LE: u16 = 104;
const IDM_FILE_SAVE_ALL: u16 = 105;
const IDM_FILE_RELOAD: u16 = 106;
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
const CMD_EDITOR_STRIKEOUT: u16 = 315;
const IDM_EDIT_FIND: u16 = 320;
const IDM_EDIT_FIND_NEXT: u16 = 321;
const IDM_EDIT_FIND_PREV: u16 = 322;
const IDM_EDIT_REPLACE: u16 = 323;
const IDM_EDIT_REPLACE_ALL: u16 = 324;
const IDM_EDIT_FIND_IN_FILES: u16 = 325;
const IDM_EDIT_GOTO_LINE: u16 = 332;
const IDM_EDIT_INSERT_DATETIME: u16 = 333;
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
const CMD_VIEW_ALWAYS_ON_TOP: u16 = 346;
const CMD_TAB_NEXT: u16 = 349;
const CMD_TAB_PREV: u16 = 350;
const CMD_EDITOR_COLLAPSE_SELECTION: u16 = 352;
const CMD_EDITOR_EXPAND_ALL: u16 = 353;
const IDM_VIEW_ZOOM_IN: u16 = 354;
const IDM_VIEW_ZOOM_OUT: u16 = 355;
const IDM_VIEW_ZOOM_RESET: u16 = 356;
// Language (syntax) override commands. CMD_LANG_AUTO clears the per-tab override
// and falls back to extension detection; the rest force a specific lexer.
const CMD_LANG_AUTO: u16 = 360;
const CMD_LANG_PLAIN: u16 = 361;
const CMD_LANG_MARKDOWN: u16 = 362;
const CMD_LANG_CPP: u16 = 363;
const CMD_LANG_JAVASCRIPT: u16 = 364;
const CMD_LANG_JSON: u16 = 365;
const CMD_LANG_YAML: u16 = 366;
const CMD_LANG_POWERSHELL: u16 = 367;
const CMD_LANG_PYTHON: u16 = 368;
const CMD_LANG_HTML: u16 = 369;
const CMD_LANG_XML: u16 = 370;
const CMD_LANG_CSS: u16 = 371;
const CMD_LANG_PROPERTIES: u16 = 372;
const IDM_TAB_CLOSE: u16 = 220;
const IDM_TAB_CLOSE_OTHERS: u16 = 221;
const IDM_TAB_CLOSE_RIGHT: u16 = 222;
const CMD_TAB_DUPLICATE: u16 = 223;
const CMD_TAB_CLOSE_LEFT: u16 = 224;
const CMD_EDITOR_DELETE: u16 = 331;
const IDM_HELP_ABOUT: u16 = 400;
// Recent-files menu: one command id per slot (700..710), plus a Clear item.
const IDM_RECENT_FILE_BASE: u16 = 700;
const IDM_RECENT_CLEAR: u16 = 710;
// Position of the "Recent Files" submenu inside the File popup (see create_menu).
const FILE_MENU_RECENT_POS: i32 = 3;

const TIMER_SESSION_ID: usize = 1;
const TIMER_FIND_RESULTS: usize = 2;
const TIMER_WORD_COUNT: usize = 3;
const WORD_COUNT_INTERVAL_MS: u32 = 250;
const TIMER_MARKDOWN_FOLD: usize = 4;
const MARKDOWN_FOLD_INTERVAL_MS: u32 = 300;
const TAB_SPLITTER_WIDTH: i32 = 4;
const SMART_HL_INDIC: usize = 8;
const STRIKE_INDIC: usize = 9;
const STRIKE_INDIC_VALUE: i32 = 1;
const SMART_HL_MAX_TOKEN_LEN: usize = 128;
const SMART_HL_MAX_MATCHES: usize = 5000;

const SCN_SAVEPOINTREACHED: u32 = 2002;
const SCN_SAVEPOINTLEFT: u32 = 2003;
const SCN_UPDATEUI: u32 = 2007;
const SCN_MODIFIED: u32 = 2008;
const SCN_MARGINCLICK: u32 = 2010;
const SCN_ZOOM: u32 = 2018;

const VK_A: u16 = 0x41;
const VK_C: u16 = 0x43;
const VK_D: u16 = 0x44;
const VK_F: u16 = 0x46;
const VK_G: u16 = 0x47;
const VK_H: u16 = 0x48;
const VK_L: u16 = 0x4C;
const VK_T: u16 = 0x54;
const VK_V: u16 = 0x56;
const VK_W: u16 = 0x57;
const VK_X: u16 = 0x58;
const VK_Y: u16 = 0x59;
const VK_Z: u16 = 0x5A;
const VK_F3: u16 = 0x72;
const VK_F5: u16 = 0x74;
const VK_N: u16 = 0x4E;
const VK_S: u16 = 0x53;
const VK_OEM_4: u16 = 0xDB;
const VK_OEM_6: u16 = 0xDD;
const VK_TAB: u16 = 0x09;
const VK_PRIOR: u16 = 0x21;
const VK_NEXT: u16 = 0x22;
const VK_UP: u16 = 0x26;
const VK_DOWN: u16 = 0x28;
const VK_0: u16 = 0x30;
const VK_ADD: u16 = 0x6B;
const VK_SUBTRACT: u16 = 0x6D;
const VK_OEM_PLUS: u16 = 0xBB;
const VK_OEM_MINUS: u16 = 0xBD;

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
const IDC_GOTO_LINE: usize = 5301;
const IDC_GOTO_GO: usize = 5302;
const IDC_GOTO_CANCEL: usize = 5303;

const FIND_CLASS: PCWSTR = w!("rivet_find_dialog");
const FIND_FILES_CLASS: PCWSTR = w!("rivet_find_in_files");
const GOTO_LINE_CLASS: PCWSTR = w!("rivet_goto_line_dialog");
const SPLITTER_CLASS: PCWSTR = w!("rivet_tab_splitter");

const SCFIND_MATCHCASE: usize = 0x4;
const SCFIND_WHOLEWORD: usize = 0x2;
const SCFIND_REGEXP: usize = 0x0020_0000;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetFocus(hwnd: HWND) -> HWND;
    fn EnableWindow(hwnd: HWND, benable: BOOL) -> BOOL;
}

struct DocTab {
    runtime_id: isize,
    editor: HWND,
    doc: Document,
    wrap_enabled: bool,
    sticky_dirty: bool,
    word_count: Option<usize>,
    change_counter: u64,
    last_backup_change_counter: Option<u64>,
    smart_highlight_token: Option<String>,
    smart_highlight_truncated: bool,
    /// Manual syntax override chosen via the Language menu. `None` means the
    /// lexer is auto-detected from the file extension.
    lexer_override: Option<scintilla::LexerKind>,
}

struct SearchState {
    find_text: String,
    replace_text: String,
    match_case: bool,
    whole_word: bool,
    regex: bool,
    wrap: bool,
    last_direction: SearchDirection,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SearchDialogMode {
    Find,
    Replace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SearchDirection {
    Down,
    Up,
}

struct FindDialogState {
    hwnd: HWND,
    find_edit: HWND,
    replace_label: HWND,
    replace_edit: HWND,
    match_case: HWND,
    whole_word: HWND,
    regex: HWND,
    wrap: HWND,
    replace_btn: HWND,
    replace_all: HWND,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct SearchRange {
    start: usize,
    end: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct SearchPlan {
    primary: SearchRange,
    wrapped: Option<SearchRange>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ReplaceAllProgress {
    next_search_start: usize,
    next_doc_len: usize,
}

struct GoToLineDialogState {
    hwnd: HWND,
    range_label: HWND,
    line_edit: HWND,
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

struct AboutDetails {
    version: &'static str,
    git_sha: &'static str,
    build_utc: &'static str,
    source_url: &'static str,
    data_dir: PathBuf,
}

#[derive(Copy, Clone)]
struct TabTheme {
    bg: COLORREF,
    fg: COLORREF,
    selection_bg: COLORREF,
    selection_fg: COLORREF,
    hover_bg: COLORREF,
    border: COLORREF,
}

struct TabStripHost {
    top_tabs: HWND,
    vertical_tabs: HWND,
    splitter: HWND,
    vertical_tabs_brush: HBRUSH,
    theme: TabTheme,
    placement: TabPlacement,
    vertical_width_px: i32,
    resizing: bool,
    drag_start_x_screen: i32,
    drag_start_width: i32,
    hot_tab: Option<usize>,
    hot_close: bool,
    hot_in_top: bool,
}

impl TabStripHost {
    fn is_vertical(&self) -> bool {
        self.placement != TabPlacement::Top
    }
}

struct AppState {
    tab_host: TabStripHost,
    ui_settings: UiSettings,
    status: HWND,
    docs: Vec<DocTab>,
    active: usize,
    editor_dark: bool,
    next_tab_runtime_id: i64,
    always_on_top: bool,
    window_placement: Option<session::WindowPlacementData>,
    icon_big: HICON,
    icon_small: HICON,
    word_count_pending: bool,
    word_count_timer: bool,
    markdown_fold_pending: bool,
    markdown_fold_timer: bool,
    remember_session: bool,
    session_snapshot_periodic_backup: bool,
    backup_interval_seconds: u32,
    word_wrap_enabled: bool,
    next_untitled_index: usize,
    search_state: SearchState,
    search_dialog_mode: SearchDialogMode,
    find_dialog: Option<FindDialogState>,
    go_to_line_dialog: Option<GoToLineDialogState>,
    find_in_files: Option<FindInFilesState>,
    status_message: Option<String>,
}

pub fn run() -> Result<()> {
    let start = Instant::now();

    let cli_paths = single_instance::cli_paths();
    let instance_guard = single_instance::acquire();
    if instance_guard.already_running && single_instance::forward_to_existing(&cli_paths) {
        // Files (if any) were handed to the existing window; exit quietly.
        return Ok(());
    }

    let instance: HINSTANCE = unsafe { GetModuleHandleW(None) }
        .map_err(|err| AppError::new(format!("GetModuleHandleW: {err}")))?
        .into();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    scintilla::register_classes(instance)?;

    unsafe {
        // Not fatal: with the comctl32 v6 manifest this always succeeds on
        // supported Windows, and its FALSE return carries a stale last-error
        // (issue #1 saw ERROR_NOACCESS). If the classes are truly missing the
        // tab/status control creation fails later with a contextful error.
        if !InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES | ICC_LISTVIEW_CLASSES,
        })
        .as_bool()
        {
            logging::log_error("InitCommonControlsEx reported failure; continuing");
        }
    }

    let class_name = w!("rivet_main_window");
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }
        .map_err(|err| AppError::new(format!("LoadCursorW: {err}")))?;
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

    let menu = create_menu().map_err(|err| AppError::new(format!("create_menu: {err}")))?;
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            w!("Rivet"),
            WS_OVERLAPPEDWINDOW,
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

    // Session restore ran inside WM_CREATE, so any saved window placement is
    // already on the state. SetWindowPlacement shows the window itself; only
    // fall back to the default show when no valid placement applies.
    let placed = get_state(hwnd)
        .and_then(|state| state.window_placement)
        .map(|placement| apply_saved_window_placement(hwnd, &placement))
        .unwrap_or(false);
    if !placed {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
        }
    }

    // Session restore already ran inside WM_CREATE, so command-line files
    // land on top of the restored tabs as the active tab.
    if !cli_paths.is_empty()
        && let Some(state) = get_state(hwnd)
    {
        open_cli_paths(hwnd, state, &cli_paths);
    }

    let accel = create_accelerators()
        .map_err(|err| AppError::new(format!("create_accelerators: {err}")))?;

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

fn show_about_dialog(hwnd: HWND) -> Result<()> {
    let details = current_about_details()?;
    let about_text = format_about_details(&details);
    let (message, buttons) = if details.source_url == "unknown" {
        (
            format!("{about_text}\r\n\r\nPress Yes to copy details to clipboard."),
            MB_YESNO | MB_ICONINFORMATION,
        )
    } else {
        (
            format!(
                "{about_text}\r\n\r\nYes copies details to the clipboard. No opens the source page."
            ),
            MB_YESNOCANCEL | MB_ICONINFORMATION,
        )
    };
    let title = HSTRING::from("About Rivet");
    let message = HSTRING::from(&message);
    let result = unsafe {
        MessageBoxW(
            hwnd,
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            buttons,
        )
    };

    match result {
        IDYES => copy_about_details(hwnd, &about_text)?,
        IDNO if details.source_url != "unknown" => open_source_url(hwnd, details.source_url)?,
        _ => {}
    }

    Ok(())
}

fn current_about_details() -> Result<AboutDetails> {
    Ok(AboutDetails {
        version: option_env!("RIVET_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        git_sha: option_env!("RIVET_GIT_SHA").unwrap_or("unknown"),
        build_utc: option_env!("RIVET_BUILD_UTC").unwrap_or("unknown"),
        source_url: option_env!("RIVET_SOURCE_URL").unwrap_or("unknown"),
        data_dir: session::data_dir()?,
    })
}

fn format_about_details(details: &AboutDetails) -> String {
    format!(
        "Rivet {}\r\n\r\nCommit: {}\r\nBuild UTC: {}\r\nSource link: {}\r\nData dir: {}",
        details.version,
        short_git_sha(details.git_sha),
        details.build_utc,
        details.source_url,
        details.data_dir.display()
    )
}

fn short_git_sha(git_sha: &str) -> &str {
    if git_sha == "unknown" {
        return git_sha;
    }
    git_sha.get(..12).unwrap_or(git_sha)
}

fn copy_about_details(hwnd: HWND, details: &str) -> Result<()> {
    let mut clipboard = WinClipboard::new(hwnd);
    clipboard
        .set_unicode_text(details)
        .map_err(|err| AppError::new(format!("Failed to copy about details: {err}")))
}

fn open_source_url(hwnd: HWND, source_url: &str) -> Result<()> {
    let verb = HSTRING::from("open");
    let url = HSTRING::from(source_url);
    let result = unsafe {
        ShellExecuteW(
            hwnd,
            PCWSTR::from_raw(verb.as_ptr()),
            PCWSTR::from_raw(url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        )
    };

    if result.0 as isize <= 32 {
        return Err(AppError::new(format!(
            "Failed to open source URL: {source_url}"
        )));
    }

    Ok(())
}

fn create_menu() -> Result<HMENU> {
    unsafe {
        let menu = CreateMenu()?;
        let file_menu = CreatePopupMenu()?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_NEW as usize, w!("New"))?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_OPEN as usize, w!("Open..."))?;
        AppendMenuW(file_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        // Empty popup at FILE_MENU_RECENT_POS; populated by rebuild_recent_files_menu.
        let recent_menu = CreatePopupMenu()?;
        AppendMenuW(
            file_menu,
            MF_POPUP,
            recent_menu.0 as usize,
            w!("Recent Files"),
        )?;
        AppendMenuW(file_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(file_menu, MF_STRING, IDM_FILE_SAVE as usize, w!("Save"))?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_SAVE_ALL as usize,
            w!("Save All"),
        )?;
        AppendMenuW(file_menu, MF_STRING, IDM_TAB_CLOSE as usize, w!("Close"))?;
        AppendMenuW(
            file_menu,
            MF_STRING,
            IDM_FILE_RELOAD as usize,
            w!("Reload from Disk"),
        )?;
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
            IDM_EDIT_GOTO_LINE as usize,
            w!("Go To Line..."),
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
            CMD_EDITOR_STRIKEOUT as usize,
            w!("Strikeout"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            CMD_TRIM_LEADING_TRAILING as usize,
            w!("Trim Leading + Trailing Whitespace"),
        )?;
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            edit_menu,
            MF_STRING,
            IDM_EDIT_INSERT_DATETIME as usize,
            w!("Insert Date/Time"),
        )?;
        AppendMenuW(menu, MF_POPUP, edit_menu.0 as usize, w!("Edit"))?;

        let view_menu = CreatePopupMenu()?;
        let tabs_menu = CreatePopupMenu()?;
        AppendMenuW(
            tabs_menu,
            MF_STRING,
            IDM_VIEW_TABS_HORIZONTAL as usize,
            w!("Top"),
        )?;
        AppendMenuW(
            tabs_menu,
            MF_STRING,
            IDM_VIEW_TABS_VERTICAL_LEFT as usize,
            w!("Left"),
        )?;
        AppendMenuW(
            tabs_menu,
            MF_STRING,
            IDM_VIEW_TABS_VERTICAL_RIGHT as usize,
            w!("Right"),
        )?;
        AppendMenuW(view_menu, MF_POPUP, tabs_menu.0 as usize, w!("Tabs"))?;
        AppendMenuW(view_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            IDM_VIEW_WORD_WRAP as usize,
            w!("Word Wrap"),
        )?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            IDM_VIEW_EDITOR_DARK as usize,
            w!("Dark Mode"),
        )?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            CMD_VIEW_ALWAYS_ON_TOP as usize,
            w!("Always On Top"),
        )?;
        AppendMenuW(view_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            IDM_VIEW_ZOOM_IN as usize,
            w!("Zoom In"),
        )?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            IDM_VIEW_ZOOM_OUT as usize,
            w!("Zoom Out"),
        )?;
        AppendMenuW(
            view_menu,
            MF_STRING,
            IDM_VIEW_ZOOM_RESET as usize,
            w!("Reset Zoom"),
        )?;
        AppendMenuW(view_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        let language_menu = CreatePopupMenu()?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_AUTO as usize,
            w!("Auto (by extension)"),
        )?;
        AppendMenuW(language_menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_PLAIN as usize,
            w!("Plain Text"),
        )?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_MARKDOWN as usize,
            w!("Markdown"),
        )?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_CPP as usize,
            w!("C / C++"),
        )?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_JAVASCRIPT as usize,
            w!("JavaScript / TypeScript"),
        )?;
        AppendMenuW(language_menu, MF_STRING, CMD_LANG_JSON as usize, w!("JSON"))?;
        AppendMenuW(language_menu, MF_STRING, CMD_LANG_YAML as usize, w!("YAML"))?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_POWERSHELL as usize,
            w!("PowerShell"),
        )?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_PYTHON as usize,
            w!("Python"),
        )?;
        AppendMenuW(language_menu, MF_STRING, CMD_LANG_HTML as usize, w!("HTML"))?;
        AppendMenuW(language_menu, MF_STRING, CMD_LANG_XML as usize, w!("XML"))?;
        AppendMenuW(language_menu, MF_STRING, CMD_LANG_CSS as usize, w!("CSS"))?;
        AppendMenuW(
            language_menu,
            MF_STRING,
            CMD_LANG_PROPERTIES as usize,
            w!("Properties / INI"),
        )?;
        AppendMenuW(
            view_menu,
            MF_POPUP,
            language_menu.0 as usize,
            w!("Language"),
        )?;
        AppendMenuW(menu, MF_POPUP, view_menu.0 as usize, w!("View"))?;

        let help_menu = CreatePopupMenu()?;
        AppendMenuW(
            help_menu,
            MF_STRING,
            IDM_HELP_ABOUT as usize,
            w!("About Rivet"),
        )?;
        AppendMenuW(menu, MF_POPUP, help_menu.0 as usize, w!("Help"))?;

        Ok(menu)
    }
}

fn register_aux_classes(instance: HINSTANCE) -> Result<()> {
    register_window_class(instance, FIND_CLASS, Some(find_wndproc))?;
    register_window_class(instance, FIND_FILES_CLASS, Some(find_in_files_wndproc))?;
    register_window_class(instance, GOTO_LINE_CLASS, Some(goto_line_wndproc))?;
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
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }
            .map_err(|err| AppError::new(format!("LoadCursorW: {err}")))?,
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
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_G,
            cmd: IDM_EDIT_GOTO_LINE,
        },
        ACCEL {
            fVirt: FVIRTKEY,
            key: VK_F5,
            cmd: IDM_EDIT_INSERT_DATETIME,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_OEM_PLUS,
            cmd: IDM_VIEW_ZOOM_IN,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_ADD,
            cmd: IDM_VIEW_ZOOM_IN,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_OEM_MINUS,
            cmd: IDM_VIEW_ZOOM_OUT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_SUBTRACT,
            cmd: IDM_VIEW_ZOOM_OUT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_0,
            cmd: IDM_VIEW_ZOOM_RESET,
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
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_S,
            cmd: IDM_FILE_SAVE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_S,
            cmd: IDM_FILE_SAVE_ALL,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_X,
            cmd: CMD_EDITOR_STRIKEOUT,
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
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_TAB,
            cmd: CMD_TAB_NEXT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_TAB,
            cmd: CMD_TAB_PREV,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_NEXT,
            cmd: CMD_TAB_NEXT,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_PRIOR,
            cmd: CMD_TAB_PREV,
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
            && (message.hwnd == state.tab_host.top_tabs
                || message.hwnd == state.tab_host.vertical_tabs)
        {
            let (x, y) = lparam_xy(message.lParam);
            if let Some((index, _)) = hit_test_tab_at(state, message.hwnd, x, y)
                && let Err(err) = close_tab(hwnd, state, index)
            {
                show_error("Rivet error", &err.to_string());
            }
            continue;
        }
        if message.message == WM_LBUTTONDOWN
            && let Some(state) = get_state(hwnd)
            && (message.hwnd == state.tab_host.top_tabs
                || message.hwnd == state.tab_host.vertical_tabs)
        {
            let (x, y) = lparam_xy(message.lParam);
            if let Some((index, TabHitArea::Close)) = hit_test_tab_at(state, message.hwnd, x, y) {
                if let Err(err) = close_tab(hwnd, state, index) {
                    show_error("Rivet error", &err.to_string());
                }
                continue;
            }
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
    if msg == dark_mode::WM_UAHDRAWMENU
        && let Some(state) = get_state(hwnd)
        && state.editor_dark
    {
        let pudm = lparam.0 as *const dark_mode::UahMenu;
        if !pudm.is_null() {
            let udm = unsafe { *pudm };
            let mut mbi = MENUBARINFO {
                cbSize: std::mem::size_of::<MENUBARINFO>() as u32,
                ..Default::default()
            };
            let mut rc_win = windows::Win32::Foundation::RECT::default();
            unsafe {
                let _ = GetMenuBarInfo(hwnd, OBJID_MENU, 0, &mut mbi);
                let _ = GetWindowRect(hwnd, &mut rc_win);
            }
            let mut rc = mbi.rcBar;
            rc.left -= rc_win.left;
            rc.top -= rc_win.top;
            rc.right -= rc_win.left;
            rc.bottom -= rc_win.top;
            let brush = unsafe { CreateSolidBrush(state.tab_host.theme.bg) };
            if brush.0 != 0 {
                unsafe {
                    let _ = FillRect(udm.hdc, &rc, brush);
                    let _ = DeleteObject(brush);
                }
            }
            return LRESULT(0);
        }
    }
    if msg == dark_mode::WM_UAHDRAWMENUITEM
        && let Some(state) = get_state(hwnd)
        && state.editor_dark
    {
        let pudmi = lparam.0 as *const dark_mode::UahDrawMenuItem;
        if !pudmi.is_null() {
            let udmi = unsafe { *pudmi };
            let theme = state.tab_host.theme;
            let hot = (udmi.dis.itemState.0 & (ODS_HOTLIGHT.0 | ODS_SELECTED.0)) != 0;
            let inactive = (udmi.dis.itemState.0 & ODS_INACTIVE.0) != 0;
            let bg = if hot { theme.selection_bg } else { theme.bg };
            let fg = if inactive {
                theme.border
            } else if hot {
                theme.selection_fg
            } else {
                theme.fg
            };

            let mut buf: [u16; 256] = [0; 256];
            let mut mii = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING,
                dwTypeData: windows::core::PWSTR(buf.as_mut_ptr()),
                cch: buf.len() as u32 - 1,
                ..Default::default()
            };
            unsafe {
                let _ = GetMenuItemInfoW(udmi.um.hmenu, udmi.umi.i_position as u32, true, &mut mii);
            }

            let bg_brush = unsafe { CreateSolidBrush(bg) };
            if bg_brush.0 != 0 {
                unsafe {
                    let _ = FillRect(udmi.um.hdc, &udmi.dis.rcItem, bg_brush);
                    let _ = DeleteObject(bg_brush);
                }
            }

            let mut flags = DT_CENTER | DT_SINGLELINE | DT_VCENTER;
            if (udmi.dis.itemState.0 & ODS_NOACCEL.0) != 0 {
                flags |= DT_HIDEPREFIX;
            }
            let len = mii.cch as usize;
            if len > 0 {
                unsafe {
                    SetBkMode(udmi.um.hdc, TRANSPARENT);
                    SetTextColor(udmi.um.hdc, fg);
                    let mut text_rect = udmi.dis.rcItem;
                    DrawTextW(udmi.um.hdc, &mut buf[..len], &mut text_rect, flags);
                }
            }
            return LRESULT(0);
        }
    }
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
        WM_MOUSEMOVE => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        WM_LBUTTONUP => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        WM_CONTEXTMENU => {
            let source = HWND(wparam.0 as isize);
            if let Some(state) = get_state(hwnd) {
                if lparam.0 == -1
                    && let Some((index, x, y)) = keyboard_tab_context_menu_target(state, source)
                {
                    if let Some(command_id) = show_tab_context_menu(hwnd, state, index, x, y) {
                        unsafe {
                            SendMessageW(hwnd, WM_COMMAND, WPARAM(command_id as usize), LPARAM(0));
                        }
                    }
                    return LRESULT(0);
                }
                if doc_index_by_hwnd(state, source).is_some() {
                    let (x, y) = context_menu_position(lparam);
                    if let Some(command_id) = show_editor_context_menu(hwnd, source, x, y) {
                        unsafe {
                            SendMessageW(hwnd, WM_COMMAND, WPARAM(command_id as usize), LPARAM(0));
                        }
                    }
                    return LRESULT(0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_INITMENUPOPUP => {
            if let Some(state) = get_state(hwnd) {
                update_file_menu(hwnd, state);
                update_copy_path_menu(hwnd, state);
                update_language_menu(hwnd, state);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let command_id = (wparam.0 & 0xffff) as u16;
            match command_id {
                IDM_FILE_OPEN => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = open_from_dialog(hwnd, state)
                    {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_FILE_RELOAD => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = reload_active_from_disk(hwnd, state)
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
                        && let Err(err) = show_find_dialog(hwnd, state, SearchDialogMode::Find)
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
                        && let Err(err) = show_find_dialog(hwnd, state, SearchDialogMode::Replace)
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
                IDM_EDIT_GOTO_LINE => {
                    if let Some(state) = get_state(hwnd)
                        && let Err(err) = show_go_to_line_dialog(hwnd, state)
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
                IDM_EDIT_INSERT_DATETIME => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        scintilla::replace_selection(editor, &current_date_time_stamp());
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
                CMD_EDITOR_STRIKEOUT => {
                    if let Some(state) = get_state(hwnd) {
                        let index = state.active;
                        let toggled = state
                            .docs
                            .get_mut(index)
                            .map(|doc_tab| toggle_strikethrough(doc_tab.editor))
                            .unwrap_or(false);
                        if toggled {
                            if let Some(doc_tab) = state.docs.get_mut(index) {
                                doc_tab.doc.is_dirty = true;
                                doc_tab.change_counter = doc_tab.change_counter.saturating_add(1);
                            }
                            if let Err(err) = save_session_checkpoint(hwnd, state) {
                                logging::log_error(&format!(
                                    "session_save_after_strikeout_failed err={err}"
                                ));
                            }
                            update_status(state);
                        }
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
                        set_tab_layout(hwnd, state, TabPlacement::Top);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_VERTICAL_LEFT => {
                    if let Some(state) = get_state(hwnd) {
                        set_tab_layout(hwnd, state, TabPlacement::Left);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_VERTICAL_RIGHT => {
                    if let Some(state) = get_state(hwnd) {
                        set_tab_layout(hwnd, state, TabPlacement::Right);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_TABS_CYCLE => {
                    if let Some(state) = get_state(hwnd) {
                        let next = state.tab_host.placement.next();
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
                IDM_VIEW_ZOOM_IN => {
                    if let Some(state) = get_state(hwnd) {
                        adjust_zoom(state, 1);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_ZOOM_OUT => {
                    if let Some(state) = get_state(hwnd) {
                        adjust_zoom(state, -1);
                    }
                    LRESULT(0)
                }
                IDM_VIEW_ZOOM_RESET => {
                    if let Some(state) = get_state(hwnd) {
                        set_app_zoom(state, settings::DEFAULT_ZOOM_LEVEL);
                    }
                    LRESULT(0)
                }
                CMD_EDITOR_COLLAPSE_SELECTION => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        collapse_selection_lines(editor);
                    }
                    LRESULT(0)
                }
                CMD_EDITOR_EXPAND_ALL => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(editor) = active_editor(state)
                    {
                        expand_all_user_collapses(editor);
                    }
                    LRESULT(0)
                }
                id if (CMD_LANG_AUTO..=CMD_LANG_PROPERTIES).contains(&id) => {
                    if let Some(state) = get_state(hwnd)
                        && let Some(over) = lexer_override_for_command(id)
                    {
                        set_language_override(state, over);
                    }
                    LRESULT(0)
                }
                CMD_VIEW_ALWAYS_ON_TOP => {
                    if let Some(state) = get_state(hwnd) {
                        let enabled = !state.always_on_top;
                        set_always_on_top(hwnd, state, enabled);
                    }
                    LRESULT(0)
                }
                CMD_TAB_NEXT => {
                    if let Some(state) = get_state(hwnd) {
                        select_adjacent_tab(hwnd, state, true);
                    }
                    LRESULT(0)
                }
                CMD_TAB_PREV => {
                    if let Some(state) = get_state(hwnd) {
                        select_adjacent_tab(hwnd, state, false);
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
                    if let Some(state) = get_state(hwnd) {
                        match can_exit(hwnd, state) {
                            Ok(true) => {
                                let _ = unsafe { DestroyWindow(hwnd) };
                            }
                            Ok(false) => {}
                            Err(err) => show_error("Rivet error", &err.to_string()),
                        }
                    }
                    LRESULT(0)
                }
                IDM_HELP_ABOUT => {
                    if let Err(err) = show_about_dialog(hwnd) {
                        show_error("Rivet error", &err.to_string());
                    }
                    LRESULT(0)
                }
                IDM_RECENT_CLEAR => {
                    if let Some(state) = get_state(hwnd) {
                        clear_recent_files(hwnd, state);
                    }
                    LRESULT(0)
                }
                id if (IDM_RECENT_FILE_BASE..IDM_RECENT_FILE_BASE + MAX_RECENT_FILES as u16)
                    .contains(&id) =>
                {
                    if let Some(state) = get_state(hwnd) {
                        let slot = (id - IDM_RECENT_FILE_BASE) as usize;
                        if let Err(err) = open_recent_file(hwnd, state, slot) {
                            show_error("Rivet error", &err.to_string());
                        }
                    }
                    LRESULT(0)
                }
                _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        WM_NOTIFY => {
            let nmhdr = unsafe { &*(lparam.0 as *const NMHDR) };
            if nmhdr.hwndFrom != HWND(0) {
                if let Some(state) = get_state(hwnd) {
                    if nmhdr.code == NM_RCLICK
                        && (nmhdr.hwndFrom == state.tab_host.top_tabs
                            || nmhdr.hwndFrom == state.tab_host.vertical_tabs)
                    {
                        if let Some((index, x, y)) = tab_hit_test_at_cursor(state) {
                            select_tab(hwnd, state, index);
                            if let Some(command_id) =
                                show_tab_context_menu(hwnd, state, index, x, y)
                            {
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

                    if nmhdr.hwndFrom == state.tab_host.top_tabs && nmhdr.code == TCN_SELCHANGE {
                        let index = unsafe {
                            SendMessageW(
                                state.tab_host.top_tabs,
                                TCM_GETCURSEL,
                                WPARAM(0),
                                LPARAM(0),
                            )
                            .0
                        } as i32;
                        if index >= 0 {
                            select_tab(hwnd, state, index as usize);
                        }
                        return LRESULT(0);
                    }

                    if nmhdr.hwndFrom == state.tab_host.vertical_tabs
                        && nmhdr.code == LVN_ITEMCHANGED
                    {
                        let info = unsafe { &*(lparam.0 as *const NMLISTVIEW) };
                        let became_selected = (info.uNewState & LVIS_SELECTED.0) != 0
                            && (info.uOldState & LVIS_SELECTED.0) == 0;
                        if became_selected && info.iItem >= 0 {
                            let target = doc_index_by_runtime_id(state, info.lParam)
                                .or(Some(info.iItem as usize));
                            if let Some(index) = target
                                && index < state.docs.len()
                                && index != state.active
                            {
                                select_tab(hwnd, state, index);
                            }
                        }
                        return LRESULT(0);
                    }

                    if nmhdr.hwndFrom == state.tab_host.vertical_tabs && nmhdr.code == NM_CUSTOMDRAW
                    {
                        return handle_vertical_tab_custom_draw(state, lparam);
                    }
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

                if nmhdr.code == SCN_MARGINCLICK {
                    let notif = unsafe { &*(lparam.0 as *const scintilla::SciNotification) };
                    if notif.margin == 1 {
                        let editor = nmhdr.hwndFrom;
                        let line = scintilla::line_from_position(editor, notif.position as usize);
                        let markers = scintilla::marker_get(editor, line);
                        if (markers & (1u32 << scintilla::USER_COLLAPSE_MARKER)) != 0 {
                            expand_user_collapse(editor, line);
                            return LRESULT(0);
                        }
                        let level = scintilla::fold_level(editor, line);
                        if (level & 0x2000) != 0 {
                            let last = scintilla::fold_last_child(editor, line, -1);
                            let lines = last.saturating_sub(line);
                            let label = if lines == 1 {
                                String::from(" \u{22EF} 1 line ")
                            } else {
                                format!(" \u{22EF} {lines} lines ")
                            };
                            scintilla::toggle_fold_show_text(editor, line, &label);
                        } else {
                            scintilla::toggle_fold(editor, line);
                        }
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_ZOOM {
                    // Ctrl+mousewheel zooms one Scintilla control; fold that
                    // back into the app-wide level so every tab stays in sync.
                    if let Some(state) = get_state(hwnd) {
                        set_app_zoom(state, scintilla::get_zoom(nmhdr.hwndFrom));
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_UPDATEUI {
                    if let Some(state) = get_state(hwnd) {
                        if let Some(index) = doc_index_by_hwnd(state, nmhdr.hwndFrom)
                            && index == state.active
                        {
                            update_smart_highlight_for_doc(state, index, false);
                        }
                        update_status(state);
                    }
                    return LRESULT(0);
                }

                if nmhdr.code == SCN_MODIFIED {
                    if let Some(state) = get_state(hwnd) {
                        if let Some(index) = doc_index_by_hwnd(state, nmhdr.hwndFrom)
                            && let Some(doc_tab) = state.docs.get_mut(index)
                        {
                            doc_tab.change_counter = doc_tab.change_counter.saturating_add(1);
                            doc_tab.doc.cursor_pos =
                                scintilla::get_current_pos(doc_tab.editor) as i64;
                        }
                        let line_count = scintilla::line_count(nmhdr.hwndFrom);
                        scintilla::set_line_number_margin_width(nmhdr.hwndFrom, line_count);
                        if let Some(index) = doc_index_by_hwnd(state, nmhdr.hwndFrom)
                            && index == state.active
                        {
                            update_smart_highlight_for_doc(state, index, true);
                        }
                        if let Some(index) = doc_index_by_hwnd(state, nmhdr.hwndFrom) {
                            schedule_markdown_fold(hwnd, state, index);
                        }
                        schedule_word_count(hwnd, state);
                        update_status(state);
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
                if let Err(err) = run_snapshot_tick(hwnd, state, false) {
                    logging::log_error(&format!("snapshot_tick_failed err={err}"));
                }
            } else if wparam.0 == TIMER_FIND_RESULTS
                && let Some(state) = get_state(hwnd)
            {
                poll_find_results(hwnd, state);
            } else if wparam.0 == TIMER_WORD_COUNT
                && let Some(state) = get_state(hwnd)
            {
                handle_word_count_timer(hwnd, state);
            } else if wparam.0 == TIMER_MARKDOWN_FOLD
                && let Some(state) = get_state(hwnd)
            {
                handle_markdown_fold_timer(hwnd, state);
            }
            LRESULT(0)
        }
        WM_COPYDATA => {
            let cds = lparam.0 as *const COPYDATASTRUCT;
            if cds.is_null() {
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
            let (dw_data, bytes) = unsafe {
                let cds = &*cds;
                let bytes = if cds.lpData.is_null() || cds.cbData == 0 {
                    Vec::new()
                } else {
                    // Copy out immediately; lpData is only valid while the
                    // sender blocks in SendMessage. Byte-wise because the
                    // foreign pointer has no alignment guarantee.
                    std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize)
                        .to_vec()
                };
                (cds.dwData, bytes)
            };
            if dw_data != single_instance::COPYDATA_OPEN_FILES {
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            let paths = single_instance::decode_paths(&units);
            if let Some(state) = get_state(hwnd) {
                open_cli_paths(hwnd, state, &paths);
            }
            unsafe {
                if IsIconic(hwnd).as_bool() {
                    ShowWindow(hwnd, SW_RESTORE);
                }
            }
            LRESULT(1)
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
            if let Some(state) = get_state(hwnd) {
                match can_exit(hwnd, state) {
                    Ok(true) => {
                        let _ = unsafe { DestroyWindow(hwnd) };
                    }
                    Ok(false) => {}
                    Err(err) => show_error("Rivet error", &err.to_string()),
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = get_state(hwnd)
                && let Err(err) = save_session_checkpoint(hwnd, state)
            {
                logging::log_error(&format!("session_save_on_destroy_failed err={err}"));
            }
            unsafe {
                let _ = KillTimer(hwnd, TIMER_SESSION_ID);
                let _ = KillTimer(hwnd, TIMER_FIND_RESULTS);
                let _ = KillTimer(hwnd, TIMER_WORD_COUNT);
                let _ = KillTimer(hwnd, TIMER_MARKDOWN_FOLD);
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
                    destroy_tab_host_brush(&mut *state_ptr);
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
    let ui_settings = match settings::load_settings() {
        Ok(value) => value,
        Err(err) => {
            logging::log_error(&format!("settings_load_failed err={err}"));
            UiSettings::default()
        }
    };

    // Apply dark mode early so the title bar, menu bar, and any child windows
    // created below pick up the right theme before the first paint.
    dark_mode::apply_to_window(hwnd, ui_settings.editor_dark);

    let (icon_big, icon_small) = load_main_icons(instance);
    let top_tabs = unsafe {
        CreateWindowExW(
            Default::default(),
            WC_TABCONTROLW,
            PCWSTR::null(),
            window_style(WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0),
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
    if top_tabs.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(TabControl)"));
    }
    unsafe {
        let _ = SetWindowSubclass(top_tabs, Some(top_tabs_subclass_proc), 0, 0);
        // Strip visual styles so the subclass's WM_PAINT takes full control;
        // otherwise comctl32 paints the standard light-themed tabs first and
        // we can only paint over a small portion (background + bottom seam).
        dark_mode::disable_visual_styles(top_tabs);
        let pad_x = scale_for_dpi(
            top_tabs,
            TAB_ITEM_LABEL_PADDING + TAB_CLOSE_BTN_SIZE + TAB_CLOSE_BTN_MARGIN,
        );
        let pad_y = scale_for_dpi(top_tabs, 4);
        SendMessageW(
            top_tabs,
            TCM_SETPADDING,
            WPARAM(0),
            LPARAM(((pad_y as isize) << 16) | (pad_x as isize & 0xFFFF)),
        );
        SendMessageW(
            top_tabs,
            TCM_SETMINTABWIDTH,
            WPARAM(0),
            LPARAM(scale_for_dpi(top_tabs, TAB_ITEM_MIN_WIDTH) as isize),
        );
    }

    let vertical_tabs = unsafe {
        CreateWindowExW(
            Default::default(),
            WC_LISTVIEWW,
            PCWSTR::null(),
            window_style(
                WS_CHILD.0
                    | WS_BORDER.0
                    | WS_TABSTOP.0
                    | WS_CLIPSIBLINGS.0
                    | LVS_REPORT
                    | LVS_SINGLESEL
                    | LVS_NOCOLUMNHEADER
                    | LVS_SHOWSELALWAYS,
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
    if vertical_tabs.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(VerticalTabs)"));
    }

    unsafe {
        SendMessageW(
            vertical_tabs,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            WPARAM((LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as usize),
            LPARAM((LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as isize),
        );
    }
    // Do NOT call dark_mode::disable_visual_styles on this ListView — unlike
    // WC_TABCONTROLW, a WC_LISTVIEWW in LVS_REPORT mode relies on visual
    // styles for item layout, and stripping them collapses item heights so
    // rows render invisible. CDRF_SKIPDEFAULT in handle_vertical_tab_custom_draw
    // is sufficient to prevent comctl32 from drawing chrome over our paint.

    let mut column = LVCOLUMNW {
        mask: LVCF_WIDTH,
        cx: ui_settings.vertical_tab_width_px,
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            vertical_tabs,
            LVM_INSERTCOLUMNW,
            WPARAM(0),
            LPARAM(&mut column as *mut LVCOLUMNW as isize),
        );
    }

    let splitter = unsafe {
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
    if splitter.0 == 0 {
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
    unsafe {
        let _ = SetWindowSubclass(status, Some(status_bar_subclass_proc), 0, 0);
    }

    let theme = tab_theme(ui_settings.editor_dark);
    let vertical_tabs_brush = unsafe { CreateSolidBrush(theme.bg) };
    if vertical_tabs_brush.0 == 0 {
        return Err(AppError::win32("CreateSolidBrush(VerticalTabs)"));
    }

    // Captured before the struct literal moves `ui_settings` into the state.
    let editor_dark = ui_settings.editor_dark;
    let state = AppState {
        tab_host: TabStripHost {
            top_tabs,
            vertical_tabs,
            splitter,
            vertical_tabs_brush,
            theme,
            placement: ui_settings.tab_placement,
            vertical_width_px: ui_settings.vertical_tab_width_px,
            resizing: false,
            drag_start_x_screen: 0,
            drag_start_width: 0,
            hot_tab: None,
            hot_close: false,
            hot_in_top: false,
        },
        ui_settings,
        status,
        docs: Vec::new(),
        active: 0,
        editor_dark,
        next_tab_runtime_id: 1,
        always_on_top: session::DEFAULT_ALWAYS_ON_TOP,
        window_placement: None,
        icon_big,
        icon_small,
        word_count_pending: false,
        word_count_timer: false,
        markdown_fold_pending: false,
        markdown_fold_timer: false,
        remember_session: session::DEFAULT_REMEMBER_SESSION,
        session_snapshot_periodic_backup: session::DEFAULT_SESSION_SNAPSHOT_PERIODIC_BACKUP,
        backup_interval_seconds: session::DEFAULT_BACKUP_INTERVAL_SECONDS,
        word_wrap_enabled: session::DEFAULT_WORD_WRAP_ENABLED,
        next_untitled_index: 1,
        search_state: SearchState {
            find_text: String::new(),
            replace_text: String::new(),
            match_case: false,
            whole_word: false,
            regex: false,
            wrap: true,
            last_direction: SearchDirection::Down,
        },
        search_dialog_mode: SearchDialogMode::Find,
        find_dialog: None,
        go_to_line_dialog: None,
        find_in_files: None,
        status_message: None,
    };

    let mut state = restore_session(hwnd, state)?;
    let editor_dark = state.editor_dark;
    if let Err(err) = update_tab_host_theme(&mut state, editor_dark) {
        logging::log_error(&format!("tab_host_theme_init_failed err={err}"));
    }
    if state.docs.is_empty() {
        create_empty_tab(hwnd, instance, &mut state)?;
    }

    let show_vertical = if state.tab_host.placement == TabPlacement::Top {
        SW_HIDE
    } else {
        SW_SHOW
    };
    unsafe {
        ShowWindow(state.tab_host.vertical_tabs, show_vertical);
        ShowWindow(state.tab_host.splitter, show_vertical);
    }

    let active = state.active.min(state.docs.len().saturating_sub(1));
    select_tab(hwnd, &mut state, active);
    layout_children(hwnd, &mut state);
    apply_always_on_top(hwnd, state.always_on_top);
    update_status(&state);
    update_tab_layout_menu(hwnd, state.tab_host.placement);
    update_wrap_menu(hwnd, &state);
    update_editor_dark_menu(hwnd, state.editor_dark);
    update_always_on_top_menu(hwnd, &state);
    rebuild_recent_files_menu(hwnd, &state);

    unsafe {
        let _ = SetTimer(
            hwnd,
            TIMER_SESSION_ID,
            backup_interval_ms(state.backup_interval_seconds),
            None,
        );
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
    let tab_height = match state.tab_host.placement {
        TabPlacement::Top => tab_bar_height(state).max(0),
        TabPlacement::Left | TabPlacement::Right => 0,
    };
    let list_width = match state.tab_host.placement {
        TabPlacement::Top => 0,
        TabPlacement::Left | TabPlacement::Right => {
            let adjusted = clamp_vertical_tab_width(state, state.tab_host.vertical_width_px, width);
            state.tab_host.vertical_width_px = adjusted;
            adjusted
        }
    };
    let editor_height = (height - status_height - tab_height).max(0);
    let editor_width = match state.tab_host.placement {
        TabPlacement::Top => width.max(0),
        TabPlacement::Left | TabPlacement::Right => {
            (width - list_width - TAB_SPLITTER_WIDTH).max(0)
        }
    };
    let (list_left, splitter_left, editor_left) = match state.tab_host.placement {
        TabPlacement::Top => (0, 0, 0),
        TabPlacement::Left => (0, list_width, list_width + TAB_SPLITTER_WIDTH),
        TabPlacement::Right => {
            let editor_left = 0;
            let splitter_left = editor_width;
            let list_left = editor_width + TAB_SPLITTER_WIDTH;
            (list_left, splitter_left, editor_left)
        }
    };

    unsafe {
        let _ = SetWindowPos(
            state.tab_host.top_tabs,
            HWND(0),
            0,
            0,
            width,
            tab_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = SetWindowPos(
            state.tab_host.vertical_tabs,
            HWND(0),
            list_left,
            0,
            list_width,
            height - status_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = SetWindowPos(
            state.tab_host.splitter,
            HWND(0),
            splitter_left,
            0,
            TAB_SPLITTER_WIDTH,
            height - status_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = SetWindowPos(
            state.status,
            HWND(0),
            0,
            height - status_height,
            width,
            status_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        set_vertical_tab_column_width(state.tab_host.vertical_tabs, list_width);
        for doc in &state.docs {
            let _ = SetWindowPos(
                doc.editor,
                HWND(0),
                editor_left,
                tab_height,
                editor_width,
                editor_height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
    if state.tab_host.placement == TabPlacement::Top {
        unsafe {
            InvalidateRect(state.tab_host.top_tabs, None, true);
        }
    }
    update_status_parts(state);
}

fn set_vertical_tab_column_width(vertical_tabs: HWND, width: i32) {
    unsafe {
        SendMessageW(
            vertical_tabs,
            LVM_SETCOLUMNWIDTH,
            WPARAM(0),
            LPARAM(width as isize),
        );
    }
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
    let wrap_enabled = wrap.unwrap_or(state.word_wrap_enabled);
    let mut doc_tab = create_doc_from_path(
        hwnd,
        instance,
        path,
        wrap_enabled,
        state.ui_settings.large_file_threshold_mb,
        state.ui_settings.large_file_disable_word_wrap,
        state.ui_settings.zoom_level,
    )?;
    if let Some(encoding) = encoding {
        doc_tab.doc.encoding = encoding;
    }
    if let Some(eol) = eol {
        doc_tab.doc.eol = eol;
        scintilla::set_eol_mode(doc_tab.editor, eol);
    }

    apply_syntax_for_doc(&doc_tab, state.editor_dark);
    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    apply_large_file_mode_restrictions(hwnd, state, index);
    if let Some(caret) = caret {
        let editor = state.docs[index].editor;
        scintilla::goto_pos(editor, caret);
    }
    if let Some(opened) = state.docs.get(index).and_then(|d| d.doc.path.clone()) {
        note_recent_file(hwnd, state, &opened);
    }
    Ok(())
}

/// Opens paths passed on the command line or forwarded from a second
/// instance. A lone pristine Untitled tab is replaced by the opened files.
fn open_cli_paths(hwnd: HWND, state: &mut AppState, paths: &[PathBuf]) {
    let lone_pristine_tab = state.docs.len() == 1
        && state
            .docs
            .first()
            .is_some_and(|doc_tab| doc_tab.doc.path.is_none() && !doc_tab.doc.is_dirty)
        && state
            .docs
            .first()
            .is_some_and(|doc_tab| scintilla::get_length(doc_tab.editor) == 0);
    let mut opened_any = false;
    for path in paths {
        match open_path_new_tab(hwnd, state, path.clone(), None, None, None, None) {
            Ok(()) => opened_any = true,
            Err(err) => {
                logging::log_error(&format!(
                    "cli_open_failed path={} err={err}",
                    path.display()
                ));
                show_error("Rivet error", &err.to_string());
            }
        }
    }
    if opened_any
        && lone_pristine_tab
        && let Err(err) = close_tab(hwnd, state, 0)
    {
        logging::log_error(&format!("cli_close_placeholder_failed err={err}"));
    }
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
            .map(|doc_tab| doc_tab.doc.is_dirty)
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
    let (encoding, existing_path) = {
        let doc_tab = state
            .docs
            .get(index)
            .ok_or_else(|| AppError::new("No active document."))?;
        (
            encoding_override.unwrap_or(doc_tab.doc.encoding),
            doc_tab.doc.path.clone(),
        )
    };
    let path = if force_save_as || existing_path.is_none() {
        match save_file_dialog(hwnd)? {
            Some(path) => path,
            None => return Ok(false),
        }
    } else {
        existing_path.ok_or_else(|| AppError::new("No file path available."))?
    };
    let backup_path = {
        let doc_tab = state
            .docs
            .get_mut(index)
            .ok_or_else(|| AppError::new("No active document."))?;
        let text = scintilla::get_text(doc_tab.editor)?;
        let normalized = document::normalize_eol(&text, doc_tab.doc.eol);
        let bytes = document::encode_text(&normalized, encoding)?;

        std::fs::write(&path, &bytes)
            .map_err(|err| AppError::new(format!("Failed to write file: {err}")))?;

        let stamp = document::FileStamp::from_path(&path)?;
        doc_tab.doc.path = Some(path.clone());
        doc_tab.doc.display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_string();
        doc_tab
            .doc
            .update_after_save(encoding, doc_tab.doc.eol, stamp);
        doc_tab.doc.large_file_mode = document::is_large_file_size(
            state.ui_settings.large_file_threshold_mb,
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
        scintilla::set_wrap_enabled(
            doc_tab.editor,
            effective_wrap_enabled(
                doc_tab.wrap_enabled,
                doc_tab.doc.large_file_mode,
                state.ui_settings.large_file_disable_word_wrap,
            ),
        );
        apply_syntax_for_doc(doc_tab, state.editor_dark);
        scintilla::set_savepoint(doc_tab.editor);
        doc_tab.sticky_dirty = false;
        doc_tab.doc.is_dirty = false;
        doc_tab.change_counter = doc_tab.change_counter.saturating_add(1);
        doc_tab.last_backup_change_counter = None;
        doc_tab.doc.first_backup_write = None;
        doc_tab.doc.last_backup_write = None;
        doc_tab.doc.backup_path.clone()
    };
    apply_large_file_mode_restrictions(hwnd, state, index);
    if let Err(err) = session::delete_backup(&backup_path) {
        logging::log_error(&format!(
            "backup_delete_failed path={} err={err}",
            path.display()
        ));
    }
    update_tab_text(state, index);
    update_title(hwnd, state);
    update_status(state);
    if let Err(err) = save_session_checkpoint(hwnd, state) {
        logging::log_error(&format!("session_save_after_manual_save_failed err={err}"));
    }
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
            reload_doc_from_path(hwnd, state, index, &path)?;
        } else if let Some(doc_tab) = state.docs.get_mut(index) {
            doc_tab.doc.stamp = Some(new_stamp);
        }
    }

    Ok(())
}

/// Re-reads `path` into the tab at `index`, preserving the caret position
/// (clamped to the new length) and refreshing syntax, large-file mode, tab
/// text, title, and status.
fn reload_doc_from_path(
    hwnd: HWND,
    state: &mut AppState,
    index: usize,
    path: &PathBuf,
) -> Result<()> {
    {
        let doc_tab = match state.docs.get_mut(index) {
            Some(doc_tab) => doc_tab,
            None => return Ok(()),
        };
        let caret = scintilla::get_current_pos(doc_tab.editor);
        load_file_into_doc(
            doc_tab,
            path,
            state.ui_settings.large_file_threshold_mb,
            state.ui_settings.large_file_disable_word_wrap,
        )?;
        apply_syntax_for_doc(doc_tab, state.editor_dark);
        scintilla::goto_pos(
            doc_tab.editor,
            caret.min(scintilla::get_length(doc_tab.editor)),
        );
    }
    apply_large_file_mode_restrictions(hwnd, state, index);
    update_tab_text(state, index);
    update_title(hwnd, state);
    update_status(state);
    Ok(())
}

/// File > Reload from Disk: unconditionally re-reads the active tab's file,
/// asking for confirmation first when unsaved changes would be discarded.
fn reload_active_from_disk(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let index = state.active;
    let Some(doc_tab) = state.docs.get(index) else {
        return Ok(());
    };
    let Some(path) = doc_tab.doc.path.clone() else {
        return Ok(());
    };
    if !path.exists() {
        return Err(AppError::new(format!(
            "File no longer exists on disk:\n{}",
            path.display()
        )));
    }
    if doc_tab.doc.is_dirty && !prompt_discard_and_reload(hwnd) {
        return Ok(());
    }
    reload_doc_from_path(hwnd, state, index, &path)
}

fn prompt_discard_and_reload(hwnd: HWND) -> bool {
    let title = HSTRING::from("Reload from Disk");
    let message =
        HSTRING::from("This file has unsaved changes. Discard them and reload from disk?");
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
        } else if !doc_tab.doc.display_name.is_empty() {
            title = format!("Rivet - {}", doc_tab.doc.display_name);
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
    update_status_parts(state);
    let mut line = 1usize;
    let mut col = 1usize;
    let mut sel_len = 0usize;
    let mut words = String::new();
    let mut eol = "CRLF".to_string();
    let mut encoding = "UTF-8".to_string();
    let mut flags = String::new();
    if let Some(doc_tab) = state.docs.get(state.active) {
        let pos = scintilla::get_current_pos(doc_tab.editor);
        line = scintilla::line_from_position(doc_tab.editor, pos).saturating_add(1);
        col = scintilla::get_column(doc_tab.editor, pos).saturating_add(1);
        sel_len = scintilla::selection_end(doc_tab.editor)
            .abs_diff(scintilla::selection_start(doc_tab.editor));
        eol = eol_mode_label(scintilla::get_eol_mode(doc_tab.editor)).to_string();
        // Scintilla always holds UTF-8 internally, so report the document's
        // on-disk encoding instead of the buffer codepage.
        encoding = doc_tab.doc.encoding.label().to_string();
        // None means the count is suppressed (Large File Mode); leave blank.
        if let Some(count) = doc_tab.word_count {
            words = format!("Words: {}", format_thousands(count));
        }
        if doc_tab.doc.is_dirty {
            flags.push('*');
        }
        if doc_tab.doc.large_file_mode {
            if !flags.is_empty() {
                flags.push(' ');
            }
            flags.push_str("Large File Mode");
        }
        if doc_tab.smart_highlight_truncated {
            if !flags.is_empty() {
                flags.push(' ');
            }
            flags.push_str("Too many matches");
        }
    }
    if let Some(message) = state.status_message.as_deref() {
        if !flags.is_empty() {
            flags.push_str(" | ");
        }
        flags.push_str(message);
    }

    set_status_part_text(state.status, 0, &format!("Ln {line}, Col {col}"));
    set_status_part_text(state.status, 1, &format!("Sel {sel_len}"));
    set_status_part_text(state.status, 2, &words);
    set_status_part_text(state.status, 3, &format!("EOL: {eol}"));
    set_status_part_text(state.status, 4, &format!("ENC: {encoding}"));
    set_status_part_text(state.status, 5, &flags);
}

fn update_status_parts(state: &AppState) {
    let mut rect = windows::Win32::Foundation::RECT::default();
    unsafe {
        let _ = GetClientRect(state.status, &mut rect);
    }
    let width = rect.right - rect.left;
    let sel_width = scale_for_dpi(state.status, 90);
    let words_width = scale_for_dpi(state.status, 120);
    let eol_width = scale_for_dpi(state.status, 90);
    let enc_width = scale_for_dpi(state.status, 110);
    let dirty_width = scale_for_dpi(state.status, 220);
    let part0 = (width - sel_width - words_width - eol_width - enc_width - dirty_width).max(0);
    let part1 = (width - words_width - eol_width - enc_width - dirty_width).max(part0);
    let part2 = (width - eol_width - enc_width - dirty_width).max(part1);
    let part3 = (width - enc_width - dirty_width).max(part2);
    let part4 = (width - dirty_width).max(part3);
    let parts = [part0, part1, part2, part3, part4, -1];
    unsafe {
        SendMessageW(
            state.status,
            SB_SETPARTS,
            WPARAM(parts.len()),
            LPARAM(parts.as_ptr() as isize),
        );
    }
}

fn set_status_part_text(status: HWND, part: usize, text: &str) {
    let text = HSTRING::from(text);
    unsafe {
        SendMessageW(
            status,
            SB_SETTEXTW,
            WPARAM(part),
            LPARAM(text.as_ptr() as isize),
        );
    }
}

fn set_status_message(state: &mut AppState, message: impl Into<String>) {
    state.status_message = Some(message.into());
}

fn clear_status_message(state: &mut AppState) {
    state.status_message = None;
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
        update_status(state);
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

fn show_find_dialog(hwnd: HWND, state: &mut AppState, mode: SearchDialogMode) -> Result<()> {
    if active_editor(state).is_none() {
        return Ok(());
    }
    state.search_dialog_mode = mode;
    seed_search_text_from_selection(state);
    if let Some(dialog) = &state.find_dialog {
        unsafe {
            ShowWindow(dialog.hwnd, SW_SHOW);
            let _ = SetFocus(dialog.find_edit);
        }
        apply_search_state_to_dialog(state)?;
        return Ok(());
    }

    let instance = module_instance()?;
    let width = scale_for_dpi(hwnd, 460);
    let height = scale_for_dpi(hwnd, 240);
    let hwnd_dialog = unsafe {
        CreateWindowExW(
            Default::default(),
            FIND_CLASS,
            search_dialog_title(mode),
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

fn search_dialog_title(mode: SearchDialogMode) -> PCWSTR {
    match mode {
        SearchDialogMode::Find => w!("Find"),
        SearchDialogMode::Replace => w!("Replace"),
    }
}

fn seed_search_text_from_selection(state: &mut AppState) {
    if !state.search_state.find_text.is_empty() {
        return;
    }
    let Some(editor) = active_editor(state) else {
        return;
    };
    if scintilla::selection_empty(editor) {
        return;
    }
    let Ok(selected_text) = scintilla::selected_text(editor) else {
        return;
    };
    if selected_text.is_empty() || selected_text.contains('\r') || selected_text.contains('\n') {
        return;
    }
    state.search_state.find_text = selected_text;
}

fn show_go_to_line_dialog(hwnd: HWND, state: &mut AppState) -> Result<()> {
    if active_editor(state).is_none() {
        return Ok(());
    }
    if let Some(dialog) = &state.go_to_line_dialog {
        unsafe {
            ShowWindow(dialog.hwnd, SW_SHOW);
            let _ = SetFocus(dialog.line_edit);
        }
        apply_go_to_line_state(state)?;
        return Ok(());
    }

    let instance = module_instance()?;
    let width = scale_for_dpi(hwnd, 280);
    let height = scale_for_dpi(hwnd, 150);
    let hwnd_dialog = unsafe {
        CreateWindowExW(
            Default::default(),
            GOTO_LINE_CLASS,
            w!("Go To Line"),
            window_style(WS_CAPTION.0 | WS_SYSMENU.0),
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
        return Err(AppError::win32("CreateWindowExW(GoToLineDialog)"));
    }

    unsafe {
        EnableWindow(hwnd, BOOL(0));
        ShowWindow(hwnd_dialog, SW_SHOW);
    }
    Ok(())
}

fn apply_go_to_line_state(state: &AppState) -> Result<()> {
    let dialog = match &state.go_to_line_dialog {
        Some(dialog) => dialog,
        None => return Ok(()),
    };
    let editor = match active_editor(state) {
        Some(editor) => editor,
        None => return Ok(()),
    };
    let line_count = scintilla::line_count(editor).max(1);
    let current_line = scintilla::line_from_position(editor, scintilla::get_current_pos(editor))
        .saturating_add(1)
        .min(line_count);
    set_window_text(
        dialog.range_label,
        &format!("Line number (1 - {line_count}):"),
    );
    set_window_text(dialog.line_edit, &current_line.to_string());
    Ok(())
}

fn confirm_go_to_line(main_hwnd: HWND, state: &mut AppState) -> Result<()> {
    let dialog = match &state.go_to_line_dialog {
        Some(dialog) => dialog,
        None => return Ok(()),
    };
    let editor = match active_editor(state) {
        Some(editor) => editor,
        None => return Ok(()),
    };
    let line_count = scintilla::line_count(editor).max(1);
    let raw = get_window_text(dialog.line_edit)?;
    let requested = raw.trim().parse::<usize>().ok();
    let clamped = clamp_line_number(requested, line_count);
    scintilla::goto_line(editor, clamped.saturating_sub(1));
    update_status(state);
    close_go_to_line_dialog(main_hwnd, state);
    Ok(())
}

fn close_go_to_line_dialog(main_hwnd: HWND, state: &mut AppState) {
    if let Some(dialog) = state.go_to_line_dialog.take() {
        let focus_target = active_editor(state).unwrap_or(main_hwnd);
        unsafe {
            EnableWindow(main_hwnd, BOOL(1));
            let _ = SetFocus(focus_target);
            DestroyWindow(dialog.hwnd).ok();
        }
    }
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
    sync_search_state_from_dialog(state)?;
    if state.search_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, SearchDialogMode::Find)?;
        return Ok(());
    }
    state.search_state.last_direction = SearchDirection::Down;
    if let Some(editor) = active_editor(state) {
        if find_in_editor(editor, &state.search_state, SearchDirection::Down) {
            clear_status_message(state);
        } else {
            set_status_message(state, "Not found");
        }
        update_status(state);
    }
    Ok(())
}

fn perform_find_prev(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_search_state_from_dialog(state)?;
    if state.search_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, SearchDialogMode::Find)?;
        return Ok(());
    }
    state.search_state.last_direction = SearchDirection::Up;
    if let Some(editor) = active_editor(state) {
        if find_in_editor(editor, &state.search_state, SearchDirection::Up) {
            clear_status_message(state);
        } else {
            set_status_message(state, "Not found");
        }
        update_status(state);
    }
    Ok(())
}

fn perform_replace(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_search_state_from_dialog(state)?;
    if state.search_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, SearchDialogMode::Replace)?;
        return Ok(());
    }
    state.search_state.last_direction = SearchDirection::Down;
    if let Some(editor) = active_editor(state) {
        if replace_in_editor(editor, &state.search_state) {
            clear_status_message(state);
            let _ = find_in_editor(editor, &state.search_state, SearchDirection::Down);
        } else {
            set_status_message(state, "Not found");
        }
        update_status(state);
    }
    Ok(())
}

fn perform_replace_all(hwnd: HWND, state: &mut AppState) -> Result<()> {
    sync_search_state_from_dialog(state)?;
    if state.search_state.find_text.is_empty() {
        show_find_dialog(hwnd, state, SearchDialogMode::Replace)?;
        return Ok(());
    }
    if let Some(editor) = active_editor(state) {
        if replace_all_in_editor(editor, &state.search_state) == 0 {
            set_status_message(state, "Not found");
        } else {
            clear_status_message(state);
        }
        update_status(state);
    }
    Ok(())
}

fn sync_search_state_from_dialog(state: &mut AppState) -> Result<()> {
    if let Some(dialog) = &state.find_dialog {
        state.search_state.find_text = get_window_text(dialog.find_edit)?;
        state.search_state.replace_text = get_window_text(dialog.replace_edit)?;
        state.search_state.match_case = is_checked(dialog.match_case);
        state.search_state.whole_word = is_checked(dialog.whole_word);
        state.search_state.regex = is_checked(dialog.regex);
        state.search_state.wrap = is_checked(dialog.wrap);
    }
    Ok(())
}

fn apply_search_state_to_dialog(state: &AppState) -> Result<()> {
    if let Some(dialog) = &state.find_dialog {
        set_window_text(dialog.find_edit, &state.search_state.find_text);
        set_window_text(dialog.replace_edit, &state.search_state.replace_text);
        set_checked(dialog.match_case, state.search_state.match_case);
        set_checked(dialog.whole_word, state.search_state.whole_word);
        set_checked(dialog.regex, state.search_state.regex);
        set_checked(dialog.wrap, state.search_state.wrap);
        apply_search_dialog_mode(dialog, state.search_dialog_mode);
    }
    Ok(())
}

fn apply_search_dialog_mode(dialog: &FindDialogState, mode: SearchDialogMode) {
    set_window_text(dialog.hwnd, search_dialog_title_text(mode));
    let replace_visibility = match mode {
        SearchDialogMode::Find => SW_HIDE,
        SearchDialogMode::Replace => SW_SHOW,
    };
    unsafe {
        ShowWindow(dialog.replace_label, replace_visibility);
        ShowWindow(dialog.replace_edit, replace_visibility);
        ShowWindow(dialog.replace_btn, replace_visibility);
        ShowWindow(dialog.replace_all, replace_visibility);
    }
}

fn search_dialog_title_text(mode: SearchDialogMode) -> &'static str {
    match mode {
        SearchDialogMode::Find => "Find",
        SearchDialogMode::Replace => "Replace",
    }
}

fn find_in_editor(editor: HWND, state: &SearchState, direction: SearchDirection) -> bool {
    let found = find_match(editor, state, direction);
    if let Some((start, end)) = found {
        scintilla::set_selection(editor, start, end);
        true
    } else {
        false
    }
}

fn replace_in_editor(editor: HWND, state: &SearchState) -> bool {
    if state.find_text.is_empty() {
        return false;
    }
    let selection_start = scintilla::selection_start(editor);
    let selection_end = scintilla::selection_end(editor);
    let current_matches = selection_matches_search(editor, state);
    let (match_start, match_end) = if current_matches {
        (selection_start, selection_end)
    } else {
        match find_match(editor, state, state.last_direction) {
            Some(range) => range,
            None => return false,
        }
    };
    scintilla::set_target_range(editor, match_start, match_end);
    scintilla::replace_target(editor, &state.replace_text);
    let new_end = match_start.saturating_add(state.replace_text.len());
    scintilla::set_selection(editor, match_start, new_end);
    true
}

fn selection_matches_search(editor: HWND, state: &SearchState) -> bool {
    let selection_start = scintilla::selection_start(editor);
    let selection_end = scintilla::selection_end(editor);
    if selection_start == selection_end || state.find_text.is_empty() {
        return false;
    }
    let flags = search_flags(state);
    matches!(
        scintilla::search_in_target(
            editor,
            &state.find_text,
            flags,
            selection_start,
            selection_end,
        ),
        Some((match_start, match_end))
            if match_start == selection_start && match_end == selection_end
    )
}

fn replace_all_in_editor(editor: HWND, state: &SearchState) -> usize {
    if state.find_text.is_empty() {
        return 0;
    }
    let flags = search_flags(state);
    let mut doc_len = scintilla::get_length(editor);
    let mut search_start = 0usize;
    let replacement_len = state.replace_text.len();
    let mut replacements = 0usize;
    scintilla::begin_undo_action(editor);
    while let Some((start, end)) =
        scintilla::search_in_target(editor, &state.find_text, flags, search_start, doc_len)
    {
        scintilla::set_target_range(editor, start, end);
        scintilla::replace_target(editor, &state.replace_text);
        replacements = replacements.saturating_add(1);
        let progress = advance_replace_all_progress(doc_len, start, end, replacement_len);
        doc_len = progress.next_doc_len;
        search_start = progress.next_search_start;
        if search_start > doc_len {
            break;
        }
    }
    scintilla::end_undo_action(editor);
    replacements
}

fn find_match(
    editor: HWND,
    state: &SearchState,
    direction: SearchDirection,
) -> Option<(usize, usize)> {
    if state.find_text.is_empty() {
        return None;
    }
    let flags = search_flags(state);
    let doc_len = scintilla::get_length(editor);
    let caret = scintilla::get_current_pos(editor);
    let sel_start = scintilla::selection_start(editor);
    let sel_end = scintilla::selection_end(editor);
    let search_plan = build_search_plan(direction, state.wrap, doc_len, caret, sel_start, sel_end);
    search_in_search_range(editor, &state.find_text, flags, search_plan.primary).or_else(|| {
        search_plan
            .wrapped
            .and_then(|range| search_in_search_range(editor, &state.find_text, flags, range))
    })
}

fn search_in_search_range(
    editor: HWND,
    find_text: &str,
    flags: usize,
    range: SearchRange,
) -> Option<(usize, usize)> {
    if range.start == range.end {
        return None;
    }
    scintilla::search_in_target(editor, find_text, flags, range.start, range.end)
}

fn build_search_plan(
    direction: SearchDirection,
    wrap: bool,
    doc_len: usize,
    caret: usize,
    sel_start: usize,
    sel_end: usize,
) -> SearchPlan {
    let anchor = match direction {
        SearchDirection::Down if sel_start != sel_end => sel_end.min(doc_len),
        SearchDirection::Up if sel_start != sel_end => sel_start.min(doc_len),
        _ => caret.min(doc_len),
    };

    match direction {
        SearchDirection::Down => SearchPlan {
            primary: SearchRange {
                start: anchor,
                end: doc_len,
            },
            wrapped: wrap.then_some(SearchRange {
                start: 0,
                end: anchor,
            }),
        },
        SearchDirection::Up => SearchPlan {
            primary: SearchRange {
                start: anchor,
                end: 0,
            },
            wrapped: wrap.then_some(SearchRange {
                start: doc_len,
                end: anchor,
            }),
        },
    }
}

fn advance_replace_all_progress(
    doc_len: usize,
    start: usize,
    end: usize,
    replacement_len: usize,
) -> ReplaceAllProgress {
    let removed = end.saturating_sub(start);
    let next_doc_len = if replacement_len >= removed {
        doc_len.saturating_add(replacement_len - removed)
    } else {
        doc_len.saturating_sub(removed - replacement_len)
    };
    ReplaceAllProgress {
        next_search_start: start.saturating_add(replacement_len),
        next_doc_len,
    }
}

fn clamp_line_number(requested: Option<usize>, line_count: usize) -> usize {
    requested.unwrap_or(1).clamp(1, line_count.max(1))
}

fn search_flags(state: &SearchState) -> usize {
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

/// Current local date/time as "YYYY/MM/DD HH:MM", for Insert Date/Time.
fn current_date_time_stamp() -> String {
    let st = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute
    )
}

/// Formats a count with comma separators, e.g. 1234567 -> "1,234,567".
fn format_thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let offset = digits.len() % 3;
    for (index, ch) in digits.chars().enumerate() {
        if index != 0 && index % 3 == offset {
            out.push(',');
        }
        out.push(ch);
    }
    out
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

const EDITOR_SUBCLASS_ID: usize = 1;

thread_local! {
    /// Set when we handle Enter ourselves on a WM_KEYDOWN so the paired WM_CHAR
    /// (a `\r` produced by TranslateMessage) can be discarded before Scintilla
    /// turns it into a second newline.
    static SWALLOW_ENTER_CHAR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn create_editor(parent: HWND, instance: HINSTANCE, zoom: i32) -> Result<HWND> {
    let editor = scintilla::create_window(parent, instance)?;
    scintilla::initialize(editor);
    scintilla::set_zoom(editor, zoom);
    unsafe {
        let _ = SetWindowSubclass(editor, Some(editor_subclass_proc), EDITOR_SUBCLASS_ID, 0);
    }
    Ok(editor)
}

unsafe extern "system" fn editor_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id_subclass: usize,
    _ref_data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN {
        SWALLOW_ENTER_CHAR.with(|c| c.set(false));
        if wparam.0 as u32 == VK_RETURN.0 as u32 {
            let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
            let alt = unsafe { GetKeyState(VK_MENU.0 as i32) } < 0;
            if !ctrl && !alt && enter_below_collapsed_block(hwnd) {
                SWALLOW_ENTER_CHAR.with(|c| c.set(true));
                return LRESULT(0);
            }
        }
    } else if msg == WM_CHAR
        && SWALLOW_ENTER_CHAR.with(|c| c.replace(false))
        && (wparam.0 == 0x0D || wparam.0 == 0x0A)
    {
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// When the caret sits on a collapsed user-collapse header, Enter would insert
/// the newline *inside* the hidden region, making the new typing invisible.
/// Instead, add a fresh visible line just below the whole collapsed block and
/// put the caret there, leaving the collapse intact. Returns true when handled.
fn enter_below_collapsed_block(editor: HWND) -> bool {
    if !scintilla::selection_empty(editor) {
        return false;
    }
    let pos = scintilla::get_current_pos(editor);
    let line = scintilla::line_from_position(editor, pos);
    let marker_bit = 1u32 << scintilla::USER_COLLAPSE_MARKER;
    if scintilla::marker_get(editor, line) & marker_bit == 0 {
        return false;
    }
    let total = scintilla::line_count(editor);
    // Only act when the block below the header is actually hidden.
    if line + 1 >= total || scintilla::line_visible(editor, line + 1) {
        return false;
    }
    // Walk to the last hidden line of this block (mirrors expand_user_collapse).
    let mut last_hidden = line;
    let mut probe = line + 1;
    while probe < total {
        if scintilla::line_visible(editor, probe) {
            break;
        }
        if scintilla::marker_get(editor, probe) & marker_bit != 0 {
            break;
        }
        last_hidden = probe;
        probe += 1;
    }
    let eol = scintilla::eol_string(editor);
    let after = last_hidden + 1;
    let caret = if after >= total {
        // The collapsed block runs to EOF: append a new line at the very end.
        let end = scintilla::get_length(editor);
        scintilla::insert_text(editor, end, eol);
        end + eol.len()
    } else {
        // Insert an empty line at the start of the first visible line below the
        // block. The hidden range (line+1..=last_hidden) is unaffected.
        let insert_pos = scintilla::position_from_line(editor, after);
        scintilla::insert_text(editor, insert_pos, eol);
        insert_pos
    };
    // Guard against the new line inheriting the hidden flag of its neighbor.
    let caret_line = scintilla::line_from_position(editor, caret);
    if !scintilla::line_visible(editor, caret_line) {
        scintilla::show_lines(editor, caret_line, caret_line);
    }
    scintilla::goto_pos(editor, caret);
    true
}

/// The lexer actually used for a tab: a manual Language-menu override when set,
/// otherwise extension-based detection. Large File Mode always wins with Null to
/// keep highlighting off the hot path.
fn effective_lexer(doc_tab: &DocTab) -> scintilla::LexerKind {
    if doc_tab.doc.large_file_mode {
        return scintilla::LexerKind::Null;
    }
    doc_tab
        .lexer_override
        .unwrap_or_else(|| lexer_for_doc(&doc_tab.doc))
}

fn apply_syntax_for_doc(doc_tab: &DocTab, dark: bool) {
    let lexer = effective_lexer(doc_tab);
    scintilla::apply_lexer(doc_tab.editor, lexer, dark);
    apply_editor_theme_overlays(doc_tab.editor, dark);
    if matches!(lexer, scintilla::LexerKind::Markdown) && !doc_tab.doc.large_file_mode {
        apply_markdown_fold_levels(doc_tab.editor);
    }
}

/// Applies a Language-menu choice to the active tab and re-highlights it.
/// `over` is `None` for "Auto (by extension)" or `Some(kind)` to force a lexer.
fn set_language_override(state: &mut AppState, over: Option<scintilla::LexerKind>) {
    let index = state.active;
    let dark = state.editor_dark;
    let Some(doc_tab) = state.docs.get_mut(index) else {
        return;
    };
    if doc_tab.lexer_override == over {
        return;
    }
    doc_tab.lexer_override = over;
    apply_syntax_for_doc(doc_tab, dark);
}

/// Maps a Language-menu command id to the override it selects: `Some(None)` for
/// Auto, `Some(Some(kind))` for a specific lexer, `None` if not a language id.
fn lexer_override_for_command(id: u16) -> Option<Option<scintilla::LexerKind>> {
    use scintilla::LexerKind as K;
    let kind = match id {
        CMD_LANG_AUTO => return Some(None),
        CMD_LANG_PLAIN => K::Null,
        CMD_LANG_MARKDOWN => K::Markdown,
        CMD_LANG_CPP => K::Cpp,
        CMD_LANG_JAVASCRIPT => K::JavaScript,
        CMD_LANG_JSON => K::Json,
        CMD_LANG_YAML => K::Yaml,
        CMD_LANG_POWERSHELL => K::PowerShell,
        CMD_LANG_PYTHON => K::Python,
        CMD_LANG_HTML => K::Html,
        CMD_LANG_XML => K::Xml,
        CMD_LANG_CSS => K::Css,
        CMD_LANG_PROPERTIES => K::Properties,
        _ => return None,
    };
    Some(Some(kind))
}

/// The Language-menu command id that represents a forced lexer kind.
fn command_for_lexer_kind(kind: scintilla::LexerKind) -> u16 {
    use scintilla::LexerKind as K;
    match kind {
        K::Null => CMD_LANG_PLAIN,
        K::Markdown => CMD_LANG_MARKDOWN,
        K::Cpp => CMD_LANG_CPP,
        K::JavaScript => CMD_LANG_JAVASCRIPT,
        K::Json => CMD_LANG_JSON,
        K::Yaml => CMD_LANG_YAML,
        K::PowerShell => CMD_LANG_POWERSHELL,
        K::Python => CMD_LANG_PYTHON,
        K::Html => CMD_LANG_HTML,
        K::Xml => CMD_LANG_XML,
        K::Css => CMD_LANG_CSS,
        K::Properties => CMD_LANG_PROPERTIES,
    }
}

fn recompute_markdown_fold_levels(state: &AppState, index: usize) {
    let Some(doc_tab) = state.docs.get(index) else {
        return;
    };
    if doc_tab.doc.large_file_mode {
        return;
    }
    if !matches!(effective_lexer(doc_tab), scintilla::LexerKind::Markdown) {
        return;
    }
    apply_markdown_fold_levels(doc_tab.editor);
}

/// Debounce a markdown fold recompute. Recomputing on every keystroke walks
/// the whole document, so we coalesce edits behind a short timer and only act
/// on the active doc once typing settles.
fn schedule_markdown_fold(hwnd: HWND, state: &mut AppState, index: usize) {
    let is_markdown = state.docs.get(index).is_some_and(|doc_tab| {
        !doc_tab.doc.large_file_mode
            && matches!(effective_lexer(doc_tab), scintilla::LexerKind::Markdown)
    });
    if !is_markdown {
        return;
    }
    state.markdown_fold_pending = true;
    if !state.markdown_fold_timer {
        unsafe {
            let _ = SetTimer(hwnd, TIMER_MARKDOWN_FOLD, MARKDOWN_FOLD_INTERVAL_MS, None);
        }
        state.markdown_fold_timer = true;
    }
}

fn handle_markdown_fold_timer(hwnd: HWND, state: &mut AppState) {
    if state.markdown_fold_pending {
        state.markdown_fold_pending = false;
        recompute_markdown_fold_levels(state, state.active);
    }
    if state.markdown_fold_timer && !state.markdown_fold_pending {
        unsafe {
            let _ = KillTimer(hwnd, TIMER_MARKDOWN_FOLD);
        }
        state.markdown_fold_timer = false;
    }
}

fn apply_markdown_fold_levels(editor: HWND) {
    let text = match scintilla::get_text(editor) {
        Ok(t) => t,
        Err(_) => return,
    };
    let line_count = scintilla::line_count(editor);
    if line_count == 0 {
        return;
    }
    // Only write lines whose level actually changed — the lexer doesn't fold
    // markdown, so the stored level is exactly what we last set, and avoiding
    // redundant SCI_SETFOLDLEVEL keeps the debounced recompute cheap.
    let levels = markdown::compute_fold_levels(&text);
    for (line_idx, &level) in levels.iter().enumerate() {
        if line_idx >= line_count {
            break;
        }
        if scintilla::fold_level(editor, line_idx) != level {
            scintilla::set_fold_level(editor, line_idx, level);
        }
    }
}

fn apply_editor_theme_overlays(editor: HWND, dark: bool) {
    let (smart_color, fill_alpha, outline_alpha) = if dark {
        (color_ref(90, 140, 220).0, 52usize, 80usize)
    } else {
        (color_ref(104, 145, 210).0, 60usize, 95usize)
    };
    scintilla::configure_smart_highlight_indicator(
        editor,
        SMART_HL_INDIC,
        smart_color,
        fill_alpha,
        outline_alpha,
    );

    let strike_color = if dark {
        color_ref(224, 156, 156).0
    } else {
        color_ref(155, 92, 92).0
    };
    scintilla::configure_strike_indicator(editor, STRIKE_INDIC, strike_color);

    let hidden_line_color = if dark {
        color_ref(114, 160, 230).0
    } else {
        color_ref(120, 120, 120).0
    };
    scintilla::set_hidden_line_color(editor, hidden_line_color);

    let (gutter_fg, gutter_bg, marker_fg, marker_bg) = if dark {
        (
            color_ref(140, 140, 140).0,
            color_ref(30, 30, 30).0,
            color_ref(180, 180, 180).0,
            color_ref(30, 30, 30).0,
        )
    } else {
        (
            color_ref(120, 120, 120).0,
            color_ref(246, 246, 246).0,
            color_ref(90, 90, 90).0,
            color_ref(246, 246, 246).0,
        )
    };
    scintilla::set_line_number_style(editor, gutter_fg, gutter_bg);
    scintilla::set_fold_marker_colors(editor, marker_fg, marker_bg);
    let line_count = scintilla::line_count(editor);
    scintilla::set_line_number_margin_width(editor, line_count);
    let fold_width = scale_for_dpi(editor, 16);
    scintilla::set_fold_margin_width(editor, fold_width);
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
        Some("md") | Some("markdown") => scintilla::LexerKind::Markdown,
        _ => scintilla::LexerKind::Null,
    }
}

fn create_doc_from_path(
    parent: HWND,
    instance: HINSTANCE,
    path: PathBuf,
    wrap_enabled: bool,
    large_file_threshold_mb: u32,
    large_file_disable_word_wrap: bool,
    zoom: i32,
) -> Result<DocTab> {
    let editor = create_editor(parent, instance, zoom)?;
    let mut doc = Document::new_empty();
    doc.backup_path = session::backup_path_for_id(doc.id)?;
    let mut doc_tab = DocTab {
        runtime_id: 0,
        editor,
        doc,
        wrap_enabled,
        sticky_dirty: false,
        word_count: Some(0),
        change_counter: 0,
        last_backup_change_counter: None,
        smart_highlight_token: None,
        smart_highlight_truncated: false,
        lexer_override: None,
    };
    load_file_into_doc(
        &mut doc_tab,
        &path,
        large_file_threshold_mb,
        large_file_disable_word_wrap,
    )?;
    Ok(doc_tab)
}

fn load_file_into_doc(
    doc_tab: &mut DocTab,
    path: &PathBuf,
    large_file_threshold_mb: u32,
    large_file_disable_word_wrap: bool,
) -> Result<()> {
    let bytes =
        std::fs::read(path).map_err(|err| AppError::new(format!("Failed to read file: {err}")))?;
    let (text, encoding) = document::decode_bytes(&bytes)?;
    let eol = document::detect_eol(&text);
    let stamp = document::FileStamp::from_path(path)?;
    let large_file_mode = document::is_large_file_size(large_file_threshold_mb, stamp.size);

    scintilla::set_text(doc_tab.editor, &text)?;
    scintilla::set_eol_mode(doc_tab.editor, eol);

    scintilla::set_wrap_enabled(
        doc_tab.editor,
        effective_wrap_enabled(
            doc_tab.wrap_enabled,
            large_file_mode,
            large_file_disable_word_wrap,
        ),
    );
    scintilla::set_savepoint(doc_tab.editor);

    doc_tab
        .doc
        .update_from_load(path.clone(), encoding, eol, stamp, large_file_mode);
    doc_tab.doc.display_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();
    doc_tab.doc.is_dirty = false;
    doc_tab.sticky_dirty = false;
    doc_tab.doc.first_backup_write = None;
    doc_tab.doc.last_backup_write = None;
    doc_tab.change_counter = 0;
    doc_tab.last_backup_change_counter = None;
    doc_tab.word_count = if large_file_mode {
        None
    } else {
        Some(count_words(&text))
    };
    Ok(())
}

fn create_empty_tab(hwnd: HWND, instance: HINSTANCE, state: &mut AppState) -> Result<()> {
    let editor = create_editor(hwnd, instance, state.ui_settings.zoom_level)?;
    let mut doc = Document::new_empty();
    doc.display_name = next_untitled_name(state);
    doc.backup_path = session::backup_path_for_id(doc.id)?;
    let doc_tab = DocTab {
        runtime_id: 0,
        editor,
        doc,
        wrap_enabled: state.word_wrap_enabled,
        sticky_dirty: false,
        word_count: Some(0),
        change_counter: 0,
        last_backup_change_counter: None,
        smart_highlight_token: None,
        smart_highlight_truncated: false,
        lexer_override: None,
    };
    scintilla::set_eol_mode(editor, doc_tab.doc.eol);
    scintilla::set_wrap_enabled(editor, state.word_wrap_enabled);
    scintilla::set_savepoint(editor);
    apply_syntax_for_doc(&doc_tab, state.editor_dark);

    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    apply_large_file_mode_restrictions(hwnd, state, index);
    Ok(())
}

fn duplicate_active_tab(hwnd: HWND, state: &mut AppState) -> Result<()> {
    let source = state
        .docs
        .get(state.active)
        .ok_or_else(|| AppError::new("No active document."))?;
    let text = scintilla::get_text(source.editor)?;
    let strike_ranges = collect_strike_ranges(source.editor);
    let instance = module_instance()?;
    let editor = create_editor(hwnd, instance, state.ui_settings.zoom_level)?;
    scintilla::set_text(editor, &text)?;
    scintilla::set_eol_mode(editor, source.doc.eol);
    let encoded_size = document::encoded_size_for_text(&text, source.doc.encoding);
    let large_file_mode =
        document::is_large_file_size(state.ui_settings.large_file_threshold_mb, encoded_size);
    let wrap_enabled = state.word_wrap_enabled;
    scintilla::set_wrap_enabled(
        editor,
        effective_wrap_enabled(
            wrap_enabled,
            large_file_mode,
            state.ui_settings.large_file_disable_word_wrap,
        ),
    );

    let mut doc = Document::new_empty();
    doc.encoding = source.doc.encoding;
    doc.eol = source.doc.eol;
    doc.large_file_mode = large_file_mode;
    doc.display_name = format!("Copy of {}", tab_base_name(source));
    doc.backup_path = session::backup_path_for_id(doc.id)?;
    doc.is_dirty = true;

    let doc_tab = DocTab {
        runtime_id: 0,
        editor,
        doc,
        wrap_enabled,
        sticky_dirty: true,
        word_count: if large_file_mode {
            None
        } else {
            Some(count_words(&text))
        },
        change_counter: 1,
        last_backup_change_counter: None,
        smart_highlight_token: None,
        smart_highlight_truncated: false,
        lexer_override: None,
    };
    apply_syntax_for_doc(&doc_tab, state.editor_dark);
    restore_strike_ranges(doc_tab.editor, &strike_ranges);

    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    select_tab(hwnd, state, index);
    apply_large_file_mode_restrictions(hwnd, state, index);
    Ok(())
}

fn tab_base_name(doc_tab: &DocTab) -> String {
    if !doc_tab.doc.display_name.is_empty() {
        doc_tab.doc.display_name.clone()
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
    if doc_tab.doc.is_dirty {
        title = format!("{title}*");
    }
    title
}

fn take_next_tab_runtime_id(state: &mut AppState) -> isize {
    let id = state.next_tab_runtime_id;
    state.next_tab_runtime_id = state.next_tab_runtime_id.saturating_add(1);
    id as isize
}

fn add_tab(state: &mut AppState, title: &str, mut doc_tab: DocTab) -> Result<usize> {
    let index = state.docs.len();
    if doc_tab.runtime_id == 0 {
        doc_tab.runtime_id = take_next_tab_runtime_id(state);
    }
    insert_tab_item(state.tab_host.top_tabs, index, title)?;
    state.docs.push(doc_tab);
    rebuild_vertical_tab_list(state);
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
                state.tab_host.top_tabs,
                TCM_SETITEMW,
                WPARAM(index),
                LPARAM(&mut item as *mut TCITEMW as isize),
            );
        }
        rebuild_vertical_tab_list(state);
        unsafe {
            InvalidateRect(state.tab_host.top_tabs, None, true);
        }
    }
}

fn rebuild_vertical_tab_list(state: &mut AppState) {
    unsafe {
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_DELETEALLITEMS,
            WPARAM(0),
            LPARAM(0),
        );
    }
    for (index, doc_tab) in state.docs.iter().enumerate() {
        let title = tab_title(doc_tab);
        let mut wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: index as i32,
            pszText: PWSTR(wide.as_mut_ptr()),
            cchTextMax: wide.len() as i32,
            lParam: LPARAM(doc_tab.runtime_id),
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                state.tab_host.vertical_tabs,
                LVM_INSERTITEMW,
                WPARAM(0),
                LPARAM(&mut item as *mut LVITEMW as isize),
            );
        }
    }
    set_vertical_tab_selection(state, state.active);
}

fn set_vertical_tab_selection(state: &AppState, index: usize) {
    if state.docs.is_empty() || index >= state.docs.len() {
        return;
    }
    let select_mask = LVIS_SELECTED.0 | LVIS_FOCUSED.0;
    let mut clear = LVITEMW {
        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(select_mask),
        state: LIST_VIEW_ITEM_STATE_FLAGS(0),
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_SETITEMSTATE,
            WPARAM(usize::MAX),
            LPARAM(&mut clear as *mut LVITEMW as isize),
        );
    }
    let mut selected = LVITEMW {
        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(select_mask),
        state: LIST_VIEW_ITEM_STATE_FLAGS(select_mask),
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_SETITEMSTATE,
            WPARAM(index),
            LPARAM(&mut selected as *mut LVITEMW as isize),
        );
    }
}

fn select_tab(hwnd: HWND, state: &mut AppState, index: usize) {
    if index >= state.docs.len() {
        return;
    }

    state.active = index;
    if let Some(doc_tab) = state.docs.get_mut(index) {
        doc_tab.doc.cursor_pos = scintilla::get_current_pos(doc_tab.editor) as i64;
    }
    update_tab_text(state, index);
    unsafe {
        SendMessageW(
            state.tab_host.top_tabs,
            TCM_SETCURSEL,
            WPARAM(index),
            LPARAM(0),
        );
    }
    set_vertical_tab_selection(state, index);

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
    clear_status_message(state);
    update_smart_highlight_for_doc(state, index, true);

    update_title(hwnd, state);
    update_status(state);
    update_wrap_menu(hwnd, state);
    update_copy_path_menu(hwnd, state);
    layout_children(hwnd, state);
}

fn select_adjacent_tab(hwnd: HWND, state: &mut AppState, next: bool) {
    let count = state.docs.len();
    if count <= 1 {
        return;
    }
    let index = if next {
        (state.active + 1) % count
    } else {
        (state.active + count - 1) % count
    };
    select_tab(hwnd, state, index);
}

fn ordered_selection_range(editor: HWND) -> (usize, usize) {
    let a = scintilla::selection_start(editor);
    let b = scintilla::selection_end(editor);
    if a <= b { (a, b) } else { (b, a) }
}

fn collapse_selection_lines(editor: HWND) -> bool {
    let (start_pos, end_pos) = ordered_selection_range(editor);
    let line_start = scintilla::line_from_position(editor, start_pos);
    let line_end = if start_pos == end_pos {
        line_start
    } else {
        scintilla::line_from_position(editor, end_pos)
    };
    if line_end <= line_start {
        return false;
    }
    let total = scintilla::line_count(editor);
    if line_start + 1 >= total {
        return false;
    }
    let hide_end = line_end.min(total - 1);
    scintilla::marker_add(editor, line_start, scintilla::USER_COLLAPSE_MARKER);
    scintilla::hide_lines(editor, line_start + 1, hide_end);
    true
}

fn expand_user_collapse(editor: HWND, header_line: usize) {
    let total = scintilla::line_count(editor);
    let mut last_hidden = header_line;
    let mut line = header_line + 1;
    while line < total {
        if scintilla::line_visible(editor, line) {
            break;
        }
        let m = scintilla::marker_get(editor, line);
        if (m & (1u32 << scintilla::USER_COLLAPSE_MARKER)) != 0 {
            break;
        }
        last_hidden = line;
        line += 1;
    }
    if last_hidden > header_line {
        scintilla::show_lines(editor, header_line + 1, last_hidden);
    }
    scintilla::marker_delete(editor, header_line, scintilla::USER_COLLAPSE_MARKER);
}

fn expand_all_user_collapses(editor: HWND) {
    let total = scintilla::line_count(editor);
    if total == 0 {
        return;
    }
    scintilla::show_lines(editor, 0, total - 1);
    scintilla::marker_delete_all(editor, scintilla::USER_COLLAPSE_MARKER as i32);
}

fn toggle_strikethrough(editor: HWND) -> bool {
    let (start, end) = ordered_selection_range(editor);
    if start >= end {
        return false;
    }

    scintilla::set_indicator_current(editor, STRIKE_INDIC);
    scintilla::set_indicator_value(editor, STRIKE_INDIC_VALUE);
    if selection_fully_struck(editor, start, end) {
        scintilla::clear_indicator_range(editor, start, end - start);
    } else {
        scintilla::fill_indicator_range(editor, start, end - start);
    }
    true
}

fn selection_fully_struck(editor: HWND, start: usize, end: usize) -> bool {
    let mut pos = start;
    while pos < end {
        if scintilla::indicator_value_at(editor, STRIKE_INDIC, pos) <= 0 {
            return false;
        }
        let run_end = scintilla::indicator_end(editor, STRIKE_INDIC, pos).min(end);
        if run_end <= pos {
            return false;
        }
        pos = run_end;
    }
    true
}

fn collect_strike_ranges(editor: HWND) -> Vec<session::StrikeRange> {
    let len = scintilla::get_length(editor);
    let mut pos = 0usize;
    let mut ranges = Vec::new();
    while pos < len {
        let flags = scintilla::indicator_all_on_for(editor, pos);
        let next = scintilla::indicator_end(editor, STRIKE_INDIC, pos).min(len);
        if (flags & (1u32 << STRIKE_INDIC)) != 0 {
            let start = scintilla::indicator_start(editor, STRIKE_INDIC, pos).min(pos);
            let end = next.max(pos.saturating_add(1));
            if end > start {
                ranges.push(session::StrikeRange {
                    start: start as i64,
                    end: end as i64,
                });
            }
        }
        pos = next.max(pos.saturating_add(1));
    }
    ranges
}

fn restore_strike_ranges(editor: HWND, ranges: &[session::StrikeRange]) {
    let len = scintilla::get_length(editor);
    if len == 0 || ranges.is_empty() {
        return;
    }

    scintilla::set_indicator_current(editor, STRIKE_INDIC);
    scintilla::set_indicator_value(editor, STRIKE_INDIC_VALUE);
    scintilla::clear_indicator_range(editor, 0, len);
    for range in ranges {
        let start = range.start.max(0) as usize;
        let end = range.end.max(0) as usize;
        let start = start.min(len);
        let end = end.min(len);
        if end > start {
            scintilla::fill_indicator_range(editor, start, end - start);
        }
    }
}

fn update_smart_highlight_for_doc(state: &mut AppState, index: usize, force: bool) {
    if index >= state.docs.len() {
        return;
    }
    let smart_enabled = state.ui_settings.smart_highlight_enabled;
    let disable_for_large = state.ui_settings.large_file_disable_smart_highlight;
    let match_case = state.ui_settings.smart_highlight_match_case;
    let whole_word = state.ui_settings.smart_highlight_whole_word;

    let doc_tab = &mut state.docs[index];
    if !smart_highlight_allowed(
        smart_enabled,
        doc_tab.doc.large_file_mode,
        disable_for_large,
    ) {
        clear_smart_highlight(doc_tab);
        return;
    }

    let token = selection_token_for_smart_highlight(doc_tab.editor, whole_word);
    let Some(token) = token else {
        clear_smart_highlight(doc_tab);
        return;
    };

    if !force
        && doc_tab
            .smart_highlight_token
            .as_ref()
            .is_some_and(|existing| existing == &token)
    {
        return;
    }

    let doc_len = scintilla::get_length(doc_tab.editor);
    scintilla::set_indicator_current(doc_tab.editor, SMART_HL_INDIC);
    scintilla::clear_indicator_range(doc_tab.editor, 0, doc_len);

    let mut flags = 0usize;
    if match_case {
        flags |= SCFIND_MATCHCASE;
    }
    if whole_word {
        flags |= SCFIND_WHOLEWORD;
    }

    let mut start = 0usize;
    let mut count = 0usize;
    let mut truncated = false;
    while start < doc_len {
        let Some((match_start, match_end)) =
            scintilla::search_in_target(doc_tab.editor, &token, flags, start, doc_len)
        else {
            break;
        };

        if match_end <= match_start {
            break;
        }
        scintilla::fill_indicator_range(doc_tab.editor, match_start, match_end - match_start);
        count = count.saturating_add(1);
        if count >= SMART_HL_MAX_MATCHES {
            truncated = true;
            break;
        }
        start = match_end;
    }

    doc_tab.smart_highlight_token = Some(token);
    doc_tab.smart_highlight_truncated = truncated;
}

fn clear_smart_highlight(doc_tab: &mut DocTab) {
    if doc_tab.smart_highlight_token.is_none() && !doc_tab.smart_highlight_truncated {
        return;
    }
    let len = scintilla::get_length(doc_tab.editor);
    scintilla::set_indicator_current(doc_tab.editor, SMART_HL_INDIC);
    scintilla::clear_indicator_range(doc_tab.editor, 0, len);
    doc_tab.smart_highlight_token = None;
    doc_tab.smart_highlight_truncated = false;
}

fn selection_token_for_smart_highlight(editor: HWND, whole_word: bool) -> Option<String> {
    if scintilla::selection_empty(editor) {
        return None;
    }
    let token = scintilla::selected_text(editor).ok()?;
    if token.is_empty()
        || token.len() > SMART_HL_MAX_TOKEN_LEN
        || token.contains('\r')
        || token.contains('\n')
    {
        return None;
    }
    if whole_word && !is_word_like_token(&token) {
        return None;
    }
    Some(token)
}

fn is_word_like_token(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

fn tab_bar_height(state: &AppState) -> i32 {
    let min_height = scale_for_dpi(state.tab_host.top_tabs, 26);
    if state.docs.is_empty() {
        return min_height;
    }
    let mut rect = windows::Win32::Foundation::RECT::default();
    let result = unsafe {
        SendMessageW(
            state.tab_host.top_tabs,
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

const TAB_ITEM_MIN_WIDTH: i32 = 80;
const TAB_ITEM_LABEL_PADDING: i32 = 8;

unsafe extern "system" fn top_tabs_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id_subclass: usize,
    _ref_data: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            let parent = unsafe { GetParent(hwnd) };
            if let Some(state) = get_state(parent) {
                let mut rect = windows::Win32::Foundation::RECT::default();
                unsafe {
                    let _ = GetClientRect(hwnd, &mut rect);
                }
                let brush = unsafe { CreateSolidBrush(state.tab_host.theme.bg) };
                if brush.0 != 0 {
                    unsafe {
                        let _ = FillRect(HDC(wparam.0 as isize), &rect, brush);
                        let _ = DeleteObject(brush);
                    }
                }
            }
            return LRESULT(1);
        }
        WM_PAINT => {
            // SysTabControl32 does not reliably honor NM_CUSTOMDRAW for full
            // item paint, so we bypass it: handle WM_PAINT directly and walk
            // the tabs ourselves. This is what gives us dark-theme tabs and a
            // visible close ×.
            let parent = unsafe { GetParent(hwnd) };
            let Some(state) = get_state(parent) else {
                return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
            };
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if hdc.0 != 0 {
                paint_top_tab_strip(hwnd, hdc, state);
                unsafe {
                    let _ = EndPaint(hwnd, &ps);
                }
            }
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(top_tabs_subclass_proc), 0);
            }
            return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn status_bar_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id_subclass: usize,
    _ref_data: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            return LRESULT(1);
        }
        WM_PAINT => {
            let parent = unsafe { GetParent(hwnd) };
            let theme_opt = get_state(parent).map(|s| s.tab_host.theme);
            let Some(theme) = theme_opt else {
                return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
            };

            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if hdc.0 == 0 {
                return LRESULT(0);
            }

            let mut client = windows::Win32::Foundation::RECT::default();
            unsafe {
                let _ = GetClientRect(hwnd, &mut client);
            }
            let bg_brush = unsafe { CreateSolidBrush(theme.bg) };
            if bg_brush.0 != 0 {
                unsafe {
                    let _ = FillRect(hdc, &client, bg_brush);
                    let _ = DeleteObject(bg_brush);
                }
            }

            let part_count =
                unsafe { SendMessageW(hwnd, SB_GETPARTS, WPARAM(0), LPARAM(0)).0 } as i32;
            let parts = part_count.max(1);

            let hfont = unsafe { SendMessageW(hwnd, WM_GETFONT, WPARAM(0), LPARAM(0)) };
            let restore_font = if hfont.0 != 0 {
                Some(unsafe { SelectObject(hdc, HGDIOBJ(hfont.0)) })
            } else {
                None
            };
            unsafe {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, theme.fg);
            }

            let border_pen = unsafe { CreatePen(PS_SOLID, 1, theme.border) };

            for part in 0..parts {
                let mut rect = windows::Win32::Foundation::RECT::default();
                unsafe {
                    SendMessageW(
                        hwnd,
                        SB_GETRECT,
                        WPARAM(part as usize),
                        LPARAM(&mut rect as *mut _ as isize),
                    );
                }

                let text_len = unsafe {
                    SendMessageW(hwnd, SB_GETTEXTLENGTHW, WPARAM(part as usize), LPARAM(0)).0
                };
                let low = (text_len as u32) & 0xFFFF;
                let mut buf: Vec<u16> = vec![0u16; (low as usize) + 1];
                if low > 0 {
                    unsafe {
                        SendMessageW(
                            hwnd,
                            SB_GETTEXTW,
                            WPARAM(part as usize),
                            LPARAM(buf.as_mut_ptr() as isize),
                        );
                    }
                }

                let mut text_rect = rect;
                text_rect.left += scale_for_dpi(hwnd, 6);
                text_rect.right -= scale_for_dpi(hwnd, 6);
                if low > 0 {
                    unsafe {
                        DrawTextW(
                            hdc,
                            &mut buf[..low as usize],
                            &mut text_rect,
                            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
                        );
                    }
                }

                if part + 1 < parts && border_pen.0 != 0 {
                    let prev = unsafe { SelectObject(hdc, HGDIOBJ(border_pen.0)) };
                    unsafe {
                        let _ = MoveToEx(hdc, rect.right - 1, rect.top + 2, None);
                        let _ = LineTo(hdc, rect.right - 1, rect.bottom - 2);
                        SelectObject(hdc, prev);
                    }
                }
            }

            if border_pen.0 != 0 {
                unsafe {
                    let _ = DeleteObject(border_pen);
                }
            }
            if let Some(prev) = restore_font {
                unsafe { SelectObject(hdc, prev) };
            }

            unsafe {
                let _ = EndPaint(hwnd, &ps);
            }
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(status_bar_subclass_proc), 0);
            }
            return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn doc_index_by_hwnd(state: &AppState, hwnd: HWND) -> Option<usize> {
    state.docs.iter().position(|doc| doc.editor == hwnd)
}

fn doc_index_by_runtime_id(state: &AppState, runtime_id: LPARAM) -> Option<usize> {
    state
        .docs
        .iter()
        .position(|doc| doc.runtime_id == runtime_id.0)
}

fn set_dirty(state: &mut AppState, index: usize, dirty: bool) {
    if let Some(doc_tab) = state.docs.get_mut(index) {
        let effective_dirty = dirty || doc_tab.sticky_dirty;
        if doc_tab.doc.is_dirty != effective_dirty {
            doc_tab.doc.is_dirty = effective_dirty;
            update_tab_text(state, index);
            if index == state.active {
                update_status(state);
            }
        }
    }
}

fn confirm_close_all(hwnd: HWND, state: &mut AppState) -> Result<bool> {
    for index in 0..state.docs.len() {
        let doc_tab = &state.docs[index];
        if !state.session_snapshot_periodic_backup && doc_tab.doc.is_dirty {
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

    if state.session_snapshot_periodic_backup
        && state
            .docs
            .get(index)
            .is_some_and(|doc_tab| doc_tab.doc.is_dirty)
    {
        backup_doc_at_index(state, index, true)?;
    }

    let should_close = {
        let doc_tab = &state.docs[index];
        if !state.session_snapshot_periodic_backup && doc_tab.doc.is_dirty {
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
    if let Err(err) = session::delete_backup(&doc.doc.backup_path) {
        logging::log_error(&format!(
            "backup_delete_failed_on_close path={} err={err}",
            doc.doc.backup_path.display()
        ));
    }
    unsafe {
        let _ = DestroyWindow(doc.editor);
        SendMessageW(
            state.tab_host.top_tabs,
            TCM_DELETEITEM,
            WPARAM(index),
            LPARAM(0),
        );
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
    unsafe {
        InvalidateRect(state.tab_host.top_tabs, None, true);
    }
    if let Err(err) = save_session_checkpoint(hwnd, state) {
        logging::log_error(&format!("session_save_after_tab_close_failed err={err}"));
    }
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
        .unwrap_or(doc_tab.doc.display_name.as_str());
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

fn apply_saved_window_placement(hwnd: HWND, placement: &session::WindowPlacementData) -> bool {
    let rect = RECT {
        left: placement.x,
        top: placement.y,
        right: placement.x + placement.width,
        bottom: placement.y + placement.height,
    };
    // rcNormalPosition is captured in workspace coordinates while
    // MonitorFromRect expects screen coordinates; the taskbar offset is
    // acceptable for this "still on a monitor?" check, and the restore itself
    // is exact because we feed back the coordinates we captured.
    let monitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONULL) };
    if monitor.0 == 0 {
        return false;
    }
    let show_cmd = if placement.maximized {
        SW_SHOWMAXIMIZED
    } else {
        SW_SHOWNORMAL
    };
    let wp = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        flags: WINDOWPLACEMENT_FLAGS(0),
        showCmd: show_cmd.0 as u32,
        ptMinPosition: POINT { x: -1, y: -1 },
        ptMaxPosition: POINT { x: -1, y: -1 },
        rcNormalPosition: rect,
    };
    unsafe { SetWindowPlacement(hwnd, &wp).is_ok() }
}

fn restore_session(hwnd: HWND, mut state: AppState) -> Result<AppState> {
    let snapshot = match session::load_session() {
        Ok(snapshot) => snapshot,
        Err(_) => return Ok(state),
    };

    state.remember_session = snapshot.remember_session;
    state.session_snapshot_periodic_backup = snapshot.session_snapshot_periodic_backup;
    state.backup_interval_seconds = snapshot.backup_interval_seconds.max(1);
    state.word_wrap_enabled = snapshot.word_wrap_enabled;
    state.always_on_top = snapshot.always_on_top;
    state.window_placement = snapshot.window_placement;

    if !state.remember_session {
        return Ok(state);
    }

    for entry in snapshot.entries {
        if let Err(err) = restore_session_entry(hwnd, &mut state, entry) {
            logging::log_error(&format!("session_restore_entry_failed err={err}"));
        }
    }

    if !state.docs.is_empty() {
        if let Some(active_id) = snapshot.active_tab_id
            && let Some(index) = state.docs.iter().position(|doc| doc.doc.id == active_id)
        {
            state.active = index;
        } else {
            state.active = 0;
        }
    }

    Ok(state)
}

fn restore_session_entry(
    hwnd: HWND,
    state: &mut AppState,
    entry: session::SessionEntry,
) -> Result<()> {
    let backup_modified = session::modified_time(&entry.backup_path).ok();
    let disk_modified = entry
        .path
        .as_ref()
        .and_then(|path| session::modified_time(path).ok());
    let restore_source = session::decide_restore_source(&session::RestoreDecisionInput {
        remember_session: state.remember_session,
        path: entry.path.clone(),
        backup_path: entry.backup_path.clone(),
        was_dirty_at_exit: entry.is_dirty,
        backup_modified,
        disk_modified,
    });
    if restore_source == session::RestoreSource::Skip {
        return Ok(());
    }

    let (text, encoding, stamp, large_file_mode) = match restore_source {
        session::RestoreSource::Disk => {
            let path = entry
                .path
                .as_ref()
                .ok_or_else(|| AppError::new("Missing disk path for restore."))?;
            let bytes = std::fs::read(path)
                .map_err(|err| AppError::new(format!("Failed to read restore path: {err}")))?;
            let (text, encoding) = document::decode_bytes(&bytes)?;
            let stamp = document::FileStamp::from_path(path)?;
            let large =
                document::is_large_file_size(state.ui_settings.large_file_threshold_mb, stamp.size);
            (text, encoding, Some(stamp), large)
        }
        session::RestoreSource::Backup => {
            let bytes = std::fs::read(&entry.backup_path)
                .map_err(|err| AppError::new(format!("Failed to read backup file: {err}")))?;
            let text = String::from_utf8(bytes.clone()).or_else(|_| {
                document::decode_bytes(&bytes)
                    .map(|(decoded, _)| decoded)
                    .map_err(|err| AppError::new(format!("Failed to decode backup file: {err}")))
            })?;
            let stamp = entry
                .path
                .as_ref()
                .and_then(|path| document::FileStamp::from_path(path).ok());
            let large = stamp
                .as_ref()
                .map(|value| {
                    document::is_large_file_size(
                        state.ui_settings.large_file_threshold_mb,
                        value.size,
                    )
                })
                .unwrap_or_else(|| {
                    document::is_large_file_size(
                        state.ui_settings.large_file_threshold_mb,
                        bytes.len() as u64,
                    )
                });
            (text, TextEncoding::Utf8, stamp, large)
        }
        session::RestoreSource::Skip => return Ok(()),
    };

    let instance = module_instance()?;
    let editor = create_editor(hwnd, instance, state.ui_settings.zoom_level)?;
    scintilla::set_text(editor, &text)?;

    let mut doc = Document::with_id(entry.id);
    doc.path = entry.path.clone();
    doc.display_name = if entry.display_name.is_empty() {
        doc.path
            .as_ref()
            .and_then(|path| path.file_name().and_then(|name| name.to_str()))
            .unwrap_or("Untitled")
            .to_string()
    } else {
        entry.display_name
    };
    doc.backup_path = entry.backup_path;
    doc.is_dirty = entry.is_dirty;
    doc.encoding = encoding;
    doc.encoding_hint = Some(encoding);
    doc.eol = document::detect_eol(&text);
    doc.stamp = stamp;
    doc.large_file_mode = large_file_mode;
    doc.cursor_pos = entry.cursor_pos;
    doc.scroll_pos = 0;
    doc.first_backup_write = None;
    doc.last_backup_write = entry.backup_timestamp.map(unix_millis_to_system_time);
    ensure_doc_backup_path(&mut doc)?;

    let wrap_enabled = state.word_wrap_enabled;
    scintilla::set_eol_mode(editor, doc.eol);
    scintilla::set_wrap_enabled(
        editor,
        effective_wrap_enabled(
            wrap_enabled,
            doc.large_file_mode,
            state.ui_settings.large_file_disable_word_wrap,
        ),
    );
    scintilla::set_savepoint(editor);

    let doc_tab = DocTab {
        runtime_id: 0,
        editor,
        doc,
        wrap_enabled,
        sticky_dirty: restore_source == session::RestoreSource::Backup && entry.is_dirty,
        word_count: if large_file_mode {
            None
        } else {
            Some(count_words(&text))
        },
        change_counter: 0,
        last_backup_change_counter: if restore_source == session::RestoreSource::Backup {
            Some(0)
        } else {
            None
        },
        smart_highlight_token: None,
        smart_highlight_truncated: false,
        lexer_override: None,
    };
    if doc_tab.doc.path.is_none() {
        update_next_untitled_index_from_name(state, &doc_tab.doc.display_name);
    }
    apply_syntax_for_doc(&doc_tab, state.editor_dark);
    restore_strike_ranges(editor, &entry.strike_ranges);
    let index = add_tab(state, &tab_title(&doc_tab), doc_tab)?;
    if entry.cursor_pos >= 0 {
        scintilla::goto_pos(state.docs[index].editor, entry.cursor_pos as usize);
    }
    Ok(())
}

fn capture_window_placement(hwnd: HWND) -> Option<session::WindowPlacementData> {
    let mut wp = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe { GetWindowPlacement(hwnd, &mut wp).ok()? };
    let rc = wp.rcNormalPosition;
    // Never persist a minimized state: rcNormalPosition always holds the
    // restored rect, and WPF_RESTORETOMAXIMIZED tells us whether a minimized
    // window would restore to maximized.
    let maximized = wp.showCmd == SW_SHOWMAXIMIZED.0 as u32
        || (wp.showCmd == SW_SHOWMINIMIZED.0 as u32 && wp.flags.contains(WPF_RESTORETOMAXIMIZED));
    session::WindowPlacementData {
        x: rc.left,
        y: rc.top,
        width: rc.right - rc.left,
        height: rc.bottom - rc.top,
        maximized,
    }
    .sanitized()
}

fn save_session_checkpoint(hwnd: HWND, state: &AppState) -> Result<()> {
    if !state.remember_session {
        let mut data = session::SessionData::empty();
        data.remember_session = false;
        data.session_snapshot_periodic_backup = false;
        data.backup_interval_seconds = state.backup_interval_seconds.max(1);
        data.word_wrap_enabled = state.word_wrap_enabled;
        data.always_on_top = state.always_on_top;
        data.window_placement = capture_window_placement(hwnd);
        data.active_tab_id = None;
        data.entries.clear();
        return session::save_session(&data);
    }

    let mut entries = Vec::new();
    for doc_tab in &state.docs {
        let backup_path = if doc_tab.doc.backup_path.as_os_str().is_empty() {
            session::backup_path_for_id(doc_tab.doc.id)?
        } else {
            doc_tab.doc.backup_path.clone()
        };
        entries.push(session::SessionEntry {
            id: doc_tab.doc.id,
            path: doc_tab.doc.path.clone(),
            display_name: tab_base_name(doc_tab),
            backup_path,
            is_dirty: doc_tab.doc.is_dirty,
            cursor_pos: scintilla::get_current_pos(doc_tab.editor) as i64,
            backup_timestamp: doc_tab.doc.last_backup_write.map(session::unix_timestamp),
            disk_timestamp_at_backup: doc_tab
                .doc
                .stamp
                .as_ref()
                .map(|stamp| session::unix_timestamp(stamp.modified)),
            strike_ranges: collect_strike_ranges(doc_tab.editor),
        });
    }

    let mut data = session::SessionData::empty();
    data.remember_session = state.remember_session;
    data.session_snapshot_periodic_backup = state.session_snapshot_periodic_backup;
    data.backup_interval_seconds = state.backup_interval_seconds.max(1);
    data.word_wrap_enabled = state.word_wrap_enabled;
    data.always_on_top = state.always_on_top;
    data.window_placement = capture_window_placement(hwnd);
    data.active_tab_id = state.docs.get(state.active).map(|doc| doc.doc.id);
    data.entries = entries;
    session::save_session(&data)
}

fn run_snapshot_tick(hwnd: HWND, state: &mut AppState, final_pass: bool) -> Result<()> {
    if state.session_snapshot_periodic_backup {
        backup_dirty_documents(state, final_pass)?;
    }
    if state.remember_session {
        save_session_checkpoint(hwnd, state)
    } else {
        Ok(())
    }
}

fn backup_dirty_documents(state: &mut AppState, force: bool) -> Result<()> {
    for index in 0..state.docs.len() {
        if !state.docs[index].doc.is_dirty {
            continue;
        }
        backup_doc_at_index(state, index, force)?;
    }
    Ok(())
}

fn backup_doc_at_index(state: &mut AppState, index: usize, force: bool) -> Result<()> {
    let doc_tab = state
        .docs
        .get_mut(index)
        .ok_or_else(|| AppError::new("Invalid document index for backup."))?;
    ensure_doc_backup_path(&mut doc_tab.doc)?;
    if !force
        && let Some(last) = doc_tab.last_backup_change_counter
        && last == doc_tab.change_counter
    {
        return Ok(());
    }

    doc_tab.doc.cursor_pos = scintilla::get_current_pos(doc_tab.editor) as i64;
    let text = scintilla::get_text(doc_tab.editor)?;
    let timestamp = session::write_backup(&doc_tab.doc.backup_path, text.as_bytes())?;
    if doc_tab.doc.first_backup_write.is_none() {
        doc_tab.doc.first_backup_write = Some(timestamp);
    }
    doc_tab.doc.last_backup_write = Some(timestamp);
    doc_tab.last_backup_change_counter = Some(doc_tab.change_counter);
    Ok(())
}

fn can_exit(hwnd: HWND, state: &mut AppState) -> Result<bool> {
    if state.session_snapshot_periodic_backup {
        run_snapshot_tick(hwnd, state, true)?;
        return Ok(true);
    }
    confirm_close_all(hwnd, state)
}

fn backup_interval_ms(interval_secs: u32) -> u32 {
    interval_secs.max(1).saturating_mul(1000)
}

fn next_untitled_name(state: &mut AppState) -> String {
    let value = state.next_untitled_index;
    state.next_untitled_index = state.next_untitled_index.saturating_add(1);
    format!("new {value:03}")
}

fn update_next_untitled_index_from_name(state: &mut AppState, name: &str) {
    if let Some(value) = name.strip_prefix("new ")
        && let Ok(parsed) = value.trim().parse::<usize>()
    {
        let candidate = parsed.saturating_add(1);
        if candidate > state.next_untitled_index {
            state.next_untitled_index = candidate;
        }
    }
}

fn ensure_doc_backup_path(doc: &mut Document) -> Result<()> {
    if doc.backup_path.as_os_str().is_empty() {
        doc.backup_path = session::backup_path_for_id(doc.id)?;
    }
    Ok(())
}

fn unix_millis_to_system_time(value: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(value)
}

fn eol_mode_label(mode: i32) -> &'static str {
    match mode {
        0 => "CRLF",
        1 => "CR",
        2 => "LF",
        _ => "?",
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

fn effective_wrap_enabled(
    wrap_enabled: bool,
    large_file_mode: bool,
    large_file_disable_word_wrap: bool,
) -> bool {
    wrap_enabled && !(large_file_mode && large_file_disable_word_wrap)
}

fn smart_highlight_allowed(
    smart_highlight_enabled: bool,
    large_file_mode: bool,
    large_file_disable_smart_highlight: bool,
) -> bool {
    smart_highlight_enabled && !(large_file_mode && large_file_disable_smart_highlight)
}

fn clamp_vertical_tab_width(state: &AppState, desired: i32, client_width: i32) -> i32 {
    let min_width = settings::MIN_VERTICAL_TAB_WIDTH_PX;
    let max_width = settings::MAX_VERTICAL_TAB_WIDTH_PX;
    let min_editor = scale_for_dpi(state.tab_host.vertical_tabs, 120);
    let max_by_client = (client_width - min_editor - TAB_SPLITTER_WIDTH).max(min_width);
    desired.clamp(min_width, max_width).min(max_by_client)
}

fn color_ref(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

fn tab_theme(dark: bool) -> TabTheme {
    if dark {
        TabTheme {
            bg: color_ref(30, 30, 30),
            fg: color_ref(212, 212, 212),
            selection_bg: color_ref(61, 122, 214),
            selection_fg: color_ref(255, 255, 255),
            hover_bg: color_ref(52, 52, 52),
            border: color_ref(45, 45, 45),
        }
    } else {
        TabTheme {
            bg: color_ref(255, 255, 255),
            fg: color_ref(32, 32, 32),
            selection_bg: color_ref(204, 228, 247),
            selection_fg: color_ref(0, 0, 0),
            hover_bg: color_ref(240, 247, 253),
            border: color_ref(215, 215, 215),
        }
    }
}

/// Retrieves the dark theme for a dialog by walking back to the main window's
/// `AppState` via the `GWLP_USERDATA` slot the dialog stored at creation.
/// Returns `None` if dark mode is off or the main window is gone.
fn dialog_dark_theme(dlg_hwnd: HWND) -> Option<TabTheme> {
    let main = unsafe { GetWindowLongPtrW(dlg_hwnd, GWLP_USERDATA) };
    if main == 0 {
        return None;
    }
    let state = get_state(HWND(main))?;
    if state.editor_dark {
        Some(state.tab_host.theme)
    } else {
        None
    }
}

/// Handles `WM_CTLCOLOR*` for a dialog child control. Returns the HBRUSH the
/// system should use for the control's background, or `None` when dark mode
/// is off (caller should fall through to `DefWindowProc`).
fn dialog_ctl_color(dlg_hwnd: HWND, hdc: HDC, edit_like: bool) -> Option<LRESULT> {
    let theme = dialog_dark_theme(dlg_hwnd)?;
    let bg = if edit_like { theme.hover_bg } else { theme.bg };
    unsafe {
        SetTextColor(hdc, theme.fg);
        SetBkMode(hdc, TRANSPARENT);
    }
    let brush = dark_mode::cached_solid_brush(bg);
    Some(LRESULT(brush.0))
}

/// Called from dialog `WM_CREATE` once all child controls exist. Applies the
/// title-bar dark attribute and re-themes child controls so scrollbars and
/// borders pick up the dark variant.
fn apply_dialog_dark_mode(dlg_hwnd: HWND) {
    let dark = dialog_dark_theme(dlg_hwnd).is_some();
    dark_mode::apply_to_window(dlg_hwnd, dark);
    dark_mode::theme_child_controls(dlg_hwnd, dark);
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

fn keyboard_tab_context_menu_target(state: &AppState, source: HWND) -> Option<(usize, i32, i32)> {
    if source != state.tab_host.top_tabs && source != state.tab_host.vertical_tabs {
        return None;
    }
    if state.docs.is_empty() || state.active >= state.docs.len() {
        return None;
    }

    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        point.x = 0;
        point.y = 0;
    }
    Some((state.active, point.x, point.y))
}

fn top_tab_hit_test_at_cursor(tabs: HWND) -> Option<(usize, i32, i32)> {
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

fn vertical_tab_hit_test_at_cursor(vertical_tabs: HWND) -> Option<(usize, i32, i32)> {
    let mut screen = POINT::default();
    if unsafe { GetCursorPos(&mut screen) }.is_err() {
        return None;
    }
    let mut client = screen;
    unsafe {
        let _ = ScreenToClient(vertical_tabs, &mut client);
    }
    let mut hit = LVHITTESTINFO {
        pt: client,
        ..Default::default()
    };
    let index = unsafe {
        SendMessageW(
            vertical_tabs,
            LVM_HITTEST,
            WPARAM(0),
            LPARAM(&mut hit as *mut LVHITTESTINFO as isize),
        )
    }
    .0 as i32;
    if index < 0 {
        None
    } else {
        Some((index as usize, screen.x, screen.y))
    }
}

fn tab_hit_test_at_cursor(state: &AppState) -> Option<(usize, i32, i32)> {
    match state.tab_host.placement {
        TabPlacement::Top => top_tab_hit_test_at_cursor(state.tab_host.top_tabs),
        TabPlacement::Left | TabPlacement::Right => {
            vertical_tab_hit_test_at_cursor(state.tab_host.vertical_tabs)
        }
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
    let is_dirty = doc.doc.is_dirty;
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
            CMD_EDITOR_STRIKEOUT as usize,
            w!("Strikeout"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRIM_LEADING_TRAILING as usize,
            w!("Trim Leading + Trailing Whitespace"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_EDITOR_COLLAPSE_SELECTION as usize,
            w!("Collapse Selection"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_EDITOR_EXPAND_ALL as usize,
            w!("Expand All Collapsed"),
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
        let strike_flags = if has_selection {
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
        let _ = EnableMenuItem(menu, CMD_EDITOR_STRIKEOUT as u32, strike_flags);
        let collapse_flags = {
            let line_start = scintilla::line_from_position(editor, sel_start.max(0) as usize);
            let line_end = scintilla::line_from_position(editor, sel_end.max(0) as usize);
            if has_selection && line_end > line_start {
                MF_BYCOMMAND | MF_ENABLED
            } else {
                MF_BYCOMMAND | MF_GRAYED
            }
        };
        let _ = EnableMenuItem(menu, CMD_EDITOR_COLLAPSE_SELECTION as u32, collapse_flags);
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

/// Locate the (initially empty) "Recent Files" popup inside the File menu.
fn recent_files_submenu(hwnd: HWND) -> Option<HMENU> {
    let bar = unsafe { GetMenu(hwnd) };
    if bar.0 == 0 {
        return None;
    }
    // File is the first top-level popup; Recent Files sits at FILE_MENU_RECENT_POS.
    let file_menu = unsafe { GetSubMenu(bar, 0) };
    if file_menu.0 == 0 {
        return None;
    }
    let recent = unsafe { GetSubMenu(file_menu, FILE_MENU_RECENT_POS) };
    if recent.0 == 0 { None } else { Some(recent) }
}

/// Rewrite the Recent Files submenu from `state.ui_settings.recent_files`.
fn rebuild_recent_files_menu(hwnd: HWND, state: &AppState) {
    let Some(menu) = recent_files_submenu(hwnd) else {
        return;
    };
    unsafe {
        for _ in 0..GetMenuItemCount(menu) {
            let _ = DeleteMenu(menu, 0, MF_BYPOSITION);
        }
    }
    let recent = &state.ui_settings.recent_files;
    if recent.is_empty() {
        unsafe {
            let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w!("(empty)"));
        }
        return;
    }
    for (i, path) in recent.iter().take(MAX_RECENT_FILES).enumerate() {
        // 1-based accelerator digit (10th item is "0"); '&' in paths must be escaped.
        let label = format!("&{} {}", (i + 1) % 10, path.replace('&', "&&"));
        let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                IDM_RECENT_FILE_BASE as usize + i,
                PCWSTR(wide.as_ptr()),
            );
        }
    }
    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_RECENT_CLEAR as usize,
            w!("Clear Recent Files"),
        );
    }
}

/// Record a freshly opened path in the MRU list and refresh the menu.
fn note_recent_file(hwnd: HWND, state: &mut AppState, path: &Path) {
    state
        .ui_settings
        .push_recent(path.to_string_lossy().into_owned());
    persist_ui_settings(state);
    rebuild_recent_files_menu(hwnd, state);
}

fn clear_recent_files(hwnd: HWND, state: &mut AppState) {
    state.ui_settings.recent_files.clear();
    persist_ui_settings(state);
    rebuild_recent_files_menu(hwnd, state);
}

fn open_recent_file(hwnd: HWND, state: &mut AppState, slot: usize) -> Result<()> {
    let Some(path_str) = state.ui_settings.recent_files.get(slot).cloned() else {
        return Ok(());
    };
    let path = PathBuf::from(&path_str);
    if !path.exists() {
        // Drop the stale entry, refresh the menu, then report it.
        state.ui_settings.recent_files.retain(|p| p != &path_str);
        persist_ui_settings(state);
        rebuild_recent_files_menu(hwnd, state);
        return Err(AppError::new(format!("File no longer exists:\n{path_str}")));
    }
    // open_path_new_tab records the MRU entry and refreshes the menu on success.
    open_path_new_tab(hwnd, state, path, None, None, None, None)
}

fn persist_ui_settings(state: &AppState) {
    let mut settings = state.ui_settings.clone();
    settings.tab_placement = state.tab_host.placement;
    settings.vertical_tab_width_px = state.tab_host.vertical_width_px;
    settings.editor_dark = state.editor_dark;
    if let Err(err) = settings::save_settings(&settings) {
        logging::log_error(&format!("settings_save_failed err={err}"));
    }
}

const TAB_CLOSE_BTN_SIZE: i32 = 14;
const TAB_CLOSE_BTN_MARGIN: i32 = 6;

fn tab_close_rect(
    hwnd: HWND,
    tab_rect: windows::Win32::Foundation::RECT,
) -> windows::Win32::Foundation::RECT {
    let size = scale_for_dpi(hwnd, TAB_CLOSE_BTN_SIZE);
    let margin = scale_for_dpi(hwnd, TAB_CLOSE_BTN_MARGIN);
    let height = tab_rect.bottom - tab_rect.top;
    let top = tab_rect.top + (height - size).max(0) / 2;
    windows::Win32::Foundation::RECT {
        left: tab_rect.right - margin - size,
        top,
        right: tab_rect.right - margin,
        bottom: top + size,
    }
}

fn point_in_rect(rect: &windows::Win32::Foundation::RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

fn lparam_xy(lparam: LPARAM) -> (i32, i32) {
    let raw = lparam.0 as i32;
    let x = (raw & 0xFFFF) as i16 as i32;
    let y = ((raw >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}

fn tab_rect_for_top(top_tabs: HWND, index: usize) -> Option<windows::Win32::Foundation::RECT> {
    let mut rect = windows::Win32::Foundation::RECT::default();
    let ok = unsafe {
        SendMessageW(
            top_tabs,
            TCM_GETITEMRECT,
            WPARAM(index),
            LPARAM(&mut rect as *mut _ as isize),
        )
        .0
    };
    if ok == 0 { None } else { Some(rect) }
}

fn tab_rect_for_vertical(
    vertical_tabs: HWND,
    index: usize,
) -> Option<windows::Win32::Foundation::RECT> {
    // LVM_GETITEMRECT: input `left` selects which rect to return; 0 == LVIR_BOUNDS.
    let mut rect = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe {
        SendMessageW(
            vertical_tabs,
            LVM_GETITEMRECT,
            WPARAM(index),
            LPARAM(&mut rect as *mut _ as isize),
        )
        .0
    };
    if ok == 0 { None } else { Some(rect) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TabHitArea {
    Body,
    Close,
}

fn hit_test_tab_at(state: &AppState, control: HWND, x: i32, y: i32) -> Option<(usize, TabHitArea)> {
    let count = state.docs.len();
    let is_top = control == state.tab_host.top_tabs;
    let is_vertical = control == state.tab_host.vertical_tabs;
    if !is_top && !is_vertical {
        return None;
    }
    for index in 0..count {
        let rect = if is_top {
            tab_rect_for_top(control, index)
        } else {
            tab_rect_for_vertical(control, index)
        };
        let Some(tab_rect) = rect else {
            continue;
        };
        if !point_in_rect(&tab_rect, x, y) {
            continue;
        }
        let close_rect = tab_close_rect(control, tab_rect);
        if point_in_rect(&close_rect, x, y) {
            return Some((index, TabHitArea::Close));
        }
        return Some((index, TabHitArea::Body));
    }
    None
}

fn draw_close_glyph(hdc: HDC, rect: windows::Win32::Foundation::RECT, fg: COLORREF, hot: bool) {
    let (pen_color, bg_fill) = if hot {
        (color_ref(255, 255, 255), Some(color_ref(232, 17, 35)))
    } else {
        (fg, None)
    };
    if let Some(bg) = bg_fill {
        unsafe {
            let brush = CreateSolidBrush(bg);
            if brush.0 != 0 {
                FillRect(hdc, &rect, brush);
                let _ = DeleteObject(brush);
            }
        }
    }
    let inset = 3;
    let left = rect.left + inset;
    let top = rect.top + inset;
    let right = rect.right - inset;
    let bottom = rect.bottom - inset;
    if right <= left || bottom <= top {
        return;
    }
    unsafe {
        let pen = CreatePen(PS_SOLID, 1, pen_color);
        if pen.0 == 0 {
            return;
        }
        let old = SelectObject(hdc, HGDIOBJ(pen.0));
        let _ = MoveToEx(hdc, left, top, None);
        let _ = LineTo(hdc, right, bottom);
        let _ = MoveToEx(hdc, left + 1, top, None);
        let _ = LineTo(hdc, right + 1, bottom);
        let _ = MoveToEx(hdc, right - 1, top, None);
        let _ = LineTo(hdc, left - 1, bottom);
        let _ = MoveToEx(hdc, right, top, None);
        let _ = LineTo(hdc, left, bottom);
        SelectObject(hdc, old);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

fn paint_top_tab_strip(hwnd: HWND, hdc: HDC, state: &AppState) {
    let mut client = windows::Win32::Foundation::RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
        let bg = CreateSolidBrush(state.tab_host.theme.bg);
        if bg.0 != 0 {
            let _ = FillRect(hdc, &client, bg);
            let _ = DeleteObject(bg);
        }
    }

    let selected = unsafe { SendMessageW(hwnd, TCM_GETCURSEL, WPARAM(0), LPARAM(0)).0 } as i32;
    let count = unsafe { SendMessageW(hwnd, TCM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0 } as usize;

    let hfont = unsafe { SendMessageW(hwnd, WM_GETFONT, WPARAM(0), LPARAM(0)).0 };
    let restore_font = if hfont != 0 {
        Some(unsafe { SelectObject(hdc, HGDIOBJ(hfont)) })
    } else {
        None
    };

    for index in 0..count {
        let mut rect = windows::Win32::Foundation::RECT::default();
        let got = unsafe {
            SendMessageW(
                hwnd,
                TCM_GETITEMRECT,
                WPARAM(index),
                LPARAM(&mut rect as *mut _ as isize),
            )
            .0
        };
        if got == 0 {
            continue;
        }
        let Some(doc_tab) = state.docs.get(index) else {
            continue;
        };
        let is_selected = index as i32 == selected;
        let hot =
            state.tab_host.hot_in_top && state.tab_host.hot_tab == Some(index) && !is_selected;
        let (fill, fg) = if is_selected {
            (
                state.tab_host.theme.selection_bg,
                state.tab_host.theme.selection_fg,
            )
        } else if hot {
            (state.tab_host.theme.hover_bg, state.tab_host.theme.fg)
        } else {
            (state.tab_host.theme.bg, state.tab_host.theme.fg)
        };
        unsafe {
            let brush = CreateSolidBrush(fill);
            if brush.0 != 0 {
                let _ = FillRect(hdc, &rect, brush);
                let _ = DeleteObject(brush);
            }
        }
        let close_rect = tab_close_rect(hwnd, rect);
        let pad = scale_for_dpi(hwnd, 8);
        let mut text_rect = windows::Win32::Foundation::RECT {
            left: rect.left + pad,
            top: rect.top,
            right: (close_rect.left - pad).max(rect.left + pad),
            bottom: rect.bottom,
        };
        let title = tab_title(doc_tab);
        let mut wide: Vec<u16> = title.encode_utf16().collect();
        unsafe {
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, fg);
            if !wide.is_empty() {
                DrawTextW(
                    hdc,
                    &mut wide,
                    &mut text_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
                );
            }
        }
        let close_hot = state.tab_host.hot_in_top
            && state.tab_host.hot_tab == Some(index)
            && state.tab_host.hot_close;
        draw_close_glyph(hdc, close_rect, fg, close_hot);
    }

    if let Some(prev) = restore_font {
        unsafe {
            SelectObject(hdc, prev);
        }
    }

    // Bottom seam to hide the tab control's default border.
    let bottom_h = scale_for_dpi(hwnd, 3);
    let seam = windows::Win32::Foundation::RECT {
        left: client.left,
        top: (client.bottom - bottom_h).max(client.top),
        right: client.right,
        bottom: client.bottom,
    };
    unsafe {
        let brush = CreateSolidBrush(state.tab_host.theme.bg);
        if brush.0 != 0 {
            let _ = FillRect(hdc, &seam, brush);
            let _ = DeleteObject(brush);
        }
    }
}

fn handle_vertical_tab_custom_draw(state: &AppState, lparam: LPARAM) -> LRESULT {
    let draw = unsafe { &mut *(lparam.0 as *mut NMLVCUSTOMDRAW) };
    if draw.nmcd.dwDrawStage == CDDS_PREPAINT {
        return LRESULT(CDRF_NOTIFYITEMDRAW as isize);
    }
    if draw.nmcd.dwDrawStage == CDDS_ITEMPREPAINT {
        let item_index = draw.nmcd.dwItemSpec;
        let doc_index = doc_index_by_runtime_id(state, draw.nmcd.lItemlParam).unwrap_or(item_index);
        let is_selected = doc_index == state.active;
        let is_hot = !is_selected && state.tab_host.hot_tab == Some(item_index);
        let (text_fg, fill_bg) = if is_selected {
            (
                state.tab_host.theme.selection_fg,
                state.tab_host.theme.selection_bg,
            )
        } else if is_hot {
            (state.tab_host.theme.fg, state.tab_host.theme.hover_bg)
        } else {
            (state.tab_host.theme.fg, state.tab_host.theme.bg)
        };
        unsafe {
            let brush = CreateSolidBrush(fill_bg);
            if brush.0 != 0 {
                FillRect(draw.nmcd.hdc, &draw.nmcd.rc, brush);
                let _ = DeleteObject(brush);
            }
        }
        draw.clrText = text_fg;
        draw.clrTextBk = fill_bg;
        return LRESULT((CDRF_NEWFONT | CDRF_NOTIFYPOSTPAINT) as isize);
    }
    if draw.nmcd.dwDrawStage == CDDS_ITEMPOSTPAINT {
        let item_index = draw.nmcd.dwItemSpec;
        let doc_index = doc_index_by_runtime_id(state, draw.nmcd.lItemlParam).unwrap_or(item_index);
        let is_selected = doc_index == state.active;
        let close_hot = state.tab_host.hot_tab == Some(item_index) && state.tab_host.hot_close;
        let text_fg = if is_selected {
            state.tab_host.theme.selection_fg
        } else {
            state.tab_host.theme.fg
        };
        let close_rect = tab_close_rect(state.tab_host.vertical_tabs, draw.nmcd.rc);
        draw_close_glyph(draw.nmcd.hdc, close_rect, text_fg, close_hot);
        return LRESULT(CDRF_DODEFAULT as isize);
    }
    LRESULT(CDRF_DODEFAULT as isize)
}

fn set_editor_dark_mode(hwnd: HWND, state: &mut AppState, enabled: bool) {
    state.editor_dark = enabled;
    update_editor_dark_menu(hwnd, enabled);
    for doc_tab in &state.docs {
        apply_syntax_for_doc(doc_tab, enabled);
    }
    if let Err(err) = update_tab_host_theme(state, enabled) {
        logging::log_error(&format!("tab_host_theme_update_failed err={err}"));
    }
    dark_mode::apply_to_window(hwnd, enabled);
    unsafe {
        InvalidateRect(state.tab_host.vertical_tabs, None, true);
        InvalidateRect(state.status, None, true);
        let _ = SetWindowPos(
            hwnd,
            HWND(0),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    persist_ui_settings(state);
}

fn set_tab_layout(hwnd: HWND, state: &mut AppState, layout: TabPlacement) {
    state.tab_host.placement = layout;
    update_tab_layout_menu(hwnd, layout);
    if layout == TabPlacement::Top && state.tab_host.resizing {
        state.tab_host.resizing = false;
        unsafe {
            let _ = ReleaseCapture();
        }
    }
    let (show_tabs, show_list) = match layout {
        TabPlacement::Top => (SW_SHOW, SW_HIDE),
        TabPlacement::Left | TabPlacement::Right => (SW_HIDE, SW_SHOW),
    };
    unsafe {
        ShowWindow(state.tab_host.top_tabs, show_tabs);
        ShowWindow(state.tab_host.vertical_tabs, show_list);
        let splitter_show = if layout == TabPlacement::Top {
            SW_HIDE
        } else {
            SW_SHOW
        };
        ShowWindow(state.tab_host.splitter, splitter_show);
    }
    set_vertical_tab_selection(state, state.active);
    layout_children(hwnd, state);
    persist_ui_settings(state);
}

fn toggle_word_wrap(hwnd: HWND, state: &mut AppState) {
    let enabled = !state.word_wrap_enabled;
    set_word_wrap(hwnd, state, enabled);
}

fn clamp_zoom(level: i32) -> i32 {
    level.clamp(settings::MIN_ZOOM_LEVEL, settings::MAX_ZOOM_LEVEL)
}

/// Applies one app-wide zoom level to every editor and persists it. Scintilla
/// only raises `SCN_ZOOM` when a control's level actually changes, and the
/// early return below stops the re-entrant notifications from the propagation
/// pass, so wheel-zooming one tab converges instead of looping.
fn set_app_zoom(state: &mut AppState, level: i32) {
    let level = clamp_zoom(level);
    if state.ui_settings.zoom_level == level {
        return;
    }
    state.ui_settings.zoom_level = level;
    for doc_tab in &state.docs {
        scintilla::set_zoom(doc_tab.editor, level);
    }
    persist_ui_settings(state);
}

fn adjust_zoom(state: &mut AppState, delta: i32) {
    set_app_zoom(state, state.ui_settings.zoom_level.saturating_add(delta));
}

fn set_word_wrap(hwnd: HWND, state: &mut AppState, enabled: bool) {
    state.word_wrap_enabled = enabled;
    for doc_tab in &mut state.docs {
        doc_tab.wrap_enabled = enabled;
        scintilla::set_wrap_enabled(
            doc_tab.editor,
            effective_wrap_enabled(
                enabled,
                doc_tab.doc.large_file_mode,
                state.ui_settings.large_file_disable_word_wrap,
            ),
        );
    }
    update_wrap_menu(hwnd, state);
    if let Err(err) = save_session_checkpoint(hwnd, state) {
        logging::log_error(&format!("session_save_after_wrap_toggle_failed err={err}"));
    }
}

fn apply_large_file_mode_restrictions(hwnd: HWND, state: &mut AppState, index: usize) {
    let mut should_refresh_smart_highlight = false;
    let Some(doc_tab) = state.docs.get_mut(index) else {
        return;
    };

    scintilla::set_wrap_enabled(
        doc_tab.editor,
        effective_wrap_enabled(
            doc_tab.wrap_enabled,
            doc_tab.doc.large_file_mode,
            state.ui_settings.large_file_disable_word_wrap,
        ),
    );

    if doc_tab.doc.large_file_mode && state.ui_settings.large_file_disable_smart_highlight {
        clear_smart_highlight(doc_tab);
    } else if index == state.active {
        should_refresh_smart_highlight = true;
    }

    if should_refresh_smart_highlight {
        update_smart_highlight_for_doc(state, index, true);
    }
    update_wrap_menu(hwnd, state);
}

fn set_always_on_top(hwnd: HWND, state: &mut AppState, enabled: bool) {
    state.always_on_top = enabled;
    update_always_on_top_menu(hwnd, state);
    apply_always_on_top(hwnd, enabled);
    if let Err(err) = save_session_checkpoint(hwnd, state) {
        logging::log_error(&format!(
            "session_save_after_always_on_top_toggle_failed err={err}"
        ));
    }
}

fn apply_always_on_top(hwnd: HWND, enabled: bool) {
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

fn update_tab_host_theme(state: &mut AppState, dark: bool) -> Result<()> {
    let theme = tab_theme(dark);
    let brush = unsafe { CreateSolidBrush(theme.bg) };
    if brush.0 == 0 {
        return Err(AppError::win32("CreateSolidBrush(VerticalTabs)"));
    }
    if state.tab_host.vertical_tabs_brush.0 != 0 {
        unsafe {
            let _ = DeleteObject(state.tab_host.vertical_tabs_brush);
        }
    }
    state.tab_host.vertical_tabs_brush = brush;
    state.tab_host.theme = theme;
    // Match the comctl32 selection rectangle / hot-track gradient to our
    // theme so clicking a vertical tab doesn't briefly flash the light
    // Explorer chrome.
    dark_mode::apply_explorer_theme(state.tab_host.vertical_tabs, dark);
    unsafe {
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_SETBKCOLOR,
            WPARAM(0),
            LPARAM(theme.bg.0 as isize),
        );
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_SETTEXTBKCOLOR,
            WPARAM(0),
            LPARAM(theme.bg.0 as isize),
        );
        SendMessageW(
            state.tab_host.vertical_tabs,
            LVM_SETTEXTCOLOR,
            WPARAM(0),
            LPARAM(theme.fg.0 as isize),
        );
        InvalidateRect(state.tab_host.vertical_tabs, None, true);
        InvalidateRect(state.tab_host.splitter, None, true);
        InvalidateRect(state.tab_host.top_tabs, None, true);
    }
    Ok(())
}

fn destroy_tab_host_brush(state: &mut AppState) {
    if state.tab_host.vertical_tabs_brush.0 != 0 {
        unsafe {
            let _ = DeleteObject(state.tab_host.vertical_tabs_brush);
        }
        state.tab_host.vertical_tabs_brush = HBRUSH(0);
    }
}

fn update_tab_layout_menu(hwnd: HWND, layout: TabPlacement) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let (horizontal, left, right) = match layout {
        TabPlacement::Top => (true, false, false),
        TabPlacement::Left => (false, true, false),
        TabPlacement::Right => (false, false, true),
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
    set_menu_check(menu, IDM_VIEW_WORD_WRAP, state.word_wrap_enabled);
}

fn update_file_menu(hwnd: HWND, state: &AppState) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let reload_state = if current_document_path(state).is_some() {
        MF_ENABLED
    } else {
        MF_GRAYED
    };
    unsafe {
        EnableMenuItem(menu, IDM_FILE_RELOAD as u32, MF_BYCOMMAND | reload_state);
    }
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
    set_menu_check(menu, CMD_VIEW_ALWAYS_ON_TOP, state.always_on_top);
}

/// Checks the Language-menu item matching the active tab's lexer: "Auto" when no
/// override is set, otherwise the forced language.
fn update_language_menu(hwnd: HWND, state: &AppState) {
    let menu = unsafe { GetMenu(hwnd) };
    if menu.0 == 0 {
        return;
    }
    let checked = match state.docs.get(state.active).and_then(|d| d.lexer_override) {
        None => CMD_LANG_AUTO,
        Some(kind) => command_for_lexer_kind(kind),
    };
    for id in CMD_LANG_AUTO..=CMD_LANG_PROPERTIES {
        set_menu_check(menu, id, id == checked);
    }
}

fn set_menu_check(menu: HMENU, id: u16, checked: bool) {
    let flag = if checked { MF_CHECKED } else { MF_UNCHECKED };
    let flags = (MF_BYCOMMAND | flag).0;
    unsafe {
        CheckMenuItem(menu, id as u32, flags);
    }
}

fn finish_splitter_resize(state: &mut AppState) {
    if !state.tab_host.resizing {
        return;
    }
    state.tab_host.resizing = false;
    unsafe {
        InvalidateRect(state.tab_host.splitter, None, true);
    }
    persist_ui_settings(state);
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
                && state.tab_host.is_vertical()
            {
                let mut cursor = POINT::default();
                if unsafe { GetCursorPos(&mut cursor) }.is_ok() {
                    state.tab_host.drag_start_x_screen = cursor.x;
                }
                state.tab_host.drag_start_width = state.tab_host.vertical_width_px;
                state.tab_host.resizing = true;
                unsafe {
                    let _ = SetCapture(hwnd);
                    InvalidateRect(hwnd, None, true);
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let parent = unsafe { GetParent(hwnd) };
            if parent.0 != 0
                && let Some(state) = get_state(parent)
                && state.tab_host.resizing
            {
                let mut cursor = POINT::default();
                if unsafe { GetCursorPos(&mut cursor) }.is_ok() {
                    let delta = cursor.x - state.tab_host.drag_start_x_screen;
                    let desired = match state.tab_host.placement {
                        TabPlacement::Left => state.tab_host.drag_start_width + delta,
                        TabPlacement::Right => state.tab_host.drag_start_width - delta,
                        TabPlacement::Top => state.tab_host.vertical_width_px,
                    };
                    let mut rect = windows::Win32::Foundation::RECT::default();
                    unsafe {
                        let _ = GetClientRect(parent, &mut rect);
                    }
                    let client_width = rect.right - rect.left;
                    state.tab_host.vertical_width_px =
                        clamp_vertical_tab_width(state, desired, client_width);
                    layout_children(parent, state);
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_LBUTTONUP => {
            let _ = unsafe { ReleaseCapture() };
            let parent = unsafe { GetParent(hwnd) };
            if parent.0 != 0
                && let Some(state) = get_state(parent)
            {
                finish_splitter_resize(state);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            let parent = unsafe { GetParent(hwnd) };
            if parent.0 != 0
                && let Some(state) = get_state(parent)
            {
                finish_splitter_resize(state);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let parent = unsafe { GetParent(hwnd) };
            let color = if parent.0 != 0 {
                get_state(parent)
                    .map(|state| {
                        if state.tab_host.resizing {
                            state.tab_host.theme.selection_bg
                        } else {
                            state.tab_host.theme.border
                        }
                    })
                    .unwrap_or(color_ref(160, 160, 160))
            } else {
                color_ref(160, 160, 160)
            };
            let mut paint = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            let brush = unsafe { CreateSolidBrush(color) };
            if brush.0 != 0 {
                unsafe {
                    let _ = FillRect(hdc, &paint.rcPaint, brush);
                    let _ = DeleteObject(brush);
                }
            }
            unsafe {
                let _ = EndPaint(hwnd, &paint);
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
                    w!("Wrap around"),
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
                    replace_label,
                    replace_edit,
                    match_case,
                    whole_word,
                    regex,
                    wrap,
                    replace_btn,
                    replace_all,
                });
                let _ = apply_search_state_to_dialog(state);
                unsafe {
                    let _ = SetFocus(find_edit);
                }
            }
            apply_dialog_dark_mode(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), false) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), true) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
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

unsafe extern "system" fn goto_line_wndproc(
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

            let scale = |value: i32| scale_for_dpi(hwnd, value);
            let label = unsafe {
                CreateWindowExW(
                    Default::default(),
                    w!("Static"),
                    w!("Line number (1 - 1):"),
                    WS_CHILD | WS_VISIBLE,
                    scale(12),
                    scale(14),
                    scale(240),
                    scale(20),
                    hwnd,
                    HMENU(0),
                    instance,
                    None,
                )
            };
            let line_edit = unsafe {
                CreateWindowExW(
                    Default::default(),
                    w!("Edit"),
                    PCWSTR::null(),
                    window_style(
                        WS_CHILD.0
                            | WS_VISIBLE.0
                            | WS_BORDER.0
                            | ES_AUTOHSCROLL as u32
                            | ES_NUMBER as u32
                            | WS_TABSTOP.0,
                    ),
                    scale(12),
                    scale(38),
                    scale(248),
                    scale(22),
                    hwnd,
                    menu_id(IDC_GOTO_LINE),
                    instance,
                    None,
                )
            };
            let go_btn = unsafe {
                CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Go"),
                    window_style(
                        WS_CHILD.0 | WS_VISIBLE.0 | BS_DEFPUSHBUTTON as u32 | WS_TABSTOP.0,
                    ),
                    scale(94),
                    scale(74),
                    scale(78),
                    scale(24),
                    hwnd,
                    menu_id(IDC_GOTO_GO),
                    instance,
                    None,
                )
            };
            let cancel_btn = unsafe {
                CreateWindowExW(
                    Default::default(),
                    w!("Button"),
                    w!("Cancel"),
                    window_style(WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32 | WS_TABSTOP.0),
                    scale(182),
                    scale(74),
                    scale(78),
                    scale(24),
                    hwnd,
                    menu_id(IDC_GOTO_CANCEL),
                    instance,
                    None,
                )
            };

            let _ = go_btn;
            let _ = cancel_btn;

            if let Some(state) = get_state(main_hwnd) {
                state.go_to_line_dialog = Some(GoToLineDialogState {
                    hwnd,
                    range_label: label,
                    line_edit,
                });
                let _ = apply_go_to_line_state(state);
                unsafe {
                    let _ = SetFocus(line_edit);
                }
            }
            apply_dialog_dark_mode(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), false) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), true) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_COMMAND => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                match loword(wparam.0) as usize {
                    IDC_GOTO_GO => {
                        if let Err(err) = confirm_go_to_line(main_hwnd, state) {
                            show_error("Rivet error", &err.to_string());
                        }
                    }
                    IDC_GOTO_CANCEL => {
                        close_go_to_line_dialog(main_hwnd, state);
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd) {
                close_go_to_line_dialog(main_hwnd, state);
            } else {
                unsafe { DestroyWindow(hwnd).ok() };
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let main_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };
            if let Some(state) = get_state(main_hwnd)
                && state
                    .go_to_line_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.hwnd == hwnd)
            {
                state.go_to_line_dialog = None;
                unsafe {
                    EnableWindow(main_hwnd, BOOL(1));
                }
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

            apply_dialog_dark_mode(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), false) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if let Some(r) = dialog_ctl_color(hwnd, HDC(wparam.0 as isize), true) {
                return r;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
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

    fn make_search_state() -> SearchState {
        SearchState {
            find_text: "needle".to_string(),
            replace_text: "swap".to_string(),
            match_case: false,
            whole_word: false,
            regex: false,
            wrap: true,
            last_direction: SearchDirection::Down,
        }
    }

    #[test]
    fn tab_layout_cycles() {
        assert_eq!(TabPlacement::Top.next(), TabPlacement::Left);
        assert_eq!(TabPlacement::Left.next(), TabPlacement::Right);
        assert_eq!(TabPlacement::Right.next(), TabPlacement::Top);
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
    fn format_thousands_groups_digits() {
        assert_eq!(format_thousands(0), "0");
        assert_eq!(format_thousands(999), "999");
        assert_eq!(format_thousands(1000), "1,000");
        assert_eq!(format_thousands(1234), "1,234");
        assert_eq!(format_thousands(1234567), "1,234,567");
    }

    #[test]
    fn clamp_zoom_bounds() {
        assert_eq!(clamp_zoom(-11), settings::MIN_ZOOM_LEVEL);
        assert_eq!(clamp_zoom(21), settings::MAX_ZOOM_LEVEL);
        assert_eq!(clamp_zoom(0), 0);
        assert_eq!(clamp_zoom(5), 5);
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

    #[test]
    fn language_command_override_roundtrip() {
        use scintilla::LexerKind as K;
        // "Auto" clears the per-tab override.
        assert_eq!(lexer_override_for_command(CMD_LANG_AUTO), Some(None));
        // Every lexer kind round-trips through its menu command.
        for kind in [
            K::Null,
            K::Cpp,
            K::JavaScript,
            K::Json,
            K::Yaml,
            K::PowerShell,
            K::Python,
            K::Html,
            K::Xml,
            K::Css,
            K::Properties,
            K::Markdown,
        ] {
            let cmd = command_for_lexer_kind(kind);
            assert_eq!(lexer_override_for_command(cmd), Some(Some(kind)));
        }
        // Non-language command ids are rejected.
        assert_eq!(lexer_override_for_command(IDM_FILE_NEW), None);
    }

    #[test]
    fn large_file_size_uses_threshold_mb() {
        assert!(document::is_large_file_size(1, 1_048_576));
        assert!(!document::is_large_file_size(1, 1_048_575));
        assert!(document::is_large_file_size(20, 20 * 1_048_576));
    }

    #[test]
    fn effective_wrap_respects_large_file_gating() {
        assert!(effective_wrap_enabled(true, false, true));
        assert!(effective_wrap_enabled(true, true, false));
        assert!(!effective_wrap_enabled(true, true, true));
        assert!(!effective_wrap_enabled(false, false, false));
    }

    #[test]
    fn smart_highlight_respects_large_file_gating() {
        assert!(smart_highlight_allowed(true, false, true));
        assert!(smart_highlight_allowed(true, true, false));
        assert!(!smart_highlight_allowed(true, true, true));
        assert!(!smart_highlight_allowed(false, false, false));
    }

    #[test]
    fn word_like_token_rules() {
        assert!(is_word_like_token("alpha"));
        assert!(is_word_like_token("alpha_12"));
        assert!(!is_word_like_token("alpha-beta"));
        assert!(!is_word_like_token("alpha beta"));
        assert!(!is_word_like_token(""));
    }

    #[test]
    fn about_details_include_requested_fields() {
        let details = AboutDetails {
            version: "0.4.3",
            git_sha: "0123456789abcdef",
            build_utc: "2026-03-22T12:34:56Z",
            source_url: "https://github.com/mgelsinger/rivetnotes",
            data_dir: PathBuf::from("C:\\Users\\test\\AppData\\Local\\Rivet"),
        };

        let rendered = format_about_details(&details);

        assert!(rendered.contains("Rivet 0.4.3"));
        assert!(rendered.contains("Commit: 0123456789ab"));
        assert!(rendered.contains("Build UTC: 2026-03-22T12:34:56Z"));
        assert!(rendered.contains("Source link: https://github.com/mgelsinger/rivetnotes"));
        assert!(rendered.contains("Data dir: C:\\Users\\test\\AppData\\Local\\Rivet"));
    }

    #[test]
    fn search_flags_include_requested_options() {
        let mut state = make_search_state();
        state.match_case = true;
        state.whole_word = true;
        assert_eq!(search_flags(&state), SCFIND_MATCHCASE | SCFIND_WHOLEWORD);

        state.regex = true;
        assert_eq!(
            search_flags(&state),
            SCFIND_MATCHCASE | SCFIND_WHOLEWORD | SCFIND_REGEXP
        );
    }

    #[test]
    fn build_search_plan_uses_selection_end_for_find_next_and_wrap() {
        let plan = build_search_plan(SearchDirection::Down, true, 120, 12, 20, 28);
        assert_eq!(
            plan,
            SearchPlan {
                primary: SearchRange {
                    start: 28,
                    end: 120
                },
                wrapped: Some(SearchRange { start: 0, end: 28 }),
            }
        );
    }

    #[test]
    fn build_search_plan_uses_selection_start_for_find_prev_and_wrap() {
        let plan = build_search_plan(SearchDirection::Up, true, 120, 60, 20, 28);
        assert_eq!(
            plan,
            SearchPlan {
                primary: SearchRange { start: 20, end: 0 },
                wrapped: Some(SearchRange {
                    start: 120,
                    end: 20
                }),
            }
        );
    }

    #[test]
    fn advance_replace_all_progress_updates_document_length() {
        let progress = advance_replace_all_progress(20, 5, 9, 2);
        assert_eq!(
            progress,
            ReplaceAllProgress {
                next_search_start: 7,
                next_doc_len: 18,
            }
        );

        let progress = advance_replace_all_progress(20, 5, 7, 6);
        assert_eq!(
            progress,
            ReplaceAllProgress {
                next_search_start: 11,
                next_doc_len: 24,
            }
        );
    }

    #[test]
    fn clamp_line_number_defaults_and_bounds() {
        assert_eq!(clamp_line_number(None, 50), 1);
        assert_eq!(clamp_line_number(Some(0), 50), 1);
        assert_eq!(clamp_line_number(Some(20), 50), 20);
        assert_eq!(clamp_line_number(Some(99), 50), 50);
    }
}
