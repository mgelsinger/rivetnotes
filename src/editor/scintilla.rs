// Scintilla is linked as a static library built from vendored source and
// hosted as a child Win32 window.

use std::ffi::{CString, c_char, c_void};

use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, HMENU, SendMessageW, WINDOW_EX_STYLE, WS_CHILD, WS_CLIPCHILDREN,
    WS_CLIPSIBLINGS, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use crate::app::document::Eol;
use crate::error::{AppError, Result};

const SCI_SETCODEPAGE: u32 = 2037;
const SCI_SETTEXT: u32 = 2181;
const SCI_GETTEXT: u32 = 2182;
const SCI_INSERTTEXT: u32 = 2003;
const SCI_GETLENGTH: u32 = 2006;
const SCI_GETCHARAT: u32 = 2007;
const SCI_GETCURRENTPOS: u32 = 2008;
const SCI_GETCOLUMN: u32 = 2129;
const SCI_GETLINEENDPOSITION: u32 = 2136;
const SCI_GETSELECTIONSTART: u32 = 2143;
const SCI_GETSELECTIONEND: u32 = 2145;
const SCI_GETSELECTIONEMPTY: u32 = 2650;
const SCI_GETLINECOUNT: u32 = 2154;
const SCI_LINEFROMPOSITION: u32 = 2166;
const SCI_POSITIONFROMLINE: u32 = 2167;
const SCI_SETMODEVENTMASK: u32 = 2359;
const SCI_ASSIGNCMDKEY: u32 = 2070;
const SCI_SETMARGINS: u32 = 2252;
const SCI_SETMARGINTYPEN: u32 = 2240;
const SCI_SETMARGINWIDTHN: u32 = 2242;
const SCI_SETMARGINMASKN: u32 = 2244;
const SCI_SETMARGINSENSITIVEN: u32 = 2246;
const SCI_SETMARGINLEFT: u32 = 2155;
const SCI_SETMARGINRIGHT: u32 = 2157;
const SCI_TEXTWIDTH: u32 = 2276;
const SCI_MARKERDEFINE: u32 = 2040;
const SCI_MARKERSETFORE: u32 = 2041;
const SCI_MARKERSETBACK: u32 = 2042;
const SCI_MARKERADD: u32 = 2043;
const SCI_MARKERDELETE: u32 = 2044;
const SCI_MARKERDELETEALL: u32 = 2045;
const SCI_MARKERGET: u32 = 2046;
const SCI_GETLINEVISIBLE: u32 = 2228;
const SCI_SETFOLDMARGINCOLOUR: u32 = 2290;
const SCI_SETFOLDMARGINHICOLOUR: u32 = 2291;
const SCI_TOGGLEFOLD: u32 = 2231;
const SCI_GETLASTCHILD: u32 = 2224;
const SCI_SETFOLDLEVEL: u32 = 2222;
const SCI_GETFOLDLEVEL: u32 = 2223;
const SCI_SETDEFAULTFOLDDISPLAYTEXT: u32 = 2722;
const SCI_TOGGLEFOLDSHOWTEXT: u32 = 2701;
const SCI_FOLDDISPLAYTEXTSETSTYLE: u32 = 2702;
const SCI_SETAUTOMATICFOLD: u32 = 2663;
const SCI_GOTOLINE: u32 = 2024;
const SCI_GOTOPOS: u32 = 2025;
const SCI_SETSAVEPOINT: u32 = 2014;
const SCI_CANREDO: u32 = 2016;
const SCI_UNDO: u32 = 2176;
const SCI_REDO: u32 = 2011;
const SCI_CUT: u32 = 2177;
const SCI_COPY: u32 = 2178;
const SCI_PASTE: u32 = 2179;
const SCI_CLEAR: u32 = 2180;
const SCI_CANPASTE: u32 = 2173;
const SCI_CANUNDO: u32 = 2174;
const SCI_SELECTALL: u32 = 2013;
const SCI_LINEDUPLICATE: u32 = 2404;
const SCI_LINEDELETE: u32 = 2338;
const SCI_LINEUP: u32 = 2302;
const SCI_LINEDOWN: u32 = 2300;
const SCI_TAB: u32 = 2327;
const SCI_BACKTAB: u32 = 2328;
const SCI_BEGINUNDOACTION: u32 = 2078;
const SCI_ENDUNDOACTION: u32 = 2079;
const SCI_SETSEL: u32 = 2160;
const SCI_SETTARGETRANGE: u32 = 2686;
const SCI_REPLACETARGET: u32 = 2194;
const SCI_SETSEARCHFLAGS: u32 = 2198;
const SCI_LOWERCASE: u32 = 2340;
const SCI_UPPERCASE: u32 = 2341;
const SCI_USEPOPUP: u32 = 2371;
const SCI_SETZOOM: u32 = 2373;
const SCI_GETZOOM: u32 = 2374;
const SCI_SETEOLMODE: u32 = 2031;
const SCI_GETEOLMODE: u32 = 2030;
const SCI_SETWRAPMODE: u32 = 2268;
const SCI_SETPRINTCOLOURMODE: u32 = 2148;
const SCI_SETPRINTWRAPMODE: u32 = 2406;
const SCI_FORMATRANGE: u32 = 2151;
const SCI_GETSELTEXT: u32 = 2161;
const SCI_GETTARGETSTART: u32 = 2191;
const SCI_GETTARGETEND: u32 = 2193;
const SCI_SEARCHINTARGET: u32 = 2197;
const SCI_STYLECLEARALL: u32 = 2050;
const SCI_STYLESETFORE: u32 = 2051;
const SCI_STYLESETBACK: u32 = 2052;
const SCI_STYLESETBOLD: u32 = 2053;
const SCI_STYLESETITALIC: u32 = 2054;
const SCI_STYLESETSIZE: u32 = 2055;
const SCI_STYLESETFONT: u32 = 2056;
const SCI_STYLESETUNDERLINE: u32 = 2059;
const SCI_SETSELFORE: u32 = 2067;
const SCI_SETSELBACK: u32 = 2068;
const SCI_SETCARETFORE: u32 = 2069;
const SCI_SETCARETLINEVISIBLE: u32 = 2096;
const SCI_SETCARETLINEBACK: u32 = 2098;
const SCI_SETCARETLINEBACKALPHA: u32 = 2470;
const SCI_SETPROPERTY: u32 = 4004;
const SCI_SETKEYWORDS: u32 = 4005;
const SCI_SETILEXER: u32 = 4033;
const SCI_INDICSETSTYLE: u32 = 2080;
const SCI_INDICSETFORE: u32 = 2082;
const SCI_SHOWLINES: u32 = 2226;
const SCI_HIDELINES: u32 = 2227;
const SCI_SETELEMENTCOLOUR: u32 = 2753;
const SCI_SETINDICATORCURRENT: u32 = 2500;
const SCI_SETINDICATORVALUE: u32 = 2502;
const SCI_INDICATORFILLRANGE: u32 = 2504;
const SCI_INDICATORCLEARRANGE: u32 = 2505;
const SCI_INDICATORALLONFOR: u32 = 2506;
const SCI_INDICATORVALUEAT: u32 = 2507;
const SCI_INDICATORSTART: u32 = 2508;
const SCI_INDICATOREND: u32 = 2509;
const SCI_INDICSETUNDER: u32 = 2510;
const SCI_INDICSETALPHA: u32 = 2523;
const SCI_INDICSETOUTLINEALPHA: u32 = 2558;

