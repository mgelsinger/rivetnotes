//! Desktop integration test with a hidden Rivet window and isolated user data.
#![allow(clippy::unwrap_used)]

use super::*;

pub(super) struct TestData {
    _directory: tempfile::TempDir,
    local: Option<std::ffi::OsString>,
    roaming: Option<std::ffi::OsString>,
}

impl TestData {
    pub(super) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let local = std::env::var_os("LOCALAPPDATA");
        let roaming = std::env::var_os("APPDATA");
        unsafe {
            std::env::set_var("LOCALAPPDATA", directory.path());
            std::env::set_var("APPDATA", directory.path());
        }
        Self {
            _directory: directory,
            local,
            roaming,
        }
    }
}

impl Drop for TestData {
    fn drop(&mut self) {
        for (key, value) in [("LOCALAPPDATA", &self.local), ("APPDATA", &self.roaming)] {
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

pub(super) struct TestWindow(pub(super) HWND);

pub(super) fn register_test_editor(instance: HINSTANCE) -> Result<()> {
    static REGISTERED: std::sync::OnceLock<std::result::Result<(), String>> =
        std::sync::OnceLock::new();
    REGISTERED
        .get_or_init(|| scintilla::register_classes(instance).map_err(|error| error.to_string()))
        .as_ref()
        .map(|_| ())
        .map_err(|error| AppError::new(error.clone()))
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

fn wait_for_scan(hwnd: HWND) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let state = get_state(hwnd).ok_or_else(|| AppError::new("Missing test window state"))?;
        handle_spellcheck_timer(state);
        if let Some(error) = &state.spelling_error {
            return Err(AppError::new(error));
        }
        let doc = &state.docs[state.active];
        if state
            .spelling_worker
            .as_ref()
            .is_some_and(|worker| worker.ready)
            && doc.spelling.mode.is_some()
            && doc.spelling.next == scintilla::get_length(doc.editor)
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(AppError::new("Spellcheck integration test timed out"))
}

#[test]
#[ignore = "Requires a desktop and Windows spelling provider; uses a hidden window"]
fn spellcheck_editor_integration() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let instance = module_instance()?;
    register_test_editor(instance)?;
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES | ICC_LISTVIEW_CLASSES,
        });
    }
    register_aux_classes(instance)?;
    let class = w!("rivet_spellcheck_integration_test");
    let registered = unsafe {
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        })
    };
    assert_ne!(registered, 0);
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class,
            w!("Spellcheck integration test"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            800,
            600,
            HWND(0),
            create_menu()?,
            instance,
            None,
        )
    };
    assert_ne!(hwnd.0, 0);
    let _window = TestWindow(hwnd);
    let state = get_state(hwnd).unwrap();
    set_language_override(state, Some(scintilla::LexerKind::Markdown));
    let editor = state.docs[0].editor;
    let text = "Hello \u{1f642} qzxqzxqzx.\n`qzxqzxqzx`\nhttps://qzxqzxqzx.example\n";
    scintilla::set_text(editor, text)?;
    scintilla::set_savepoint(editor);
    let revision = get_state(hwnd).unwrap().docs[0].change_counter;
    scintilla::set_indicator_current(editor, STRIKE_INDIC);
    scintilla::set_indicator_value(editor, 1);
    scintilla::fill_indicator_range(editor, 0, 5);
    wait_for_scan(hwnd)?;
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 1);
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 24), 0);
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 43), 0);
    assert_eq!(scintilla::indicator_value_at(editor, STRIKE_INDIC, 0), 1);
    assert_eq!(scintilla::get_text(editor)?, text);
    assert_eq!(get_state(hwnd).unwrap().docs[0].change_counter, revision);
    assert!(!get_state(hwnd).unwrap().docs[0].doc.is_dirty);

    // Correcting a word and undoing/redoing must repaint the right byte ranges.
    scintilla::set_target_range(editor, 11, 20);
    scintilla::replace_target(editor, "hello");
    wait_for_scan(hwnd)?;
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 0);
    scintilla::undo(editor);
    wait_for_scan(hwnd)?;
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 1);
    scintilla::redo(editor);
    wait_for_scan(hwnd)?;
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 0);
    scintilla::undo(editor);
    wait_for_scan(hwnd)?;

    // Switching tabs and changing themes must preserve independent indicators.
    let state = get_state(hwnd).unwrap();
    create_empty_tab(hwnd, instance, state)?;
    let second = state.docs[state.active].editor;
    scintilla::set_text(second, "qzxqzxqzx")?;
    wait_for_scan(hwnd)?;
    assert_eq!(scintilla::indicator_value_at(second, SPELLING_INDIC, 0), 1);
    let state = get_state(hwnd).unwrap();
    select_tab(hwnd, state, 0);
    set_editor_dark_mode(hwnd, state, false);
    scintilla::set_zoom(editor, 5);
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 1);
    assert_eq!(scintilla::indicator_value_at(editor, STRIKE_INDIC, 0), 1);

    // The toggle clears every tab immediately and persists across settings loads.
    toggle_spellcheck(hwnd, get_state(hwnd).unwrap());
    assert!(!settings::load_settings()?.spellcheck_enabled);
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 0);
    assert_eq!(scintilla::indicator_value_at(second, SPELLING_INDIC, 0), 0);
    toggle_spellcheck(hwnd, get_state(hwnd).unwrap());
    wait_for_scan(hwnd)?;
    assert!(settings::load_settings()?.spellcheck_enabled);
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 1);

    // Unsupported source files and Large File Mode never receive underlines.
    let state = get_state(hwnd).unwrap();
    state.docs[0].doc.path = Some(PathBuf::from("example.rs"));
    set_language_override(state, None);
    handle_spellcheck_timer(state);
    assert!(state.docs[0].spelling.mode.is_none());
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 0);
    set_language_override(state, Some(scintilla::LexerKind::Markdown));
    wait_for_scan(hwnd)?;
    let state = get_state(hwnd).unwrap();
    state.docs[0].doc.large_file_mode = true;
    handle_spellcheck_timer(state);
    assert!(state.docs[0].spelling.mode.is_none());
    assert_eq!(scintilla::indicator_value_at(editor, SPELLING_INDIC, 11), 0);

    // A word crossing the 16 KiB boundary in a wrapped plain-text paragraph
    // must still be checked as one word.
    state.docs[0].doc.large_file_mode = false;
    set_language_override(state, Some(scintilla::LexerKind::Null));
    let prefix = "hello ".repeat(2730);
    let long_note = format!("{prefix}qzxqzxqzx {}", "hello ".repeat(3000));
    scintilla::set_text(editor, &long_note)?;
    wait_for_scan(hwnd)?;
    assert_eq!(
        scintilla::indicator_value_at(editor, SPELLING_INDIC, prefix.len()),
        1
    );
    assert_eq!(scintilla::get_text(editor)?, long_note);
    Ok(())
}

