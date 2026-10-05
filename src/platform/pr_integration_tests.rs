//! Regressions for the accepted editing and window-behavior pull requests.
#![allow(clippy::unwrap_used)]

use super::safety_tests::window;
use super::spellcheck_tests::{TestData, TestWindow};
use super::*;
use std::cell::Cell;
use windows::Win32::UI::Controls::{EM_GETSEL, EM_REPLACESEL};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyboardState, IsWindowEnabled, SetKeyboardState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyAcceleratorTableW, GetDlgItem, IsWindow, LB_GETCOUNT, PM_REMOVE, PeekMessageW,
    SW_SHOWNOACTIVATE, WM_COPY, WM_CUT, WM_PASTE, WM_QUIT,
};

struct TestKeyboardState([u8; 256]);

impl TestKeyboardState {
    fn new(pressed: &[u16]) -> Result<Self> {
        let mut previous = [0; 256];
        unsafe { GetKeyboardState(&mut previous)? };
        let mut state = [0; 256];
        for &key in pressed {
            state[key as usize] = 0x80;
        }
        // This changes only this test thread's state, without physical input.
        unsafe { SetKeyboardState(&state)? };
        Ok(Self(previous))
    }
}

impl Drop for TestKeyboardState {
    fn drop(&mut self) {
        let _ = unsafe { SetKeyboardState(&self.0) };
    }
}

fn keyboard_test_dialog(main: HWND, class: PCWSTR) -> Result<TestWindow> {
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class,
            w!("Dialog keyboard regression test"),
            WS_OVERLAPPEDWINDOW,
            -32000,
            -32000,
            520,
            240,
            main,
            HMENU(0),
            module_instance()?,
            Some(main.0 as *const std::ffi::c_void),
        )
    };
    assert_ne!(hwnd.0, 0);
    // The routing requires a visible dialog. Keep it off screen and avoid
    // activating it so this regression does not interrupt the user's desktop.
    unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    assert!(unsafe { IsWindowVisible(hwnd) }.as_bool());
    Ok(TestWindow(hwnd))
}

fn edit_selection(edit: HWND) -> (u32, u32) {
    let (mut start, mut end) = (0_u32, 0_u32);
    unsafe {
        SendMessageW(
            edit,
            EM_GETSEL,
            WPARAM(&mut start as *mut u32 as usize),
            LPARAM(&mut end as *mut u32 as isize),
        );
    }
    (start, end)
}

fn dispatch_test_key(main: HWND, target: HWND, key: u16) -> Result<()> {
    let accelerator = create_accelerators()?;
    let result = (|| -> Result<()> {
        unsafe {
            // TestWindow destruction can leave a quit message on the thread.
            let mut message = MSG::default();
            while PeekMessageW(&mut message, HWND(0), WM_QUIT, WM_QUIT, PM_REMOVE).as_bool() {}
            PostMessageW(target, WM_KEYDOWN, WPARAM(key as usize), LPARAM(1))?;
            PostQuitMessage(0);
        }
        message_loop(main, accelerator)?;
        // TranslateMessage may enqueue WM_CHAR after the test's quit message.
        // Deliver it before inspecting the native edit control's behavior.
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, target, WM_CHAR, WM_CHAR, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
        }
        Ok(())
    })();
    unsafe { DestroyAcceleratorTable(accelerator) };
    result
}

struct ShortcutProbe {
    main: HWND,
    target: HWND,
    character: Cell<usize>,
    document_command: Cell<u16>,
}

impl ShortcutProbe {
    fn new(main: HWND, target: HWND) -> Box<Self> {
        let probe = Box::new(Self {
            main,
            target,
            character: Cell::new(0),
            document_command: Cell::new(0),
        });
        let data = &*probe as *const Self as usize;
        for hwnd in [main, target] {
            assert!(unsafe { SetWindowSubclass(hwnd, Some(shortcut_probe), data, data) }.as_bool());
        }
        probe
    }
}

impl Drop for ShortcutProbe {
    fn drop(&mut self) {
        for hwnd in [self.main, self.target] {
            let _ = unsafe {
                RemoveWindowSubclass(hwnd, Some(shortcut_probe), self as *const Self as usize)
            };
        }
    }
}