const SC_CP_UTF8: usize = 65001;
const SC_EOL_CRLF: usize = 0;
const SC_EOL_CR: usize = 1;
const SC_EOL_LF: usize = 2;
const SC_WRAP_NONE: usize = 0;
const SC_WRAP_WORD: usize = 1;
const SC_PRINT_BLACKONWHITE: usize = 2;
const SC_MARGIN_SYMBOL: usize = 0;
const SC_MOD_INSERTTEXT: usize = 0x1;
const SC_MOD_DELETETEXT: usize = 0x2;
const SC_POPUP_NEVER: usize = 0;
const SCMOD_SHIFT: usize = 0x1;
const SCMOD_CTRL: usize = 0x2;
const KEY_U: usize = b'U' as usize;
const INDIC_STRIKE: usize = 4;
const INDIC_ROUNDBOX: usize = 7;
const SC_ELEMENT_HIDDEN_LINE: usize = 81;

const SC_MARGIN_NUMBER: usize = 1;
const SC_MASK_FOLDERS: isize = 0xFE00_0000_u32 as i32 as isize;
const SC_MARK_PLUS: usize = 2;
const SC_MARK_BOXPLUS: usize = 12;
const SC_MARK_BOXPLUSCONNECTED: usize = 13;
const SC_MARK_BOXMINUS: usize = 14;
const SC_MARK_BOXMINUSCONNECTED: usize = 15;
const SC_MARK_VLINE: usize = 9;
const SC_MARK_LCORNER: usize = 10;
const SC_MARK_TCORNER: usize = 11;
const SC_MARKNUM_FOLDEREND: usize = 25;
const SC_MARKNUM_FOLDEROPENMID: usize = 26;
const SC_MARKNUM_FOLDERMIDTAIL: usize = 27;
const SC_MARKNUM_FOLDERTAIL: usize = 28;
const SC_MARKNUM_FOLDERSUB: usize = 29;
const SC_MARKNUM_FOLDER: usize = 30;
const SC_MARKNUM_FOLDEROPEN: usize = 31;
const SC_FOLDDISPLAYTEXT_BOXED: usize = 2;
const SC_AUTOMATICFOLD_SHOW: usize = 0x0001;
const SC_AUTOMATICFOLD_CHANGE: usize = 0x0004;

const STYLE_DEFAULT: usize = 32;
const STYLE_LINENUMBER: usize = 33;

const SCE_C_COMMENT: usize = 1;
const SCE_C_COMMENTLINE: usize = 2;
const SCE_C_COMMENTDOC: usize = 3;
const SCE_C_NUMBER: usize = 4;
const SCE_C_WORD: usize = 5;
const SCE_C_STRING: usize = 6;
const SCE_C_CHARACTER: usize = 7;
const SCE_C_PREPROCESSOR: usize = 9;
const SCE_C_OPERATOR: usize = 10;
const SCE_C_COMMENTLINEDOC: usize = 15;
const SCE_C_WORD2: usize = 16;

const SCE_P_COMMENTLINE: usize = 1;
const SCE_P_NUMBER: usize = 2;
const SCE_P_STRING: usize = 3;
const SCE_P_CHARACTER: usize = 4;
const SCE_P_WORD: usize = 5;
const SCE_P_TRIPLE: usize = 6;
const SCE_P_TRIPLEDOUBLE: usize = 7;
const SCE_P_CLASSNAME: usize = 8;
const SCE_P_DEFNAME: usize = 9;
const SCE_P_OPERATOR: usize = 10;
const SCE_P_DECORATOR: usize = 15;
const SCE_P_FSTRING: usize = 16;

const SCE_JSON_NUMBER: usize = 1;
const SCE_JSON_STRING: usize = 2;
const SCE_JSON_PROPERTYNAME: usize = 4;
const SCE_JSON_ESCAPESEQUENCE: usize = 5;
const SCE_JSON_LINECOMMENT: usize = 6;
const SCE_JSON_BLOCKCOMMENT: usize = 7;
const SCE_JSON_KEYWORD: usize = 11;

const SCE_YAML_COMMENT: usize = 1;
const SCE_YAML_IDENTIFIER: usize = 2;
const SCE_YAML_KEYWORD: usize = 3;
const SCE_YAML_NUMBER: usize = 4;
const SCE_YAML_OPERATOR: usize = 9;

const SCE_POWERSHELL_COMMENT: usize = 1;
const SCE_POWERSHELL_STRING: usize = 2;
const SCE_POWERSHELL_CHARACTER: usize = 3;
const SCE_POWERSHELL_NUMBER: usize = 4;
const SCE_POWERSHELL_VARIABLE: usize = 5;
const SCE_POWERSHELL_OPERATOR: usize = 6;
const SCE_POWERSHELL_KEYWORD: usize = 8;
const SCE_POWERSHELL_CMDLET: usize = 9;
const SCE_POWERSHELL_FUNCTION: usize = 11;
const SCE_POWERSHELL_COMMENTSTREAM: usize = 13;
const SCE_POWERSHELL_HERE_STRING: usize = 14;

const SCE_H_TAG: usize = 1;
const SCE_H_ATTRIBUTE: usize = 3;
const SCE_H_NUMBER: usize = 5;
const SCE_H_DOUBLESTRING: usize = 6;
const SCE_H_SINGLESTRING: usize = 7;
const SCE_H_COMMENT: usize = 9;
const SCE_H_ENTITY: usize = 10;
const SCE_H_VALUE: usize = 19;

const SCE_CSS_TAG: usize = 1;
const SCE_CSS_CLASS: usize = 2;
const SCE_CSS_PSEUDOCLASS: usize = 3;
const SCE_CSS_OPERATOR: usize = 5;
const SCE_CSS_VALUE: usize = 8;
const SCE_CSS_COMMENT: usize = 9;
const SCE_CSS_ID: usize = 10;
const SCE_CSS_IMPORTANT: usize = 11;
const SCE_CSS_DIRECTIVE: usize = 12;
const SCE_CSS_DOUBLESTRING: usize = 13;
const SCE_CSS_SINGLESTRING: usize = 14;
const SCE_CSS_ATTRIBUTE: usize = 16;

const SCE_PROPS_COMMENT: usize = 1;
const SCE_PROPS_SECTION: usize = 2;
const SCE_PROPS_ASSIGNMENT: usize = 3;
const SCE_PROPS_KEY: usize = 5;

const SCE_MARKDOWN_STRONG1: usize = 2;
const SCE_MARKDOWN_STRONG2: usize = 3;
const SCE_MARKDOWN_EM1: usize = 4;
const SCE_MARKDOWN_EM2: usize = 5;
const SCE_MARKDOWN_HEADER1: usize = 6;
const SCE_MARKDOWN_HEADER2: usize = 7;
const SCE_MARKDOWN_HEADER3: usize = 8;
const SCE_MARKDOWN_HEADER4: usize = 9;
const SCE_MARKDOWN_HEADER5: usize = 10;
const SCE_MARKDOWN_HEADER6: usize = 11;
const SCE_MARKDOWN_ULIST_ITEM: usize = 13;
const SCE_MARKDOWN_OLIST_ITEM: usize = 14;
const SCE_MARKDOWN_BLOCKQUOTE: usize = 15;
const SCE_MARKDOWN_STRIKEOUT: usize = 16;
const SCE_MARKDOWN_LINK: usize = 18;
const SCE_MARKDOWN_CODE: usize = 19;
const SCE_MARKDOWN_CODE2: usize = 20;
const SCE_MARKDOWN_CODEBK: usize = 21;