#[test]
#[ignore = "Requires a desktop; uses a hidden window and isolated user data"]
fn updater_inline_controls_and_failed_checkpoint() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let instance = module_instance()?;
    register_test_editor(instance)?;
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES | ICC_LISTVIEW_CLASSES,
        });
    }
    register_aux_classes(instance)?;
    let class = w!("rivet_updater_integration_test");
    assert_ne!(
        unsafe {
            RegisterClassExW(&WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                hInstance: instance,
                lpszClassName: class,
                lpfnWndProc: Some(wndproc),
                ..Default::default()
            })
        },
        0
    );
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class,
            w!("Updater integration test"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            800,
            600,
            HWND(0),
            create_menu()?,
            instance,
            None,
        )
    };
    assert_ne!(hwnd.0, 0);
    let _window = TestWindow(hwnd);
    let state = get_state(hwnd).unwrap();
    assert!(!state.ui_settings.automatic_updates);
    assert!(!state.updates.supported());
    unsafe {
        SendMessageW(
            hwnd,
            WM_COMMAND,
            WPARAM(IDM_HELP_CHECK_UPDATES as usize),
            LPARAM(0),
        );
    }
    let state = get_state(hwnd).unwrap();
    assert!(state.updates.status().contains("installed copy"));
    assert_eq!(
        unsafe { SendMessageW(state.status, SB_GETPARTS, WPARAM(0), LPARAM(0)).0 },
        7
    );
    state.updates.simulate_ready();
    update_status(state);
    update_updates_menu(hwnd, state);
    let mut rect = RECT::default();
    unsafe {
        SendMessageW(
            state.status,
            SB_GETRECT,
            WPARAM(6),
            LPARAM(&mut rect as *mut RECT as isize),
        );
    }
    assert!(rect.right > rect.left);
    // A failed backup must leave both the window and edited buffer intact,
    // without launching the helper or showing an updater dialog.
    state.session_snapshot_periodic_backup = true;
    let editor = state.docs[0].editor;
    scintilla::set_text(editor, "Unsaved notes must survive an update")?;
    let state = get_state(hwnd).unwrap();
    state.docs[0].doc.is_dirty = true;
    let obstacle = _data._directory.path().join("not-a-directory");
    std::fs::write(&obstacle, b"blocked").unwrap();
    state.docs[0].doc.backup_path = obstacle.join("backup.txt");
    request_app_close(hwnd, true);
    let state = get_state(hwnd).unwrap();
    assert_eq!(
        state.updates.status(),
        "Update postponed - could not save notes"
    );
    assert!(!state.close_checkpoint_saved);
    assert_eq!(
        scintilla::get_text(editor)?,
        "Unsaved notes must survive an update"
    );
    state.updates.set_enabled(false);
    assert!(!state.updates.ready());
    assert!(!state.updates.install_on_exit());
    state.updates.simulate_machine_install();
    state.updates.simulate_ready();
    assert!(state.updates.supported());
    assert!(!state.updates.install_on_exit());
    update_updates_menu(hwnd, state);
    let mut text = [0u16; 100];
    let mut menu_info = MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_STRING,
        dwTypeData: PWSTR(text.as_mut_ptr()),
        cch: text.len() as u32,
        ..Default::default()
    };
    unsafe {
        GetMenuItemInfoW(
            GetMenu(hwnd),
            IDM_HELP_RESTART_UPDATE as u32,
            false,
            &mut menu_info,
        )?;
    }
    assert!(String::from_utf16_lossy(&text).contains("administrator approval"));
    // Ordinary closing of a ready machine install must never launch a helper
    // or request elevation, even with background updates enabled.
    state.docs[0].doc.backup_path = _data._directory.path().join("valid-backup.txt");
    request_app_close(hwnd, false);
    assert!(!unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) }.as_bool());
    let update_dir = session::data_dir()?.join("updates");
    assert!(!update_dir.exists());
    Ok(())
}
