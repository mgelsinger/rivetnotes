//! Regressions for the accepted editing and window-behavior pull requests.
#![allow(clippy::unwrap_used)]

use super::safety_tests::window;
use super::spellcheck_tests::{TestData, TestWindow};
use super::*;

#[test]
fn save_extensions_preserve_auto_and_follow_explicit_language() {
    use scintilla::LexerKind::*;
    for (path, language, expected) in [
        (None, None, ".txt"),
        (None, Some(Python), ".py"),
        (None, Some(Null), ".txt"),
        (Some(Path::new("config.yml")), None, ".yml"),
        (Some(Path::new("notes.MD")), None, ".MD"),
        (Some(Path::new("data.custom")), None, ".custom"),
        (Some(Path::new("notes.txt")), Some(Markdown), ".md"),
    ] {
        assert_eq!(default_save_extension(path, language), expected);
    }
}

#[test]
fn accelerator_table_with_f5_can_be_created_repeatedly() -> Result<()> {
    // Run this in both debug and release: PR #12 reported a failure at 35
    // entries in optimized builds. Verify the real table, not a simplified copy.
    for _ in 0..100 {
        let accelerator = create_accelerators()?;
        unsafe {
            assert!(DestroyAcceleratorTable(accelerator).as_bool());
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses hidden windows and temporary files"]
fn save_updates_recent_files_and_external_delete_keeps_contents() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let state = get_state(window.0).unwrap();
    let path = session::data_dir()?.join("newly-saved.txt");
    state.docs[0].doc.path = Some(path.clone());
    scintilla::set_text(state.docs[0].editor, "keep this note")?;
    assert!(state.ui_settings.recent_files.is_empty());
    assert!(save_document_at(window.0, state, 0, None, false)?);
    assert_eq!(state.ui_settings.recent_files, vec![path.to_string_lossy()]);
    assert_eq!(
        settings::load_settings()?.recent_files,
        state.ui_settings.recent_files
    );
    std::fs::remove_file(&path).unwrap();
    check_external_change(window.0, state)?;
    assert_eq!(scintilla::get_text(state.docs[0].editor)?, "keep this note");
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses a hidden window"]
fn small_window_and_dpi_changes_preserve_preferred_tab_width() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let hwnd = window.0;
    {
        let state = get_state(hwnd).unwrap();
        state.tab_host.vertical_width_px = 320;
        set_tab_layout(hwnd, state, TabPlacement::Left);
    }
    unsafe {
        SetWindowPos(hwnd, HWND(0), 0, 0, 200, 300, SWP_NOZORDER | SWP_NOACTIVATE)?;
    }
    assert_eq!(get_state(hwnd).unwrap().tab_host.vertical_width_px, 320);
    let suggested = RECT {
        left: 30,
        top: 40,
        right: 930,
        bottom: 640,
    };
    unsafe {
        SendMessageW(
            hwnd,
            WM_DPICHANGED,
            WPARAM(144 | (144 << 16)),
            LPARAM(&suggested as *const RECT as isize),
        );
    }
    let state = get_state(hwnd).unwrap();
    assert_eq!(state.tab_host.vertical_width_px, 320);
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)?;
    }
    assert_eq!(rect, suggested);
    persist_ui_settings(state);
    assert_eq!(settings::load_settings()?.vertical_tab_width_px, 320);
    Ok(())
}

fn margin_width(editor: HWND) -> isize {
    unsafe {
        SendMessageW(
            editor,
            2243, /* SCI_GETMARGINWIDTHN */
            WPARAM(0),
            LPARAM(0),
        )
        .0
    }
}