const fn color(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

const COLOR_DEFAULT_FORE: u32 = color(32, 32, 32);
const COLOR_DEFAULT_BACK: u32 = color(255, 255, 255);
const COLOR_DARK_FORE: u32 = color(212, 212, 212);
const COLOR_DARK_BACK: u32 = color(30, 30, 30);
const COLOR_SELECTION_LIGHT: u32 = color(205, 232, 255);
const COLOR_SELECTION_DARK: u32 = color(38, 79, 120);
const COLOR_CARET_LIGHT: u32 = color(32, 32, 32);
const COLOR_CARET_DARK: u32 = color(220, 220, 220);
const COLOR_CARET_LINE_LIGHT: u32 = color(240, 240, 240);
const COLOR_CARET_LINE_DARK: u32 = color(42, 42, 42);
const COLOR_COMMENT: u32 = color(0, 128, 0);
const COLOR_STRING: u32 = color(163, 21, 21);
const COLOR_NUMBER: u32 = color(9, 134, 88);
const COLOR_KEYWORD: u32 = color(0, 0, 255);
const COLOR_KEYWORD_ALT: u32 = color(43, 145, 175);
const COLOR_OPERATOR: u32 = color(0, 0, 0);
const COLOR_PREPROCESSOR: u32 = color(128, 64, 0);
const COLOR_TAG: u32 = color(0, 0, 160);
const COLOR_ATTRIBUTE: u32 = color(153, 0, 0);
const COLOR_VALUE: u32 = color(4, 81, 165);

const KEYWORDS_CPP: &str = concat!(
    "alignas alignof and and_eq asm auto bitand bitor bool break case catch char char8_t ",
    "char16_t char32_t class compl concept const consteval constexpr constinit const_cast ",
    "continue co_await co_return co_yield decltype default delete do double dynamic_cast ",
    "else enum explicit export extern false float for friend goto if inline int long ",
    "mutable namespace new noexcept not not_eq nullptr operator or or_eq private protected ",
    "public register reinterpret_cast requires return short signed sizeof static static_assert ",
    "static_cast struct switch template this thread_local throw true try typedef typeid ",
    "typename union unsigned using virtual void volatile wchar_t while xor xor_eq"
);
const KEYWORDS_JAVASCRIPT: &str = concat!(
    "async await break case catch class const continue debugger default delete do else export ",
    "extends finally for function if import in instanceof let new return super switch this ",
    "throw try typeof var void while with yield enum interface type implements public private ",
    "protected readonly namespace abstract as assert is keyof module require global of"
);
const KEYWORDS_PYTHON: &str = concat!(
    "False None True and as assert async await break class continue def del elif else except ",
    "finally for from global if import in is lambda nonlocal not or pass raise return try ",
    "while with yield"
);
const KEYWORDS_POWERSHELL: &str = concat!(
    "begin break catch class continue data do dynamicparam else elseif end enum exit filter ",
    "finally for foreach from function if in param process return switch throw trap try ",
    "until using var while"
);
const KEYWORDS_JSON: &str = "true false null";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LexerKind {
    Null,
    Cpp,
    JavaScript,
    Json,
    Yaml,
    PowerShell,
    Python,
    Html,
    Xml,
    Css,
    Properties,
    Markdown,
}

#[repr(C)]
struct SciNotifyHeader {
    hwnd_from: *mut c_void,
    id_from: usize,
    code: u32,
}

#[repr(C)]
pub struct SciNotification {
    nmhdr: SciNotifyHeader,
    pub position: isize,
    pub ch: i32,
    pub modifiers: i32,
    pub modification_type: i32,
    pub text: *const c_char,
    pub length: isize,
    pub lines_added: isize,
    pub message: i32,
    pub w_param: usize,
    pub l_param: isize,
    pub line: isize,
    pub fold_level_now: i32,
    pub fold_level_prev: i32,
    pub margin: i32,
    pub list_type: i32,
    pub x: i32,
    pub y: i32,
    pub token: i32,
    pub annotation_lines_added: isize,
    pub updated: i32,
    pub list_completion_method: i32,
    pub character_source: i32,
}

unsafe extern "C" {
    fn Scintilla_RegisterClasses(hinstance: *mut c_void) -> i32;
}

unsafe extern "system" {
    fn CreateLexer(name: *const c_char) -> *mut c_void;
}

pub fn register_classes(instance: HINSTANCE) -> Result<()> {
    let result = unsafe { Scintilla_RegisterClasses(instance.0 as *mut c_void) };
    if result == 0 {
        return Err(AppError::win32("Scintilla_RegisterClasses"));
    }
    Ok(())
}

pub fn initialize(hwnd: HWND) {
    send_message(hwnd, SCI_SETCODEPAGE, SC_CP_UTF8, 0);
    send_message(
        hwnd,
        SCI_SETMODEVENTMASK,
        SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT,
        0,
    );
    send_message(hwnd, SCI_SETMARGINS, 5, 0);
    // Margin 0: line numbers (width updated dynamically).
    send_message(hwnd, SCI_SETMARGINTYPEN, 0, SC_MARGIN_NUMBER as isize);
    send_message(hwnd, SCI_SETMARGINMASKN, 0, 0);
    send_message(hwnd, SCI_SETMARGINWIDTHN, 0, 0);
    send_message(hwnd, SCI_SETMARGINSENSITIVEN, 0, 0);
    // Margin 1: fold markers + user-collapse marker, click-sensitive.
    send_message(hwnd, SCI_SETMARGINTYPEN, 1, SC_MARGIN_SYMBOL as isize);
    let fold_mask: isize = SC_MASK_FOLDERS | (1 << USER_COLLAPSE_MARKER);
    send_message(hwnd, SCI_SETMARGINMASKN, 1, fold_mask);
    send_message(hwnd, SCI_SETMARGINWIDTHN, 1, 0);
    send_message(hwnd, SCI_SETMARGINSENSITIVEN, 1, 1);
    for margin in 2..5 {
        send_message(hwnd, SCI_SETMARGINTYPEN, margin, SC_MARGIN_SYMBOL as isize);
        send_message(hwnd, SCI_SETMARGINWIDTHN, margin, 0);
    }
    send_message(hwnd, SCI_SETMARGINLEFT, 0, 0);
    send_message(hwnd, SCI_SETMARGINRIGHT, 0, 0);
    send_message(hwnd, SCI_USEPOPUP, SC_POPUP_NEVER, 0);
    configure_fold_markers(hwnd);
    configure_user_collapse_marker(hwnd);
    send_message(
        hwnd,
        SCI_SETAUTOMATICFOLD,
        SC_AUTOMATICFOLD_SHOW | SC_AUTOMATICFOLD_CHANGE,
        0,
    );
    send_message(
        hwnd,
        SCI_FOLDDISPLAYTEXTSETSTYLE,
        SC_FOLDDISPLAYTEXT_BOXED,
        0,
    );
    set_default_fold_display_text(hwnd, " \u{22EF} ");
    assign_default_command_keys(hwnd);
}

fn configure_fold_markers(hwnd: HWND) {
    let pairs: [(usize, usize); 7] = [
        (SC_MARKNUM_FOLDER, SC_MARK_BOXPLUS),
        (SC_MARKNUM_FOLDEROPEN, SC_MARK_BOXMINUS),
        (SC_MARKNUM_FOLDERSUB, SC_MARK_VLINE),
        (SC_MARKNUM_FOLDERTAIL, SC_MARK_LCORNER),
        (SC_MARKNUM_FOLDEREND, SC_MARK_BOXPLUSCONNECTED),
        (SC_MARKNUM_FOLDEROPENMID, SC_MARK_BOXMINUSCONNECTED),
        (SC_MARKNUM_FOLDERMIDTAIL, SC_MARK_TCORNER),
    ];
    for (marker, shape) in pairs {
        send_message(hwnd, SCI_MARKERDEFINE, marker, shape as isize);
    }
}

pub const USER_COLLAPSE_MARKER: usize = 5;

fn configure_user_collapse_marker(hwnd: HWND) {
    send_message(
        hwnd,
        SCI_MARKERDEFINE,
        USER_COLLAPSE_MARKER,
        SC_MARK_PLUS as isize,
    );
    // Accent color so the user-collapse glyph is distinguishable from lexer fold boxes.
    send_message(hwnd, SCI_MARKERSETFORE, USER_COLLAPSE_MARKER, 0x00C07020);
    send_message(hwnd, SCI_MARKERSETBACK, USER_COLLAPSE_MARKER, 0x00FFFFFF);
}

pub fn marker_add(hwnd: HWND, line: usize, marker_num: usize) {
    send_message(hwnd, SCI_MARKERADD, line, marker_num as isize);
}

pub fn marker_delete(hwnd: HWND, line: usize, marker_num: usize) {
    send_message(hwnd, SCI_MARKERDELETE, line, marker_num as isize);
}

pub fn marker_delete_all(hwnd: HWND, marker_num: i32) {
    send_message(hwnd, SCI_MARKERDELETEALL, marker_num as usize, 0);
}

pub fn marker_get(hwnd: HWND, line: usize) -> u32 {
    send_message(hwnd, SCI_MARKERGET, line, 0).0 as u32
}

pub fn line_visible(hwnd: HWND, line: usize) -> bool {
    send_message(hwnd, SCI_GETLINEVISIBLE, line, 0).0 != 0
}

pub fn set_fold_marker_colors(hwnd: HWND, fore_rgb: u32, back_rgb: u32) {
    for marker in [
        SC_MARKNUM_FOLDER,
        SC_MARKNUM_FOLDEROPEN,
        SC_MARKNUM_FOLDERSUB,
        SC_MARKNUM_FOLDERTAIL,
        SC_MARKNUM_FOLDEREND,
        SC_MARKNUM_FOLDEROPENMID,
        SC_MARKNUM_FOLDERMIDTAIL,
    ] {
        send_message(hwnd, SCI_MARKERSETFORE, marker, back_rgb as isize);
        send_message(hwnd, SCI_MARKERSETBACK, marker, fore_rgb as isize);
    }
    send_message(hwnd, SCI_SETFOLDMARGINCOLOUR, 1, back_rgb as isize);
    send_message(hwnd, SCI_SETFOLDMARGINHICOLOUR, 1, back_rgb as isize);
}

pub fn set_line_number_style(hwnd: HWND, fore_rgb: u32, back_rgb: u32) {
    send_message(hwnd, SCI_STYLESETFORE, STYLE_LINENUMBER, fore_rgb as isize);
    send_message(hwnd, SCI_STYLESETBACK, STYLE_LINENUMBER, back_rgb as isize);
}

pub fn set_line_number_margin_width(hwnd: HWND, total_lines: usize) {
    let digits = digit_count(total_lines.max(1)).max(3);
    let sample: String = "9".repeat(digits);
    let cstr = match CString::new(sample) {
        Ok(c) => c,
        Err(_) => return,
    };
    let width = send_message(
        hwnd,
        SCI_TEXTWIDTH,
        STYLE_LINENUMBER,
        cstr.as_ptr() as isize,
    )
    .0;
    let padding = 8isize;
    let final_width = (width + padding).max(20);
    send_message(hwnd, SCI_SETMARGINWIDTHN, 0, final_width);
}

pub fn set_fold_margin_width(hwnd: HWND, width: i32) {
    send_message(hwnd, SCI_SETMARGINWIDTHN, 1, width as isize);
}

fn digit_count(n: usize) -> usize {
    let mut n = n;
    let mut digits = 1usize;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

pub fn toggle_fold(hwnd: HWND, line: usize) {
    send_message(hwnd, SCI_TOGGLEFOLD, line, 0);
}

pub fn fold_last_child(hwnd: HWND, line: usize, level: i32) -> usize {
    send_message(hwnd, SCI_GETLASTCHILD, line, level as isize).0 as usize
}

pub fn set_fold_level(hwnd: HWND, line: usize, level: u32) {
    send_message(hwnd, SCI_SETFOLDLEVEL, line, level as isize);
}

pub fn fold_level(hwnd: HWND, line: usize) -> u32 {
    send_message(hwnd, SCI_GETFOLDLEVEL, line, 0).0 as u32
}

pub fn set_default_fold_display_text(hwnd: HWND, text: &str) {
    let cstr = match CString::new(text) {
        Ok(c) => c,
        Err(_) => return,
    };
    send_message(
        hwnd,
        SCI_SETDEFAULTFOLDDISPLAYTEXT,
        0,
        cstr.as_ptr() as isize,
    );
}

pub fn toggle_fold_show_text(hwnd: HWND, line: usize, text: &str) {
    let cstr = match CString::new(text) {
        Ok(c) => c,
        Err(_) => return,
    };
    send_message(hwnd, SCI_TOGGLEFOLDSHOWTEXT, line, cstr.as_ptr() as isize);
}

fn assign_default_command_keys(hwnd: HWND) {
    let lower = command_key(KEY_U, SCMOD_CTRL);
    let upper = command_key(KEY_U, SCMOD_CTRL | SCMOD_SHIFT);
    send_message(hwnd, SCI_ASSIGNCMDKEY, lower, SCI_LOWERCASE as isize);
    send_message(hwnd, SCI_ASSIGNCMDKEY, upper, SCI_UPPERCASE as isize);
}

fn command_key(key_code: usize, key_mod: usize) -> usize {
    key_code | (key_mod << 16)
}

pub fn create_window(parent: HWND, instance: HINSTANCE) -> Result<HWND> {
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Scintilla"),
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            parent,
            HMENU(1),
            instance,
            None,
        )
    };

    if hwnd.0 == 0 {
        return Err(AppError::win32("CreateWindowExW(Scintilla)"));
    }

    Ok(hwnd)
}

