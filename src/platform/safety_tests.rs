//! Regression tests use hidden windows and a temporary profile, never user notes.
#![allow(clippy::unwrap_used)]

use super::spellcheck_tests::{TestData, TestWindow, register_test_editor};
use super::*;
use windows::Win32::UI::WindowsAndMessaging::MoveWindow;

pub(super) fn window() -> Result<TestWindow> {
    let instance = module_instance()?;
    register_test_editor(instance)?;
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES | ICC_LISTVIEW_CLASSES,
        });
    }
    register_aux_classes(instance)?;
    let class = w!("rivet_safety_integration_test");
    register_window_class(instance, class, Some(wndproc))?;
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class,
            w!("Safety regression test"),
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
    get_state(hwnd).unwrap().ui_settings.spellcheck_enabled = false;
    Ok(TestWindow(hwnd))
}

#[test]
#[ignore = "Requires Windows controls; uses hidden windows and temporary files"]
fn encoding_nuls_and_backup_restore_roundtrip() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf8Bom,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Ansi,
    ] {
        let _data = TestData::new();
        let window = window()?;
        let hwnd = window.0;
        let root = session::data_dir()?;
        let path = root.join("original.txt");
        let original = "café\0after NUL\noriginal";
        std::fs::write(&path, document::encode_text(original, encoding)?).unwrap();
        let state = get_state(hwnd).unwrap();
        open_path_new_tab(hwnd, state, path.clone(), None, None, None, None)?;
        let index = state.active;
        let editor = state.docs[index].editor;
        assert_eq!(scintilla::get_text(editor)?, original);
        let edited = "café\0after NUL\nedited";
        scintilla::set_text(editor, edited)?;
        assert_eq!(scintilla::get_text(editor)?, edited);
        assert!(state.docs[index].doc.is_dirty);
        run_snapshot_tick(hwnd, state, true)?;
        let entry = session::load_session()?
            .entries
            .into_iter()
            .find(|entry| entry.id == state.docs[index].doc.id)
            .unwrap();
        assert_eq!(entry.encoding, Some(encoding));
        assert_eq!(entry.eol, Some(Eol::Lf));
        restore_session_entry(hwnd, state, entry)?;
        let restored = state.docs.len() - 1;
        assert_eq!(state.docs[restored].doc.encoding, encoding);
        assert_eq!(state.docs[restored].doc.eol, Eol::Lf);
        assert_eq!(scintilla::get_text(state.docs[restored].editor)?, edited);
        assert!(save_document_at(hwnd, state, restored, None, false)?);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            document::encode_text(edited, encoding)?
        );
    }
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; tests cancellation without showing prompts"]
fn closing_dirty_tabs_requires_an_explicit_choice() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let hwnd = window.0;
    let state = get_state(hwnd).unwrap();
    assert!(state.session_snapshot_periodic_backup);
    let editor = state.docs[0].editor;
    scintilla::set_text(editor, "unsaved work")?;
    run_snapshot_tick(hwnd, state, true)?;
    let backup = state.docs[0].doc.backup_path.clone();
    let mut prompted = false;
    assert!(!close_tab_with_prompt(hwnd, state, 0, |_, _| {
        prompted = true;
        SaveChoice::Cancel
    })?);
    assert!(prompted);
    assert_eq!(scintilla::get_text(editor)?, "unsaved work");
    assert!(backup.exists());
    assert!(close_tab_with_prompt(hwnd, state, 0, |_, _| {
        SaveChoice::No
    })?);
    assert!(!backup.exists());
    assert_eq!(scintilla::get_text(state.docs[0].editor)?, "");
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; locks an isolated recovery file"]
fn unreadable_recovery_entry_survives_future_checkpoints() -> Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let state = get_state(window.0).unwrap();
    scintilla::set_text(state.docs[0].editor, "recover this note")?;
    run_snapshot_tick(window.0, state, true)?;
    let entry = session::load_session()?.entries.remove(0);
    // Give the failed entry a separate ID, as if another tab failed to open.
    let id = uuid::Uuid::new_v4();
    let entry = session::SessionEntry {
        id,
        backup_path: session::backup_path_for_id(id)?,
        ..entry
    };
    std::fs::write(&entry.backup_path, b"temporarily inaccessible").unwrap();
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&entry.backup_path)
        .unwrap();
    restore_or_preserve_entry(window.0, state, entry.clone());
    assert_eq!(state.unrestored_entries, vec![entry.clone()]);
    save_session_checkpoint(window.0, state)?;
    assert!(
        session::load_session()?
            .entries
            .iter()
            .any(|saved| saved.id == id)
    );
    drop(locked);
    restore_session_entry(window.0, state, entry)?;
    assert_eq!(
        scintilla::get_text(state.docs.last().unwrap().editor)?,
        "temporarily inaccessible"
    );
    state.unrestored_entries.clear();
    let locked_session = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(session::session_file_path()?)
        .unwrap();
    assert!(run_snapshot_tick(window.0, state, true).is_err());
    assert!(state.snapshot_retry_at.is_some());
    let files_before = std::fs::read_dir(session::sessions_dir()?).unwrap().count();
    run_snapshot_tick(window.0, state, false)?;
    assert_eq!(
        std::fs::read_dir(session::sessions_dir()?).unwrap().count(),
        files_before
    );
    drop(locked_session);
    run_snapshot_tick(window.0, state, true)?;
    assert!(state.snapshot_retry_at.is_none());
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; tests shutdown without ending Windows"]
fn shutdown_checkpoints_and_cancelled_discard_keep_notes() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let hwnd = window.0;
    let state = get_state(hwnd).unwrap();
    scintilla::set_text(state.docs[0].editor, "first unsaved note")?;
    assert_eq!(
        unsafe { SendMessageW(hwnd, WM_QUERYENDSESSION, WPARAM(0), LPARAM(0)) }.0,
        1
    );
    let saved = session::load_session()?;
    assert_eq!(
        std::fs::read(&saved.entries[0].backup_path).unwrap(),
        b"first unsaved note"
    );
    unsafe {
        SendMessageW(hwnd, WM_ENDSESSION, WPARAM(0), LPARAM(0));
    }
    create_empty_tab(hwnd, module_instance()?, state)?;
    scintilla::set_text(state.docs[1].editor, "second unsaved note")?;
    state.session_snapshot_periodic_backup = false;
    let mut calls = 0;
    assert!(!confirm_close_all_with_prompt(hwnd, state, |_, _| {
        calls += 1;
        if calls == 1 {
            SaveChoice::No
        } else {
            SaveChoice::Cancel
        }
    })?);
    assert!(state.discard_on_exit.is_empty());
    assert!(state.docs.iter().all(|doc| doc.doc.is_dirty));
    assert!(confirm_close_all_with_prompt(hwnd, state, |_, _| {
        SaveChoice::No
    })?);
    save_session_checkpoint_with_discard(hwnd, state, &state.discard_on_exit)?;
    assert!(session::load_session()?.entries.is_empty());
    // A cancelled operation has not actually cleared any edited buffer.
    save_session_checkpoint(hwnd, state)?;
    assert_eq!(session::load_session()?.entries.len(), 2);
    state.session_snapshot_periodic_backup = true;
    state.discard_on_exit.clear();
    unsafe {
        SendMessageW(hwnd, WM_ENDSESSION, WPARAM(1), LPARAM(0));
    }
    assert_eq!(session::load_session()?.entries.len(), 2);
    assert!(!session::data_dir()?.join("updates").exists());
    Ok(())
}