#[test]
#[ignore = "Requires Windows controls; tests settings and commands in a hidden window"]
fn font_line_numbers_datetime_and_spellcheck_coexist() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let hwnd = window.0;
    {
        let state = get_state(hwnd).unwrap();
        set_editor_font(state, "Courier New".to_string(), 16);
        set_line_numbers(hwnd, state, false);
        set_editor_dark_mode(hwnd, state, false);
        set_language_override(state, Some(scintilla::LexerKind::Markdown));
        create_empty_tab(hwnd, module_instance()?, state)?;
        duplicate_active_tab(hwnd, state)?;
        for tab in &state.docs {
            assert_eq!(margin_width(tab.editor), 0);
            let size = unsafe {
                SendMessageW(
                    tab.editor,
                    2485, /* SCI_STYLEGETSIZE */
                    WPARAM(32),
                    LPARAM(0),
                )
                .0
            };
            assert_eq!(size, 16);
        }
        assert!(!session::load_session()?.line_numbers_enabled);
        let preferences = settings::load_settings()?;
        assert_eq!(preferences.editor_font_name, "Courier New");
        assert_eq!(preferences.editor_font_size, 16);
        assert!(!state.ui_settings.spellcheck_enabled);
        assert_ne!(IDM_VIEW_FONT, IDM_VIEW_SPELLCHECK);
        assert_ne!(IDM_VIEW_LINE_NUMBERS, IDM_VIEW_SPELLCHECK);
    }
    unsafe {
        SendMessageW(
            hwnd,
            WM_COMMAND,
            WPARAM(IDM_VIEW_LINE_NUMBERS as usize),
            LPARAM(0),
        );
    }
    let editor = active_editor(get_state(hwnd).unwrap()).unwrap();
    assert!(margin_width(editor) > 0);
    scintilla::set_text(editor, "replace me")?;
    scintilla::set_selection(editor, 0, 10);
    unsafe {
        SendMessageW(
            hwnd,
            WM_COMMAND,
            WPARAM(IDM_EDIT_INSERT_DATETIME as usize),
            LPARAM(0),
        );
    }
    let inserted = scintilla::get_text(editor)?;
    assert!(
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$")
            .unwrap()
            .is_match(&inserted)
    );
    scintilla::undo(editor);
    assert_eq!(scintilla::get_text(editor)?, "replace me");
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; paints hidden dialogs in both themes"]
fn dialogs_retheme_and_find_controls_do_not_overlap() -> Result<()> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, GetPixel, ReleaseDC,
    };
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let mut dialogs = Vec::new();
    for (class, width, height) in [
        (FIND_CLASS, 520, 240),
        (GOTO_LINE_CLASS, 320, 180),
        (FIND_FILES_CLASS, 700, 500),
    ] {
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                class,
                w!("Test dialog"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                width,
                height,
                main.0,
                HMENU(0),
                module_instance()?,
                Some(main.0.0 as *const std::ffi::c_void),
            )
        };
        assert_ne!(hwnd.0, 0);
        dialogs.push(TestWindow(hwnd));
    }
    for dark in [true, false, true] {
        set_editor_dark_mode(main.0, get_state(main.0).unwrap(), dark);
        for dialog in &dialogs {
            let screen = unsafe { GetDC(HWND(0)) };
            let dc = unsafe { CreateCompatibleDC(screen) };
            let bitmap = unsafe { CreateCompatibleBitmap(screen, 8, 8) };
            let old = unsafe { SelectObject(dc, bitmap) };
            assert_ne!(dc.0, 0);
            let result = unsafe {
                SendMessageW(dialog.0, WM_ERASEBKGND, WPARAM(dc.0 as usize), LPARAM(0)).0
            };
            assert_eq!(result, 1);
            let expected = if dark {
                dialog_dark_theme(dialog.0).unwrap().bg
            } else {
                COLORREF(unsafe { windows::Win32::Graphics::Gdi::GetSysColor(COLOR_BTNFACE) })
            };
            assert_eq!(unsafe { GetPixel(dc, 1, 1) }, expected);
            unsafe {
                SelectObject(dc, old);
                let _ = DeleteObject(bitmap);
                let _ = DeleteDC(dc);
                ReleaseDC(HWND(0), screen);
            }
        }
    }
    let find = get_state(main.0).unwrap().find_dialog.as_ref().unwrap();
    let mut checkbox = RECT::default();
    let mut button = RECT::default();
    unsafe {
        GetWindowRect(find.wrap, &mut checkbox)?;
        GetWindowRect(find.replace_btn, &mut button)?;
    }
    assert!(checkbox.right <= button.left);
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; verifies shared preference writers"]
fn spellcheck_and_update_saves_preserve_current_editor_preferences() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let hwnd = window.0;
    let assert_preferences = || -> Result<()> {
        let saved = settings::load_settings()?;
        assert_eq!(saved.editor_font_name, "Courier New");
        assert_eq!(saved.editor_font_size, 16);
        assert!(!saved.editor_dark);
        assert_eq!(saved.tab_placement, TabPlacement::Right);
        assert_eq!(saved.vertical_tab_width_px, 320);
        Ok(())
    };
    {
        let state = get_state(hwnd).unwrap();
        set_editor_font(state, "Courier New".to_string(), 16);
        set_editor_dark_mode(hwnd, state, false);
        state.tab_host.vertical_width_px = 320;
        set_tab_layout(hwnd, state, TabPlacement::Right);
        assert_preferences()?;

        toggle_spellcheck(hwnd, state);
        assert_preferences()?;
        assert!(settings::load_settings()?.spellcheck_enabled);
        toggle_spellcheck(hwnd, state);
        assert_preferences()?;
        assert!(!settings::load_settings()?.spellcheck_enabled);

        remember_update_check(state, 123);
        assert_preferences()?;
        assert_eq!(settings::load_settings()?.last_update_check, 123);
    }
    unsafe {
        SendMessageW(
            hwnd,
            WM_COMMAND,
            WPARAM(IDM_HELP_AUTO_UPDATES as usize),
            LPARAM(0),
        );
    }
    assert_preferences()?;
    Ok(())
}