pub fn apply_lexer(hwnd: HWND, lexer: LexerKind, dark: bool) {
    apply_base_theme(hwnd, dark);
    set_lexer_by_name(hwnd, lexer_name(lexer));
    clear_keywords(hwnd);
    apply_fold_properties(hwnd);
    apply_lexer_properties(hwnd, lexer);
    apply_lexer_styles(hwnd, lexer);
    apply_lexer_keywords(hwnd, lexer);
}

fn apply_fold_properties(hwnd: HWND) {
    set_property(hwnd, "fold", "1");
    set_property(hwnd, "fold.compact", "0");
    set_property(hwnd, "fold.html", "1");
    set_property(hwnd, "fold.preprocessor", "1");
    set_property(hwnd, "fold.comment", "1");
    set_property(hwnd, "fold.cpp.syntax.based", "1");
}

pub fn set_text(hwnd: HWND, text: &str) -> Result<()> {
    let mut buffer = Vec::with_capacity(text.len() + 1);
    buffer.extend_from_slice(text.as_bytes());
    buffer.push(0);
    send_message(hwnd, SCI_SETTEXT, 0, buffer.as_ptr() as isize);
    Ok(())
}

pub fn get_text(hwnd: HWND) -> Result<String> {
    let length = send_message(hwnd, SCI_GETLENGTH, 0, 0).0 as usize;
    let mut buffer = vec![0u8; length + 1];
    send_message(
        hwnd,
        SCI_GETTEXT,
        buffer.len(),
        buffer.as_mut_ptr() as isize,
    );
    buffer.truncate(length);
    String::from_utf8(buffer).map_err(|err| AppError::new(format!("Invalid UTF-8 text: {err}")))
}

