use std::path::PathBuf;
use std::process::Command;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn main() {
    emit_build_metadata();

    let base = PathBuf::from("third_party/scintilla");
    let src = base.join("src");
    let win32 = base.join("win32");
    let lexilla = PathBuf::from("third_party/lexilla");
    let lexilla_include = lexilla.join("include");
    let lexilla_lexlib = lexilla.join("lexlib");
    let lexilla_lexers = lexilla.join("lexers");
    let lexilla_src = lexilla.join("src");

    let mut build = cc::Build::new();
    build.cpp(true);
    build.warnings(false);
    build.include(base.join("include"));
    build.include(&src);
    build.include(&win32);
    build.define("UNICODE", None);
    build.define("_UNICODE", None);

    let compiler = build.get_compiler();
    if compiler.is_like_msvc() {
        build.flag("/std:c++17");
        build.flag("/EHsc");
        build.flag("/utf-8");
    } else {
        build.flag("-std=c++17");
    }

    let src_files = [
        "AutoComplete.cxx",
        "CallTip.cxx",
        "CaseConvert.cxx",
        "CaseFolder.cxx",
        "CellBuffer.cxx",
        "ChangeHistory.cxx",
        "CharacterCategoryMap.cxx",
        "CharacterType.cxx",
        "CharClassify.cxx",
        "ContractionState.cxx",
        "DBCS.cxx",
        "Decoration.cxx",
        "Document.cxx",
        "EditModel.cxx",
        "Editor.cxx",
        "EditView.cxx",
        "Geometry.cxx",
        "Indicator.cxx",
        "KeyMap.cxx",
        "LineMarker.cxx",
        "MarginView.cxx",
        "PerLine.cxx",
        "PositionCache.cxx",
        "RESearch.cxx",
        "RunStyles.cxx",
        "ScintillaBase.cxx",
        "Selection.cxx",
        "Style.cxx",
        "UndoHistory.cxx",
        "UniConversion.cxx",
        "UniqueString.cxx",
        "ViewStyle.cxx",
        "XPM.cxx",
    ];

    for file in src_files {
        let path = src.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        build.file(path);
    }

    let win32_files = ["HanjaDic.cxx", "PlatWin.cxx", "ScintillaWin.cxx"];
    for file in win32_files {
        let path = win32.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        build.file(path);
    }

    build.compile("scintilla");

    let mut lex_build = cc::Build::new();
    lex_build.cpp(true);
    lex_build.warnings(false);
    lex_build.include(base.join("include"));
    lex_build.include(&lexilla_include);
    lex_build.include(&lexilla_lexlib);
    lex_build.include(&lexilla_lexers);
    lex_build.include(&lexilla_src);
    lex_build.define("UNICODE", None);
    lex_build.define("_UNICODE", None);

    let compiler = lex_build.get_compiler();
    if compiler.is_like_msvc() {
        lex_build.flag("/std:c++17");
        lex_build.flag("/EHsc");
        lex_build.flag("/utf-8");
    } else {
        lex_build.flag("-std=c++17");
    }

    let lexlib_files = [
        "Accessor.cxx",
        "CharacterCategory.cxx",
        "CharacterSet.cxx",
        "DefaultLexer.cxx",
        "InList.cxx",
        "LexAccessor.cxx",
        "LexerBase.cxx",
        "LexerModule.cxx",
        "LexerSimple.cxx",
        "PropSetSimple.cxx",
        "StyleContext.cxx",
        "WordList.cxx",
    ];

    for file in lexlib_files {
        let path = lexilla_lexlib.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        lex_build.file(path);
    }

    let lexer_files = [
        "LexCPP.cxx",
        "LexCSS.cxx",
        "LexHTML.cxx",
        "LexJSON.cxx",
        "LexNull.cxx",
        "LexPowerShell.cxx",
        "LexProps.cxx",
        "LexPython.cxx",
        "LexYAML.cxx",
    ];

    for file in lexer_files {
        let path = lexilla_lexers.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        lex_build.file(path);
    }

    let lexilla_entry = lexilla_src.join("LexillaMinimal.cxx");
    println!("cargo:rerun-if-changed={}", lexilla_entry.display());
    lex_build.file(lexilla_entry);

    lex_build.compile("lexilla");

    for lib in ["User32", "Gdi32", "Imm32", "Ole32", "OleAut32", "Advapi32"] {
        println!("cargo:rustc-link-lib={lib}");
    }

    #[cfg(windows)]
    {
        let icon = PathBuf::from("assets").join("rivet.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        if icon.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(icon.to_string_lossy().as_ref());
            res.compile().expect("Failed to compile Windows resources");
        }
    }
}

fn emit_build_metadata() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=RIVET_VERSION={version}");

    println!("cargo:rerun-if-changed=.git/HEAD");
    let git_sha = git_output(["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=RIVET_GIT_SHA={git_sha}");

    let build_utc = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=RIVET_BUILD_UTC={build_utc}");

    let source_url = std::env::var("CARGO_PKG_REPOSITORY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_output(["config", "--get", "remote.origin.url"]))
        .map(|value| normalize_git_url(&value))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=RIVET_SOURCE_URL={source_url}");
}

fn git_output<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_git_url(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
    {
        let path = path.strip_suffix(".git").unwrap_or(path);
        return format!("https://{host}/{path}");
    }
    if let Some(rest) = trimmed.strip_prefix("ssh://git@")
        && let Some((host, path)) = rest.split_once('/')
    {
        let path = path.strip_suffix(".git").unwrap_or(path);
        return format!("https://{host}/{path}");
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        let normalized = rest.strip_suffix(".git").unwrap_or(rest);
        return format!("https://{normalized}");
    }
    trimmed.to_string()
}
