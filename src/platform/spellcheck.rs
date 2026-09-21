//! Windows spelling providers live exclusively on the worker's COM apartment.

use std::sync::mpsc::{self, Receiver, SyncSender};

use windows::Win32::Globalization::{
    CORRECTIVE_ACTION_NONE, GetUserDefaultLocaleName, ISpellChecker, ISpellCheckerFactory,
    SpellCheckerFactory,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::core::PCWSTR;

use crate::editor::spellcheck::{self, Checked, Job};
use crate::error::{AppError, Result};

pub enum Event {
    Ready(String),
    Checked(Checked),
    Unavailable(String),
}

pub struct Worker {
    jobs: SyncSender<Job>,
    pub results: Receiver<Event>,
    pub busy: bool,
    pub ready: bool,
}

impl Worker {
    pub fn start() -> Self {
        let (jobs, requests) = mpsc::sync_channel(1);
        let (sender, results) = mpsc::channel();
        let failed = sender.clone();
        if let Err(err) = std::thread::Builder::new()
            .name("rivet-spellcheck".into())
            .spawn(move || {
                if let Err(err) = run(requests, &sender) {
                    let _ = sender.send(Event::Unavailable(err.to_string()));
                }
            })
        {
            let _ = failed.send(Event::Unavailable(format!(
                "Could not start spellcheck: {err}"
            )));
        }
        Self {
            jobs,
            results,
            busy: false,
            ready: false,
        }
    }

    pub fn submit(&mut self, job: Job) -> bool {
        if self.busy || !self.ready {
            return false;
        }
        self.busy = self.jobs.try_send(job).is_ok();
        self.busy
    }
}

struct Apartment;

impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn run(requests: Receiver<Job>, sender: &mpsc::Sender<Event>) -> Result<()> {
    let _apartment = Apartment::new()?;
    let (checker, language) = create_checker()?;
    if sender.send(Event::Ready(language)).is_err() {
        return Ok(());
    }
    while let Ok(job) = requests.recv() {
        let checked = check(&checker, job)?;
        if sender.send(Event::Checked(checked)).is_err() {
            break;
        }
    }
    Ok(())
}

fn create_checker() -> Result<(ISpellChecker, String)> {
    let factory: ISpellCheckerFactory =
        unsafe { CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER)? };
    let mut locale = [0u16; 85];
    let count = unsafe { GetUserDefaultLocaleName(&mut locale) };
    let preferred = if count > 0 {
        String::from_utf16_lossy(&locale[..count as usize - 1])
    } else {
        String::new()
    };
    for language in [preferred.as_str(), "en-US", "en-GB"] {
        if language.is_empty() {
            continue;
        }
        let wide: Vec<u16> = language.encode_utf16().chain(Some(0)).collect();
        if unsafe { factory.IsSupported(PCWSTR(wide.as_ptr()))? }.as_bool() {
            let checker = unsafe { factory.CreateSpellChecker(PCWSTR(wide.as_ptr()))? };
            return Ok((checker, language.to_string()));
        }
    }
    Err(AppError::new(
        "No spelling dictionary is installed for your Windows language or English. Add a language's Basic typing feature in Windows Settings, then turn Spellcheck on again.",
    ))
}

fn check(checker: &ISpellChecker, job: Job) -> Result<Checked> {
    let prepared = spellcheck::prepare(&job);
    let mut checked = Checked {
        key: job.key,
        next: job.key.start + job.text.len(),
        context: prepared.context,
        ranges: Vec::new(),
    };
    if !prepared.text.chars().any(char::is_alphabetic) {
        return Ok(checked);
    }
    let wide: Vec<u16> = prepared.text.encode_utf16().chain(Some(0)).collect();
    let errors = unsafe { checker.Check(PCWSTR(wide.as_ptr()))? };
    // Bound output even if a third-party provider returns a broken enumeration.
    for _ in 0..4096 {
        let mut error = None;
        unsafe {
            errors.Next(&mut error).ok()?;
        }
        let Some(error) = error else {
            break;
        };
        if unsafe { error.CorrectiveAction()? } == CORRECTIVE_ACTION_NONE {
            continue;
        }
        let start = unsafe { error.StartIndex()? };
        let length = unsafe { error.Length()? };
        if let Some(range) = prepared.byte_range(start, length) {
            checked
                .ranges
                .push(job.key.start + range.start..job.key.start + range.end);
        }
    }
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::spellcheck::{JobKey, ProseState};

    #[test]
    #[ignore = "Requires a Windows spelling provider; run explicitly on a desktop"]
    fn windows_provider_finds_misspellings_without_modifying_text() -> Result<()> {
        let _apartment = Apartment::new()?;
        let (checker, language) = create_checker()?;
        let text = "Hello \u{1f642} qzxqzxqzx. `qzxqzxqzx` https://qzxqzxqzx.example";
        let result = check(
            &checker,
            Job {
                key: JobKey {
                    tab: 1,
                    revision: 0,
                    generation: 0,
                    start: 0,
                },
                text: text.into(),
                markdown: true,
                context: ProseState::default(),
                end_of_document: true,
            },
        )?;
        assert!(
            result.ranges.contains(&(11..20)),
            "provider language: {language}"
        );
        assert!(result.ranges.iter().all(|range| range.end <= 21));
        Ok(())
    }
}