pub fn get_current_pos(hwnd: HWND) -> usize {
    send_message(hwnd, SCI_GETCURRENTPOS, 0, 0).0 as usize
}

pub fn get_column(hwnd: HWND, pos: usize) -> usize {
    send_message(hwnd, SCI_GETCOLUMN, pos, 0).0 as usize
}

pub fn line_from_position(hwnd: HWND, pos: usize) -> usize {
    send_message(hwnd, SCI_LINEFROMPOSITION, pos, 0).0 as usize
}

pub fn goto_pos(hwnd: HWND, pos: usize) {
    send_message(hwnd, SCI_GOTOPOS, pos, 0);
}

/// Inserts `text` at byte position `pos` without moving the caret/selection
/// the way a paste would. Used for structural edits like adding a line below a
/// collapsed block.
pub fn insert_text(hwnd: HWND, pos: usize, text: &str) {
    let Ok(text) = CString::new(text) else {
        return;
    };
    send_message(hwnd, SCI_INSERTTEXT, pos, text.as_ptr() as isize);
}

/// The newline sequence matching the document's current EOL mode.
pub fn eol_string(hwnd: HWND) -> &'static str {
    match get_eol_mode(hwnd) {
        n if n == SC_EOL_CR as i32 => "\r",
        n if n == SC_EOL_LF as i32 => "\n",
        _ => "\r\n",
    }
}

pub fn goto_line(hwnd: HWND, line: usize) {
    send_message(hwnd, SCI_GOTOLINE, line, 0);
}

pub fn set_savepoint(hwnd: HWND) {
    send_message(hwnd, SCI_SETSAVEPOINT, 0, 0);
}

pub fn can_undo(hwnd: HWND) -> bool {
    send_message(hwnd, SCI_CANUNDO, 0, 0).0 != 0
}

pub fn can_redo(hwnd: HWND) -> bool {
    send_message(hwnd, SCI_CANREDO, 0, 0).0 != 0
}

pub fn undo(hwnd: HWND) {
    send_message(hwnd, SCI_UNDO, 0, 0);
}

pub fn redo(hwnd: HWND) {
    send_message(hwnd, SCI_REDO, 0, 0);
}

pub fn cut(hwnd: HWND) {
    send_message(hwnd, SCI_CUT, 0, 0);
}

pub fn copy(hwnd: HWND) {
    send_message(hwnd, SCI_COPY, 0, 0);
}

pub fn paste(hwnd: HWND) {
    send_message(hwnd, SCI_PASTE, 0, 0);
}

pub fn clear(hwnd: HWND) {
    send_message(hwnd, SCI_CLEAR, 0, 0);
}

pub fn can_paste(hwnd: HWND) -> bool {
    send_message(hwnd, SCI_CANPASTE, 0, 0).0 != 0
}

pub fn select_all(hwnd: HWND) {
    send_message(hwnd, SCI_SELECTALL, 0, 0);
}

pub fn duplicate_line(hwnd: HWND) {
    send_message(hwnd, SCI_LINEDUPLICATE, 0, 0);
}

pub fn delete_line(hwnd: HWND) {
    send_message(hwnd, SCI_LINEDELETE, 0, 0);
}

pub fn move_line_up(hwnd: HWND) {
    send_message(hwnd, SCI_LINEUP, 0, 0);
}

pub fn move_line_down(hwnd: HWND) {
    send_message(hwnd, SCI_LINEDOWN, 0, 0);
}

pub fn indent_selection(hwnd: HWND) {
    send_message(hwnd, SCI_TAB, 0, 0);
}

/// Zoom is points added to the base font size; Scintilla supports -10..=20.
pub fn set_zoom(hwnd: HWND, zoom: i32) {
    send_message(hwnd, SCI_SETZOOM, zoom as usize, 0);
}

pub fn get_zoom(hwnd: HWND) -> i32 {
    send_message(hwnd, SCI_GETZOOM, 0, 0).0 as i32
}

pub fn outdent_selection(hwnd: HWND) {
    send_message(hwnd, SCI_BACKTAB, 0, 0);
}

pub fn selection_start(hwnd: HWND) -> usize {
    send_message(hwnd, SCI_GETSELECTIONSTART, 0, 0).0 as usize
}

pub fn selection_end(hwnd: HWND) -> usize {
    send_message(hwnd, SCI_GETSELECTIONEND, 0, 0).0 as usize
}

pub fn selection_empty(hwnd: HWND) -> bool {
    send_message(hwnd, SCI_GETSELECTIONEMPTY, 0, 0).0 != 0
}

pub fn uppercase_selection(hwnd: HWND) {
    send_message(hwnd, SCI_UPPERCASE, 0, 0);
}

pub fn lowercase_selection(hwnd: HWND) {
    send_message(hwnd, SCI_LOWERCASE, 0, 0);
}

pub fn line_count(hwnd: HWND) -> usize {
    send_message(hwnd, SCI_GETLINECOUNT, 0, 0).0 as usize
}

pub fn position_from_line(hwnd: HWND, line: usize) -> usize {
    send_message(hwnd, SCI_POSITIONFROMLINE, line, 0).0 as usize
}

pub fn line_end_position(hwnd: HWND, line: usize) -> usize {
    send_message(hwnd, SCI_GETLINEENDPOSITION, line, 0).0 as usize
}

pub fn char_at(hwnd: HWND, pos: usize) -> u8 {
    send_message(hwnd, SCI_GETCHARAT, pos, 0).0 as u8
}

pub fn begin_undo_action(hwnd: HWND) {
    send_message(hwnd, SCI_BEGINUNDOACTION, 0, 0);
}

pub fn end_undo_action(hwnd: HWND) {
    send_message(hwnd, SCI_ENDUNDOACTION, 0, 0);
}

pub fn set_target_range(hwnd: HWND, start: usize, end: usize) {
    send_message(hwnd, SCI_SETTARGETRANGE, start, end as isize);
}

pub fn replace_target_empty(hwnd: HWND) {
    const EMPTY: [u8; 1] = [0];
    send_message(hwnd, SCI_REPLACETARGET, 0, EMPTY.as_ptr() as isize);
}

pub fn replace_target(hwnd: HWND, text: &str) {
    send_message(hwnd, SCI_REPLACETARGET, text.len(), text.as_ptr() as isize);
}