#[test]
#[ignore = "Requires Windows controls; exercises the bundled native editor"]
fn editing_undo_redo_lexers_and_invalid_ipc() -> Result<()> {
    let _lock = session::test_env_lock().lock().unwrap();
    let _data = TestData::new();
    let window = window()?;
    let editor = get_state(window.0).unwrap().docs[0].editor;
    use scintilla::LexerKind::*;
    for lexer in [
        Null, Cpp, JavaScript, Json, Yaml, PowerShell, Python, Html, Xml, Css, Properties, Markdown,
    ] {
        for dark in [false, true] {
            let original = "# Title\r\nhello café\0tail\r\n{} <tag> /* comment */";
            scintilla::set_text(editor, original)?;
            scintilla::apply_lexer(editor, lexer, dark, "Consolas", 11);
            scintilla::set_wrap_enabled(editor, true);
            scintilla::set_target_range(editor, 0, 7);
            scintilla::replace_target(editor, "new");
            assert!(scintilla::get_text(editor)?.starts_with("new\r\n"));
            scintilla::undo(editor);
            assert_eq!(scintilla::get_text(editor)?, original);
            scintilla::redo(editor);
            assert!(scintilla::get_text(editor)?.starts_with("new\r\n"));
            // A nearly zero-width editor used to trigger upstream wrap crashes.
            unsafe {
                MoveWindow(editor, 0, 0, 1, 200, true)?;
            }
            scintilla::get_text(editor)?;
        }
    }
    for (magic, size) in [
        (0, 8),
        (single_instance::COPYDATA_OPEN_FILES, 3),
        (
            single_instance::COPYDATA_OPEN_FILES,
            single_instance::MAX_COPYDATA_BYTES as u32 + 2,
        ),
    ] {
        let mut byte = 0u8;
        let message = COPYDATASTRUCT {
            dwData: magic,
            cbData: size,
            lpData: (&mut byte as *mut u8).cast(),
        };
        assert_eq!(
            unsafe {
                SendMessageW(
                    window.0,
                    WM_COPYDATA,
                    WPARAM(0),
                    LPARAM(&message as *const COPYDATASTRUCT as isize),
                )
            }
            .0,
            0
        );
    }
    Ok(())
}
