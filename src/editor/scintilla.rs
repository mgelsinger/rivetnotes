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
const SCI_SETMARGINLEFT: u32 = 2155;
const SCI_SETMARGINRIGHT: u32 = 2157;
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
const SCI_SETEOLMODE: u32 = 2031;
const SCI_GETEOLMODE: u32 = 2030;
const SCI_GETCODEPAGE: u32 = 2137;
const SCI_SETWRAPMODE: u32 = 2268;
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
const SCI_SETSELFORE: u32 = 2067;
const SCI_SETSELBACK: u32 = 2068;
const SCI_SETCARETFORE: u32 = 2069;
const SCI_SETCARETLINEVISIBLE: u32 = 2096;
const SCI_SETCARETLINEBACK: u32 = 2098;
const SCI_SETCARETLINEBACKALPHA: u32 = 2470;
const SCI_SETPROPERTY: u32 = 4004;
const SCI_SETKEYWORDS: u32 = 4005;
const SCI_SETILEXER: u32 = 4033;

const SC_CP_UTF8: usize = 65001;
const SC_EOL_CRLF: usize = 0;
const SC_EOL_LF: usize = 2;
const SC_WRAP_NONE: usize = 0;
const SC_WRAP_WORD: usize = 1;
const SC_MARGIN_SYMBOL: usize = 0;
const SC_MOD_INSERTTEXT: usize = 0x1;
const SC_MOD_DELETETEXT: usize = 0x2;
const SC_POPUP_NEVER: usize = 0;
const SCMOD_SHIFT: usize = 0x1;
const SCMOD_CTRL: usize = 0x2;
const KEY_U: usize = b'U' as usize;

const STYLE_DEFAULT: usize = 32;

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
    send_message(hwnd, SCI_SETMARGINS, 0, 0);
    for margin in 0..5 {
        send_message(hwnd, SCI_SETMARGINTYPEN, margin, SC_MARGIN_SYMBOL as isize);
        send_message(hwnd, SCI_SETMARGINWIDTHN, margin, 0);
    }
    send_message(hwnd, SCI_SETMARGINLEFT, 0, 0);
    send_message(hwnd, SCI_SETMARGINRIGHT, 0, 0);
    send_message(hwnd, SCI_USEPOPUP, SC_POPUP_NEVER, 0);
    assign_default_command_keys(hwnd);
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
    apply_lexer_properties(hwnd, lexer);
    apply_lexer_styles(hwnd, lexer);
    apply_lexer_keywords(hwnd, lexer);
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

pub fn get_codepage(hwnd: HWND) -> i32 {
    send_message(hwnd, SCI_GETCODEPAGE, 0, 0).0 as i32
}

pub fn set_wrap_enabled(hwnd: HWND, enabled: bool) {
    let mode = if enabled { SC_WRAP_WORD } else { SC_WRAP_NONE };
    send_message(hwnd, SCI_SETWRAPMODE, mode, 0);
}

fn lexer_name(lexer: LexerKind) -> &'static str {
    match lexer {
        LexerKind::Null => "null",
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

fn set_property(hwnd: HWND, key: &str, value: &str) {
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