unsafe extern "system" fn shortcut_probe(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let probe = unsafe { &*(data as *const ShortcutProbe) };
    if hwnd == probe.main && message == WM_COMMAND {
        let command = (wparam.0 & 0xffff) as u16;
        if matches!(
            command,
            IDM_EDIT_COPY | IDM_EDIT_CUT | IDM_EDIT_PASTE | IDM_EDIT_FIND | IDM_EDIT_REPLACE
        ) {
            // Observe document shortcuts without opening dialogs or accessing
            // the user's clipboard, including when a field key is misrouted.
            probe.document_command.set(command);
            return LRESULT(0);
        }
    }
    if hwnd == probe.target && message == WM_CHAR {
        probe.character.set(wparam.0);
        if matches!(wparam.0, 0x03 | 0x18 | 0x16) {
            // Observe the translated Ctrl+C/X/V before the native control can
            // access the user's clipboard. Ctrl+Z/Y still run natively.
            return LRESULT(0);
        }
    }
    if matches!(message, WM_COPY | WM_CUT | WM_PASTE) {
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[test]
#[ignore = "Requires Windows controls; uses off-screen dialogs and isolated user data"]
fn dialog_edit_shortcuts_stay_in_the_focused_field() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let _find = keyboard_test_dialog(main.0, FIND_CLASS)?;
    let _files = keyboard_test_dialog(main.0, FIND_FILES_CLASS)?;
    let _go_to = keyboard_test_dialog(main.0, GOTO_LINE_CLASS)?;
    let (editor, targets) = {
        let state = get_state(main.0).unwrap();
        state.search_dialog_mode = SearchDialogMode::Replace;
        apply_search_state_to_dialog(state)?;
        let find = state.find_dialog.as_ref().unwrap();
        let files = state.find_in_files.as_ref().unwrap();
        let go_to = state.go_to_line_dialog.as_ref().unwrap();
        (
            active_editor(state).unwrap(),
            [
                find.find_edit,
                find.replace_edit,
                files.find_edit,
                files.folder_edit,
                files.include_edit,
                files.exclude_edit,
                go_to.line_edit,
            ],
        )
    };
    scintilla::set_text(editor, "Before document edit")?;
    scintilla::set_text(editor, "Current document edit")?;
    scintilla::set_text(editor, "Future document edit")?;
    scintilla::undo(editor);
    scintilla::set_selection(editor, 3, 7);
    let assert_document_unchanged = || -> Result<()> {
        assert_eq!(scintilla::get_text(editor)?, "Current document edit");
        assert_eq!(scintilla::selection_start(editor), 3);
        assert_eq!(scintilla::selection_end(editor), 7);
        assert!(scintilla::can_undo(editor));
        assert!(scintilla::can_redo(editor));
        Ok(())
    };
    let _keyboard = TestKeyboardState::new(&[VK_CONTROL.0])?;
    for target in targets {
        let probe = ShortcutProbe::new(main.0, target);
        set_window_text(target, "12345");
        unsafe {
            SetFocus(target);
            SendMessageW(target, EM_SETSEL, WPARAM(1), LPARAM(3));
            SendMessageW(
                target,
                EM_REPLACESEL,
                WPARAM(1),
                LPARAM(w!("90").as_ptr() as isize),
            );
        }
        assert_eq!(get_window_text(target)?, "19045");
        dispatch_test_key(main.0, target, VK_A)?;
        assert_eq!(edit_selection(target), (0, 5));
        assert_document_unchanged()?;
        for (key, character) in [(VK_C, 0x03), (VK_X, 0x18), (VK_V, 0x16)] {
            probe.character.set(0);
            dispatch_test_key(main.0, target, key)?;
            assert_eq!(probe.document_command.get(), 0);
            assert_eq!(probe.character.get(), character);
            assert_eq!(get_window_text(target)?, "19045");
            assert_document_unchanged()?;
        }
        dispatch_test_key(main.0, target, VK_Z)?;
        assert_eq!(get_window_text(target)?, "12345");
        assert_document_unchanged()?;
        probe.character.set(0);
        dispatch_test_key(main.0, target, VK_Y)?;
        assert_eq!(probe.character.get(), 0x19);
        assert_document_unchanged()?;
    }
    // The same accelerators must still reach the document when it has focus,
    // even while the custom dialogs remain visible.
    unsafe { SetFocus(editor) };
    let probe = ShortcutProbe::new(main.0, editor);
    for (key, command) in [
        (VK_C, IDM_EDIT_COPY),
        (VK_X, IDM_EDIT_CUT),
        (VK_V, IDM_EDIT_PASTE),
    ] {
        probe.document_command.set(0);
        dispatch_test_key(main.0, editor, key)?;
        assert_eq!(probe.document_command.get(), command);
    }
    dispatch_test_key(main.0, editor, VK_Z)?;
    assert_eq!(scintilla::get_text(editor)?, "Before document edit");
    dispatch_test_key(main.0, editor, VK_Y)?;
    assert_eq!(scintilla::get_text(editor)?, "Current document edit");
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses an off-screen dialog and isolated user data"]
fn find_and_replace_ctrl_a_selects_only_the_target_field() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let dialog = keyboard_test_dialog(main.0, FIND_CLASS)?;
    let editor = active_editor(get_state(main.0).unwrap()).unwrap();
    let document = "Document selection must stay unchanged";
    scintilla::set_text(editor, document)?;
    let _keyboard = TestKeyboardState::new(&[VK_CONTROL.0])?;
    for mode in [SearchDialogMode::Find, SearchDialogMode::Replace] {
        let (find, replace) = {
            let state = get_state(main.0).unwrap();
            state.search_dialog_mode = mode;
            apply_search_state_to_dialog(state)?;
            let dialog = state.find_dialog.as_ref().unwrap();
            (dialog.find_edit, dialog.replace_edit)
        };
        set_window_text(find, "Find café");
        set_window_text(replace, "Replace café");
        for (target, other, expected_length) in [(find, replace, 9), (replace, find, 12)] {
            if mode == SearchDialogMode::Find && target == replace {
                continue;
            }
            scintilla::set_selection(editor, 3, 7);
            unsafe {
                SetFocus(target);
                SendMessageW(target, EM_SETSEL, WPARAM(2), LPARAM(2));
                SendMessageW(other, EM_SETSEL, WPARAM(1), LPARAM(1));
            }
            dispatch_test_key(main.0, target, VK_A)?;
            assert_eq!(edit_selection(target), (0, expected_length));
            assert_eq!(edit_selection(other), (1, 1));
            assert_eq!(scintilla::selection_start(editor), 3);
            assert_eq!(scintilla::selection_end(editor), 7);
            assert_eq!(scintilla::get_text(editor)?, document);
            assert_eq!(get_window_text(find)?, "Find café");
            assert_eq!(get_window_text(replace)?, "Replace café");
        }
    }
    // A visible Find dialog must not intercept an editor accelerator.
    unsafe { SetFocus(editor) };
    dispatch_test_key(main.0, editor, VK_A)?;
    assert_eq!(scintilla::selection_start(editor), 0);
    assert_eq!(scintilla::selection_end(editor), document.len());
    unsafe { ShowWindow(dialog.0, SW_HIDE) };
    let find = get_state(main.0)
        .unwrap()
        .find_dialog
        .as_ref()
        .unwrap()
        .find_edit;
    for key in [VK_A, VK_ESCAPE.0] {
        let message = MSG {
            hwnd: find,
            message: WM_KEYDOWN,
            wParam: WPARAM(key as usize),
            ..Default::default()
        };
        assert!(!handle_dialog_key(main.0, &message));
    }
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses an off-screen dialog and isolated user data"]
fn find_and_replace_escape_closes_from_children_and_restores_editor_focus() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let dialog = keyboard_test_dialog(main.0, FIND_CLASS)?;
    let editor = active_editor(get_state(main.0).unwrap()).unwrap();
    let _keyboard = TestKeyboardState::new(&[])?;
    dispatch_test_key(main.0, editor, VK_ESCAPE.0)?;
    assert!(unsafe { IsWindowVisible(dialog.0) }.as_bool());
    for mode in [SearchDialogMode::Find, SearchDialogMode::Replace] {
        let (find, replace, checkbox) = {
            let state = get_state(main.0).unwrap();
            state.search_dialog_mode = mode;
            apply_search_state_to_dialog(state)?;
            let dialog = state.find_dialog.as_ref().unwrap();
            (dialog.find_edit, dialog.replace_edit, dialog.match_case)
        };
        let close = unsafe { GetDlgItem(dialog.0, IDC_FIND_CLOSE as i32) };
        assert_ne!(close.0, 0);
        for target in [find, replace, checkbox, close, dialog.0] {
            if mode == SearchDialogMode::Find && target == replace {
                continue;
            }
            unsafe {
                ShowWindow(dialog.0, SW_SHOWNOACTIVATE);
                SetFocus(target);
            }
            assert!(unsafe { IsWindowVisible(dialog.0) }.as_bool());
            dispatch_test_key(main.0, target, VK_ESCAPE.0)?;
            assert!(!unsafe { IsWindowVisible(dialog.0) }.as_bool());
            assert_eq!(unsafe { GetFocus() }, editor);
            assert!(get_state(main.0).unwrap().find_dialog.is_some());
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses off-screen dialogs and isolated user data"]
fn go_to_line_escape_cancels_and_reenables_the_editor() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let find = keyboard_test_dialog(main.0, FIND_CLASS)?;
    let editor = active_editor(get_state(main.0).unwrap()).unwrap();
    let document = "First line\nSecond line\nThird line";
    scintilla::set_text(editor, document)?;
    scintilla::set_selection(editor, 4, 4);
    let _keyboard = TestKeyboardState::new(&[])?;
    for control in [
        Some(IDC_GOTO_LINE),
        Some(IDC_GOTO_GO),
        Some(IDC_GOTO_CANCEL),
        None,
    ] {
        let dialog = keyboard_test_dialog(main.0, GOTO_LINE_CLASS)?;
        let line_edit = get_state(main.0)
            .unwrap()
            .go_to_line_dialog
            .as_ref()
            .unwrap()
            .line_edit;
        set_window_text(line_edit, "3");
        let target = control.map_or(dialog.0, |id| unsafe { GetDlgItem(dialog.0, id as i32) });
        unsafe {
            EnableWindow(main.0, BOOL(0));
            SetFocus(target);
        }
        assert!(!unsafe { IsWindowEnabled(main.0) }.as_bool());
        dispatch_test_key(main.0, target, VK_ESCAPE.0)?;
        assert!(!unsafe { IsWindow(dialog.0) }.as_bool());
        assert!(get_state(main.0).unwrap().go_to_line_dialog.is_none());
        assert!(unsafe { IsWindowEnabled(main.0) }.as_bool());
        assert_eq!(unsafe { GetFocus() }, editor);
        assert_eq!(scintilla::get_current_pos(editor), 4);
        assert_eq!(scintilla::get_text(editor)?, document);
        assert!(unsafe { IsWindowVisible(find.0) }.as_bool());
    }
    dispatch_test_key(main.0, editor, VK_ESCAPE.0)?;
    assert!(unsafe { IsWindow(main.0) }.as_bool());
    assert!(unsafe { IsWindowVisible(find.0) }.as_bool());
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses off-screen dialogs and isolated user data"]
fn find_in_files_escape_preserves_results_and_background_search() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let find = keyboard_test_dialog(main.0, FIND_CLASS)?;
    let files = keyboard_test_dialog(main.0, FIND_FILES_CLASS)?;
    let editor = active_editor(get_state(main.0).unwrap()).unwrap();
    let (_sender, receiver) = mpsc::channel();
    let (mut targets, cancellation, results) = {
        let state = get_state(main.0).unwrap();
        let dialog = state.find_in_files.as_mut().unwrap();
        dialog.running = true;
        dialog.receiver = Some(receiver);
        dialog.hits.push(FindHit {
            path: PathBuf::from("test.txt"),
            line: 7,
            text: "keep this result".into(),
        });
        set_window_text(dialog.find_edit, "keep this query");
        unsafe {
            SendMessageW(
                dialog.results,
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(w!("keep this result").as_ptr() as isize),
            );
        }
        (
            vec![
                dialog.find_edit,
                dialog.folder_edit,
                dialog.include_edit,
                dialog.exclude_edit,
                dialog.results,
            ],
            dialog.cancel.clone(),
            dialog.results,
        )
    };
    for id in [IDC_FIF_FIND, IDC_FIF_CANCEL, IDC_FIF_CLOSE, IDC_FIF_BROWSE] {
        targets.push(unsafe { GetDlgItem(files.0, id as i32) });
    }
    targets.push(files.0);
    let _keyboard = TestKeyboardState::new(&[])?;
    for target in targets {
        unsafe {
            ShowWindow(files.0, SW_SHOWNOACTIVATE);
            SetFocus(target);
        }
        dispatch_test_key(main.0, target, VK_ESCAPE.0)?;
        assert!(!unsafe { IsWindowVisible(files.0) }.as_bool());
        assert!(unsafe { IsWindowVisible(find.0) }.as_bool());
        assert_eq!(unsafe { GetFocus() }, editor);
        let state = get_state(main.0).unwrap();
        let dialog = state.find_in_files.as_ref().unwrap();
        assert!(dialog.running && dialog.receiver.is_some());
        assert!(Arc::ptr_eq(&dialog.cancel, &cancellation));
        assert!(!cancellation.load(Ordering::SeqCst));
        assert_eq!(dialog.hits.len(), 1);
        assert_eq!(dialog.hits[0].text, "keep this result");
        assert_eq!(get_window_text(dialog.find_edit)?, "keep this query");
        assert_eq!(
            unsafe { SendMessageW(results, LB_GETCOUNT, WPARAM(0), LPARAM(0)) }.0,
            1
        );
    }
    Ok(())
}

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
    assert!(std::mem::align_of::<AcceleratorEntries<35>>() >= 8);
    assert_eq!(std::mem::size_of::<ACCEL>(), 6);
    for _ in 0..100 {
        let accelerator = create_accelerators()?;
        let mut entries = AcceleratorEntries([ACCEL::default(); 35]);
        assert_eq!(entries.0.as_ptr() as usize % 8, 0);
        unsafe {
            assert_eq!(CopyAcceleratorTableW(accelerator, None), 35);
            assert_eq!(CopyAcceleratorTableW(accelerator, Some(&mut entries.0)), 35);
            assert!(DestroyAcceleratorTable(accelerator).as_bool());
        }
        for (flags, key, command) in [
            (FVIRTKEY | FCONTROL, VK_F, IDM_EDIT_FIND),
            (FVIRTKEY | FCONTROL, VK_H, IDM_EDIT_REPLACE),
            (FVIRTKEY, VK_F5, IDM_EDIT_INSERT_DATETIME),
        ] {
            assert_eq!(
                entries
                    .0
                    .iter()
                    .filter(|entry| entry.fVirt == flags
                        && entry.key == key
                        && entry.cmd == command)
                    .count(),
                1
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; uses a hidden window and isolated user data"]
fn find_and_replace_shortcuts_send_commands_without_inserting_control_characters() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let main = window()?;
    let editor = active_editor(get_state(main.0).unwrap()).unwrap();
    let document = "Find and Replace must leave this document unchanged";
    scintilla::set_text(editor, document)?;
    scintilla::set_selection(editor, 5, 8);
    let probe = ShortcutProbe::new(main.0, editor);
    let _keyboard = TestKeyboardState::new(&[VK_CONTROL.0])?;
    for (key, command) in [(VK_F, IDM_EDIT_FIND), (VK_H, IDM_EDIT_REPLACE)] {
        probe.character.set(0);
        probe.document_command.set(0);
        dispatch_test_key(main.0, editor, key)?;
        assert_eq!(probe.document_command.get(), command);
        assert_eq!(probe.character.get(), 0);
        assert_eq!(scintilla::get_text(editor)?, document);
        assert_eq!(scintilla::selection_start(editor), 5);
        assert_eq!(scintilla::selection_end(editor), 8);
        assert!(get_state(main.0).unwrap().find_dialog.is_none());
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