pub fn search_in_target(
    hwnd: HWND,
    text: &str,
    flags: usize,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    if text.is_empty() {
        return None;
    }
    send_message(hwnd, SCI_SETSEARCHFLAGS, flags, 0);
    set_target_range(hwnd, start, end);
    let pos = send_message(hwnd, SCI_SEARCHINTARGET, text.len(), text.as_ptr() as isize).0;
    if pos < 0 {
        return None;
    }
    let target_start = send_message(hwnd, SCI_GETTARGETSTART, 0, 0).0 as usize;
    let target_end = send_message(hwnd, SCI_GETTARGETEND, 0, 0).0 as usize;
    Some((target_start, target_end))
}

pub fn set_selection(hwnd: HWND, start: usize, end: usize) {
    let start_param = start.try_into().unwrap_or(isize::MAX) as usize;
    let end_param = end.try_into().unwrap_or(isize::MAX);
    send_message(hwnd, SCI_SETSEL, start_param, end_param);
}

pub fn selected_text(hwnd: HWND) -> Result<String> {
    let start = selection_start(hwnd);
    let end = selection_end(hwnd);
    let len = end.abs_diff(start);
    let mut buffer = vec![0u8; len + 1];
    send_message(hwnd, SCI_GETSELTEXT, 0, buffer.as_mut_ptr() as isize);
    buffer.truncate(len);
    String::from_utf8(buffer).map_err(|err| AppError::new(format!("Invalid UTF-8 text: {err}")))
}

pub fn get_length(hwnd: HWND) -> usize {
    send_message(hwnd, SCI_GETLENGTH, 0, 0).0 as usize
}

/// Layout of a device rectangle used by `SCI_FORMATRANGE`, matching
/// Scintilla's `Sci_Rectangle` (plain `int` fields regardless of large-file
/// position mode).
#[repr(C)]
struct SciRectangle {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct SciCharacterRange {
    cp_min: i32,
    cp_max: i32,
}

/// Matches Scintilla's `Sci_RangeToFormat`. `hdc`/`hdc_target` hold raw
/// `HDC` values (Scintilla declares them `void*`).
#[repr(C)]
struct SciRangeToFormat {
    hdc: isize,
    hdc_target: isize,
    rc: SciRectangle,
    rc_page: SciRectangle,
    chrg: SciCharacterRange,
}

/// A device-coordinate rectangle for print layout (page bounds or the
/// content area within it).
#[derive(Copy, Clone)]
pub struct PrintRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Forces print output to black text on white background, ignoring both
/// syntax-highlight colors and the live editor's dark/light theme — the
/// print job should look the same regardless of how the document is
/// currently themed on screen. Also forces word wrap for the print job
/// specifically, via Scintilla's print-only wrap mode, which is fully
/// independent of `SCI_SETWRAPMODE` and does not touch on-screen display.
pub fn prepare_for_print(hwnd: HWND) {
    send_message(hwnd, SCI_SETPRINTCOLOURMODE, SC_PRINT_BLACKONWHITE, 0);
    send_message(hwnd, SCI_SETPRINTWRAPMODE, SC_WRAP_WORD, 0);
}

/// Renders (or, with `draw = false`, only measures) the document range
/// `[range_start, range_end)` into `print_rect` on the page described by
/// `page_rect`, using `hdc` as both the render target and the device
/// context Scintilla measures text metrics against (the same HDC for
/// actual printing). Returns the position just past the last character
/// that fit, which becomes `range_start` for the next page.
pub fn format_range(
    hwnd: HWND,
    draw: bool,
    hdc: isize,
    print_rect: PrintRect,
    page_rect: PrintRect,
    range_start: usize,
    range_end: usize,
) -> usize {
    let mut fr = SciRangeToFormat {
        hdc,
        hdc_target: hdc,
        rc: SciRectangle {
            left: print_rect.left,
            top: print_rect.top,
            right: print_rect.right,
            bottom: print_rect.bottom,
        },
        rc_page: SciRectangle {
            left: page_rect.left,
            top: page_rect.top,
            right: page_rect.right,
            bottom: page_rect.bottom,
        },
        chrg: SciCharacterRange {
            cp_min: range_start as i32,
            cp_max: range_end as i32,
        },
    };
    let lparam = std::ptr::addr_of_mut!(fr) as isize;
    send_message(hwnd, SCI_FORMATRANGE, draw as usize, lparam).0 as usize
}

/// Must be called once after the print job finishes to let Scintilla free
/// resources cached across the `SCI_FORMATRANGE` calls.
pub fn end_format_range(hwnd: HWND) {
    send_message(hwnd, SCI_FORMATRANGE, 0, 0);
}

pub fn set_eol_mode(hwnd: HWND, eol: Eol) {
    let mode = match eol {
        Eol::Crlf => SC_EOL_CRLF,
        Eol::Lf => SC_EOL_LF,
    };
    send_message(hwnd, SCI_SETEOLMODE, mode, 0);
}

pub fn get_eol_mode(hwnd: HWND) -> i32 {
    send_message(hwnd, SCI_GETEOLMODE, 0, 0).0 as i32
}

pub fn set_wrap_enabled(hwnd: HWND, enabled: bool) {
    let mode = if enabled { SC_WRAP_WORD } else { SC_WRAP_NONE };
    send_message(hwnd, SCI_SETWRAPMODE, mode, 0);
}

pub fn configure_smart_highlight_indicator(
    hwnd: HWND,
    indicator: usize,
    fore_rgb: u32,
    fill_alpha: usize,
    outline_alpha: usize,
) {
    send_message(hwnd, SCI_INDICSETSTYLE, indicator, INDIC_ROUNDBOX as isize);
    send_message(hwnd, SCI_INDICSETFORE, indicator, fore_rgb as isize);
    send_message(hwnd, SCI_INDICSETALPHA, indicator, fill_alpha as isize);
    send_message(
        hwnd,
        SCI_INDICSETOUTLINEALPHA,
        indicator,
        outline_alpha as isize,
    );
}

pub fn configure_strike_indicator(hwnd: HWND, indicator: usize, fore_rgb: u32) {
    send_message(hwnd, SCI_INDICSETSTYLE, indicator, INDIC_STRIKE as isize);
    send_message(hwnd, SCI_INDICSETFORE, indicator, fore_rgb as isize);
    send_message(hwnd, SCI_INDICSETUNDER, indicator, 0);
}

pub fn set_indicator_current(hwnd: HWND, indicator: usize) {
    send_message(hwnd, SCI_SETINDICATORCURRENT, indicator, 0);
}

pub fn set_indicator_value(hwnd: HWND, value: i32) {
    send_message(hwnd, SCI_SETINDICATORVALUE, value as usize, 0);
}

pub fn fill_indicator_range(hwnd: HWND, start: usize, len: usize) {
    send_message(hwnd, SCI_INDICATORFILLRANGE, start, len as isize);
}

pub fn clear_indicator_range(hwnd: HWND, start: usize, len: usize) {
    send_message(hwnd, SCI_INDICATORCLEARRANGE, start, len as isize);
}

pub fn indicator_all_on_for(hwnd: HWND, pos: usize) -> u32 {
    send_message(hwnd, SCI_INDICATORALLONFOR, pos, 0).0 as u32
}

pub fn indicator_value_at(hwnd: HWND, indicator: usize, pos: usize) -> i32 {
    send_message(hwnd, SCI_INDICATORVALUEAT, indicator, pos as isize).0 as i32
}

pub fn indicator_start(hwnd: HWND, indicator: usize, pos: usize) -> usize {
    send_message(hwnd, SCI_INDICATORSTART, indicator, pos as isize).0 as usize
}

pub fn indicator_end(hwnd: HWND, indicator: usize, pos: usize) -> usize {
    send_message(hwnd, SCI_INDICATOREND, indicator, pos as isize).0 as usize
}

pub fn hide_lines(hwnd: HWND, line_start: usize, line_end: usize) {
    send_message(hwnd, SCI_HIDELINES, line_start, line_end as isize);
}

pub fn show_lines(hwnd: HWND, line_start: usize, line_end: usize) {
    send_message(hwnd, SCI_SHOWLINES, line_start, line_end as isize);
}

pub fn set_hidden_line_color(hwnd: HWND, rgb: u32) {
    let colour_alpha = rgb | 0xFF00_0000;
    send_message(
        hwnd,
        SCI_SETELEMENTCOLOUR,
        SC_ELEMENT_HIDDEN_LINE,
        colour_alpha as isize,
    );
}

fn lexer_name(lexer: LexerKind) -> &'static str {
    match lexer {
        LexerKind::Null => "null",
        LexerKind::Markdown => "markdown",
        LexerKind::Cpp | LexerKind::JavaScript => "cpp",
        LexerKind::Json => "json",
        LexerKind::Yaml => "yaml",
        LexerKind::PowerShell => "powershell",
        LexerKind::Python => "python",
        LexerKind::Html => "hypertext",
        LexerKind::Xml => "xml",
        LexerKind::Css => "css",
        LexerKind::Properties => "props",
    }
}

