use std::{fs, path::Path};

fn source_files(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("read crate directory") {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            source_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
fn violations(line: &str) -> bool {
    let forbidden = [
        ["rgb", "("].concat(),
        ["rgba", "("].concat(),
        ["hsla", "("].concat(),
        ["Hsla", " {"].concat(),
    ];
    if forbidden.iter().any(|pattern| line.contains(pattern)) {
        return true;
    }
    for (index, _) in line.match_indices("0x") {
        let digits = line[index + 2..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count();
        if digits == 6 || digits == 8 {
            return true;
        }
    }
    for (index, name) in line.match_indices("font_family") {
        let suffix = line[index + name.len()..].trim_start();
        if suffix
            .strip_prefix('(')
            .is_some_and(|s| s.trim_start().starts_with('"'))
        {
            return true;
        }
    }
    false
}
#[test]
fn application_code_uses_only_topo_colors_and_font_helpers() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory");
    let mut files = Vec::new();
    source_files(crates, &mut files);
    let own_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let build_script = Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs");
    let mut failures = Vec::new();
    for file in files {
        if file.starts_with(&own_src) || file == build_script {
            continue;
        }
        let source = fs::read_to_string(&file).expect("read Rust source");
        for (index, line) in source.lines().enumerate() {
            if violations(line) {
                failures.push(format!("{}:{}: {line}", file.display(), index + 1));
            }
        }
        // Also inspect multiline calls, where the quoted family follows a newline.
        for (offset, name) in source.match_indices("font_family") {
            let tail = &source[offset + name.len()..];
            let suffix = tail.trim_start();
            if suffix.strip_prefix('(').is_some_and(|s| {
                s.trim_start().starts_with('"')
                    && (tail[..tail.len() - suffix.len()].contains('\n')
                        || s[..s.len() - s.trim_start().len()].contains('\n'))
            }) {
                let line = source[..offset]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    + 1;
                failures.push(format!("{}:{line}: literal font family", file.display()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Topo design guard violations:\n{}",
        failures.join("\n")
    );
}
#[test]
fn guard_detects_colors_and_literal_font_families() {
    for fragment in [
        ["rgb", "(1)"].concat(),
        ["rgba", "(1)"].concat(),
        ["hsla", "(1)"].concat(),
        ["Hsla", " { a: 1. }"].concat(),
        ["0x", "abcdef"].concat(),
        ["0x", "abcdef12"].concat(),
        ["font_family", "(\"other\")"].concat(),
    ] {
        assert!(violations(&fragment), "missed {fragment}");
    }
    assert!(!violations("theme.header_ink"));
}
