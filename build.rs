//! Prepares the guide pages in `src/docs` for rustdoc.
//!
//! The pages link to each other with ordinary relative links, such as
//! `[Layout](../concepts/layout.md)`, so the links work in editors and on
//! GitHub. Rustdoc can't follow links to files, so this writes a copy of every
//! page with those links turned into rustdoc paths, such as
//! `crate::guide::concepts::layout`, and `src/guide.rs` includes the copies.

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn main() {
    let docs = Path::new("src/docs");
    println!("cargo:rerun-if-changed=src/docs");

    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("docs");
    for page in markdown_files(docs) {
        let relative = page.strip_prefix(docs).expect("pages are inside src/docs");
        let source = fs::read_to_string(&page)
            .unwrap_or_else(|err| panic!("reading {}: {err}", page.display()));

        let target = out.join(relative);
        fs::create_dir_all(target.parent().expect("pages are in a folder"))
            .unwrap_or_else(|err| panic!("creating {}: {err}", target.display()));
        fs::write(&target, rewrite_links(&source, relative, docs))
            .unwrap_or_else(|err| panic!("writing {}: {err}", target.display()));
    }
}

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()));

    let mut files = Vec::new();
    for entry in entries {
        let path = entry.expect("readable directory entry").path();
        if path.is_dir() {
            files.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    files
}

// Rewrites links to other pages, leaving code blocks untouched.
fn rewrite_links(source: &str, page: &Path, docs: &Path) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_code_block = false;

    for line in source.split_inclusive('\n') {
        let fence = line.trim_start().starts_with("```");
        if fence {
            in_code_block = !in_code_block;
        }
        if fence || in_code_block {
            out.push_str(line);
        } else {
            out.push_str(&rewrite_line(line, page, docs));
        }
    }
    out
}

fn rewrite_line(line: &str, page: &Path, docs: &Path) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(open) = rest.find("](") {
        let (before, after) = rest.split_at(open + 2);
        out.push_str(before);

        let Some(close) = after.find(')') else {
            rest = after;
            break;
        };
        let target = &after[..close];
        match rustdoc_path(target, page, docs) {
            Some(path) => out.push_str(&path),
            None => out.push_str(target),
        }
        rest = &after[close..];
    }

    out.push_str(rest);
    out
}

// The rustdoc path for a link to another guide page, or `None` for any other
// kind of link.
fn rustdoc_path(target: &str, page: &Path, docs: &Path) -> Option<String> {
    let (file, fragment) = match target.split_once('#') {
        Some((file, fragment)) => (file, Some(fragment)),
        None => (target, None),
    };
    if !file.ends_with(".md") || file.contains("://") || file.starts_with('/') {
        return None;
    }

    let linked = normalize(&page.parent().unwrap_or(Path::new("")).join(file));
    assert!(
        docs.join(&linked).is_file(),
        "src/docs/{} links to {target}, which doesn't exist",
        page.display()
    );

    // `index.md` is its folder's page; any other file is a page inside it.
    let parts: Vec<String> = linked
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut path = String::from("crate::guide");
    for (i, part) in parts.iter().enumerate() {
        let name = if i + 1 == parts.len() {
            part.trim_end_matches(".md")
        } else {
            part.as_str()
        };
        if i + 1 == parts.len() && name == "index" {
            break;
        }
        path.push_str("::");
        path.push_str(&name.replace('-', "_"));
    }

    if let Some(fragment) = fragment {
        path.push('#');
        path.push_str(fragment);
    }
    Some(path)
}

// Resolves `.` and `..` in a path relative to `src/docs`.
fn normalize(path: &Path) -> PathBuf {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                assert!(
                    parts.pop().is_some(),
                    "a guide link leaves src/docs: {}",
                    path.display()
                );
            }
            other => parts.push(other.as_os_str().to_owned()),
        }
    }
    parts.iter().collect()
}