fn apply_base_theme(hwnd: HWND, dark: bool) {
    let (fore, back, sel_back, caret, caret_line) = if dark {
        (
            COLOR_DARK_FORE,
            COLOR_DARK_BACK,
            COLOR_SELECTION_DARK,
            COLOR_CARET_DARK,
            COLOR_CARET_LINE_DARK,
        )
    } else {
        (
            COLOR_DEFAULT_FORE,
            COLOR_DEFAULT_BACK,
            COLOR_SELECTION_LIGHT,
            COLOR_CARET_LIGHT,
            COLOR_CARET_LINE_LIGHT,
        )
    };
    set_style_fore(hwnd, STYLE_DEFAULT, fore);
    set_style_back(hwnd, STYLE_DEFAULT, back);
    set_style_size(hwnd, STYLE_DEFAULT, 11);
    set_style_font(hwnd, STYLE_DEFAULT, "Consolas");
    send_message(hwnd, SCI_STYLECLEARALL, 0, 0);
    send_message(hwnd, SCI_SETSELFORE, 1, fore as isize);
    send_message(hwnd, SCI_SETSELBACK, 1, sel_back as isize);
    send_message(hwnd, SCI_SETCARETFORE, caret as usize, 0);
    send_message(hwnd, SCI_SETCARETLINEVISIBLE, 0, 0);
    send_message(hwnd, SCI_SETCARETLINEBACK, 0, caret_line as isize);
    send_message(hwnd, SCI_SETCARETLINEBACKALPHA, 0, 0);
}

fn apply_lexer_properties(hwnd: HWND, lexer: LexerKind) {
    match lexer {
        LexerKind::Cpp => {
            set_property(hwnd, "lexer.cpp.allow.dollars", "0");
            set_property(hwnd, "lexer.cpp.track.preprocessor", "1");
        }
        LexerKind::JavaScript => {
            set_property(hwnd, "lexer.cpp.allow.dollars", "1");
            set_property(hwnd, "lexer.cpp.track.preprocessor", "0");
        }
        _ => {}
    }
}

fn apply_lexer_styles(hwnd: HWND, lexer: LexerKind) {
    match lexer {
        LexerKind::Null => {}
        LexerKind::Markdown => {
            for header in [
                SCE_MARKDOWN_HEADER1,
                SCE_MARKDOWN_HEADER2,
                SCE_MARKDOWN_HEADER3,
                SCE_MARKDOWN_HEADER4,
                SCE_MARKDOWN_HEADER5,
                SCE_MARKDOWN_HEADER6,
            ] {
                set_style(hwnd, header, COLOR_KEYWORD, true, false);
            }
            // Strong/emphasis keep the theme's default foreground.
            set_style_bold(hwnd, SCE_MARKDOWN_STRONG1, true);
            set_style_bold(hwnd, SCE_MARKDOWN_STRONG2, true);
            set_style_italic(hwnd, SCE_MARKDOWN_EM1, true);
            set_style_italic(hwnd, SCE_MARKDOWN_EM2, true);
            set_style(
                hwnd,
                SCE_MARKDOWN_ULIST_ITEM,
                COLOR_KEYWORD_ALT,
                true,
                false,
            );
            set_style(
                hwnd,
                SCE_MARKDOWN_OLIST_ITEM,
                COLOR_KEYWORD_ALT,
                true,
                false,
            );
            set_style(hwnd, SCE_MARKDOWN_BLOCKQUOTE, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_MARKDOWN_STRIKEOUT, COLOR_COMMENT, false, false);
            set_style(hwnd, SCE_MARKDOWN_LINK, COLOR_VALUE, false, false);
            set_style_underline(hwnd, SCE_MARKDOWN_LINK, true);
            set_style(hwnd, SCE_MARKDOWN_CODE, COLOR_STRING, false, false);
            set_style(hwnd, SCE_MARKDOWN_CODE2, COLOR_STRING, false, false);
            set_style(hwnd, SCE_MARKDOWN_CODEBK, COLOR_STRING, false, false);
        }
        LexerKind::Cpp | LexerKind::JavaScript => {
            set_style(hwnd, SCE_C_COMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_C_COMMENTLINE, COLOR_COMMENT, false, false);
            set_style(hwnd, SCE_C_COMMENTDOC, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_C_COMMENTLINEDOC, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_C_NUMBER, COLOR_NUMBER, false, false);
            set_style(hwnd, SCE_C_WORD, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_C_WORD2, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_C_STRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_C_CHARACTER, COLOR_STRING, false, false);
            set_style(hwnd, SCE_C_PREPROCESSOR, COLOR_PREPROCESSOR, false, false);
            set_style(hwnd, SCE_C_OPERATOR, COLOR_OPERATOR, false, false);
        }
        LexerKind::Python => {
            set_style(hwnd, SCE_P_COMMENTLINE, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_P_NUMBER, COLOR_NUMBER, false, false);
            set_style(hwnd, SCE_P_STRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_P_CHARACTER, COLOR_STRING, false, false);
            set_style(hwnd, SCE_P_TRIPLE, COLOR_STRING, false, false);
            set_style(hwnd, SCE_P_TRIPLEDOUBLE, COLOR_STRING, false, false);
            set_style(hwnd, SCE_P_WORD, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_P_CLASSNAME, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_P_DEFNAME, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_P_DECORATOR, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_P_FSTRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_P_OPERATOR, COLOR_OPERATOR, false, false);
        }
        LexerKind::PowerShell => {
            set_style(hwnd, SCE_POWERSHELL_COMMENT, COLOR_COMMENT, false, true);
            set_style(
                hwnd,
                SCE_POWERSHELL_COMMENTSTREAM,
                COLOR_COMMENT,
                false,
                true,
            );
            set_style(hwnd, SCE_POWERSHELL_STRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_POWERSHELL_HERE_STRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_POWERSHELL_CHARACTER, COLOR_STRING, false, false);
            set_style(hwnd, SCE_POWERSHELL_NUMBER, COLOR_NUMBER, false, false);
            set_style(
                hwnd,
                SCE_POWERSHELL_VARIABLE,
                COLOR_KEYWORD_ALT,
                false,
                false,
            );
            set_style(hwnd, SCE_POWERSHELL_KEYWORD, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_POWERSHELL_CMDLET, COLOR_KEYWORD_ALT, false, false);
            set_style(
                hwnd,
                SCE_POWERSHELL_FUNCTION,
                COLOR_KEYWORD_ALT,
                false,
                false,
            );
            set_style(hwnd, SCE_POWERSHELL_OPERATOR, COLOR_OPERATOR, false, false);
        }
        LexerKind::Json => {
            set_style(hwnd, SCE_JSON_NUMBER, COLOR_NUMBER, false, false);
            set_style(hwnd, SCE_JSON_STRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_JSON_PROPERTYNAME, COLOR_ATTRIBUTE, false, false);
            set_style(hwnd, SCE_JSON_ESCAPESEQUENCE, COLOR_VALUE, false, false);
            set_style(hwnd, SCE_JSON_LINECOMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_JSON_BLOCKCOMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_JSON_KEYWORD, COLOR_KEYWORD, true, false);
        }
        LexerKind::Yaml => {
            set_style(hwnd, SCE_YAML_COMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_YAML_IDENTIFIER, COLOR_DEFAULT_FORE, false, false);
            set_style(hwnd, SCE_YAML_KEYWORD, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_YAML_NUMBER, COLOR_NUMBER, false, false);
            set_style(hwnd, SCE_YAML_OPERATOR, COLOR_OPERATOR, false, false);
        }
        LexerKind::Html | LexerKind::Xml => {
            set_style(hwnd, SCE_H_TAG, COLOR_TAG, true, false);
            set_style(hwnd, SCE_H_ATTRIBUTE, COLOR_ATTRIBUTE, false, false);
            set_style(hwnd, SCE_H_NUMBER, COLOR_NUMBER, false, false);
            set_style(hwnd, SCE_H_DOUBLESTRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_H_SINGLESTRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_H_COMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_H_ENTITY, COLOR_VALUE, false, false);
            set_style(hwnd, SCE_H_VALUE, COLOR_STRING, false, false);
        }
        LexerKind::Css => {
            set_style(hwnd, SCE_CSS_TAG, COLOR_TAG, true, false);
            set_style(hwnd, SCE_CSS_CLASS, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_CSS_ID, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_CSS_PSEUDOCLASS, COLOR_KEYWORD_ALT, false, false);
            set_style(hwnd, SCE_CSS_ATTRIBUTE, COLOR_ATTRIBUTE, false, false);
            set_style(hwnd, SCE_CSS_VALUE, COLOR_STRING, false, false);
            set_style(hwnd, SCE_CSS_COMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_CSS_IMPORTANT, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_CSS_DIRECTIVE, COLOR_KEYWORD, true, false);
            set_style(hwnd, SCE_CSS_DOUBLESTRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_CSS_SINGLESTRING, COLOR_STRING, false, false);
            set_style(hwnd, SCE_CSS_OPERATOR, COLOR_OPERATOR, false, false);
        }
        LexerKind::Properties => {
            set_style(hwnd, SCE_PROPS_COMMENT, COLOR_COMMENT, false, true);
            set_style(hwnd, SCE_PROPS_SECTION, COLOR_TAG, true, false);
            set_style(hwnd, SCE_PROPS_ASSIGNMENT, COLOR_OPERATOR, false, false);
            set_style(hwnd, SCE_PROPS_KEY, COLOR_ATTRIBUTE, false, false);
        }
    }
}

fn apply_lexer_keywords(hwnd: HWND, lexer: LexerKind) {
    match lexer {
        LexerKind::Cpp => set_keywords(hwnd, 0, KEYWORDS_CPP),
        LexerKind::JavaScript => set_keywords(hwnd, 0, KEYWORDS_JAVASCRIPT),
        LexerKind::Python => set_keywords(hwnd, 0, KEYWORDS_PYTHON),
        LexerKind::PowerShell => set_keywords(hwnd, 0, KEYWORDS_POWERSHELL),
        LexerKind::Json => set_keywords(hwnd, 0, KEYWORDS_JSON),
        _ => {}
    }
}

fn clear_keywords(hwnd: HWND) {
    set_keywords(hwnd, 0, "");
    set_keywords(hwnd, 1, "");
    set_keywords(hwnd, 2, "");
    set_keywords(hwnd, 3, "");
}

fn set_lexer_by_name(hwnd: HWND, name: &str) {
    let Ok(name) = CString::new(name) else {
        return;
    };
    let lexer = unsafe { CreateLexer(name.as_ptr()) };
    let ptr = if lexer.is_null() { 0 } else { lexer as isize };
    send_message(hwnd, SCI_SETILEXER, 0, ptr);
}

fn set_keywords(hwnd: HWND, set: usize, words: &str) {
    let Ok(words) = CString::new(words) else {
        return;
    };
    send_message(hwnd, SCI_SETKEYWORDS, set, words.as_ptr() as isize);
}

pub fn set_property(hwnd: HWND, key: &str, value: &str) {
    let Ok(key) = CString::new(key) else {
        return;
    };
    let Ok(value) = CString::new(value) else {
        return;
    };
    send_message(
        hwnd,
        SCI_SETPROPERTY,
        key.as_ptr() as usize,
        value.as_ptr() as isize,
    );
}

fn set_style(hwnd: HWND, style: usize, fore: u32, bold: bool, italic: bool) {
    set_style_fore(hwnd, style, fore);
    set_style_bold(hwnd, style, bold);
    set_style_italic(hwnd, style, italic);
}

fn set_style_fore(hwnd: HWND, style: usize, fore: u32) {
    send_message(hwnd, SCI_STYLESETFORE, style, fore as isize);
}

fn set_style_back(hwnd: HWND, style: usize, back: u32) {
    send_message(hwnd, SCI_STYLESETBACK, style, back as isize);
}

fn set_style_bold(hwnd: HWND, style: usize, bold: bool) {
    send_message(hwnd, SCI_STYLESETBOLD, style, if bold { 1 } else { 0 });
}

fn set_style_italic(hwnd: HWND, style: usize, italic: bool) {
    send_message(hwnd, SCI_STYLESETITALIC, style, if italic { 1 } else { 0 });
}

fn set_style_underline(hwnd: HWND, style: usize, underline: bool) {
    send_message(
        hwnd,
        SCI_STYLESETUNDERLINE,
        style,
        if underline { 1 } else { 0 },
    );
}

fn set_style_size(hwnd: HWND, style: usize, size: usize) {
    send_message(hwnd, SCI_STYLESETSIZE, style, size as isize);
}

fn set_style_font(hwnd: HWND, style: usize, font: &str) {
    let Ok(font) = CString::new(font) else {
        return;
    };
    send_message(hwnd, SCI_STYLESETFONT, style, font.as_ptr() as isize);
}

fn send_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> LRESULT {
    unsafe { SendMessageW(hwnd, msg, WPARAM(wparam), LPARAM(lparam)) }
}
