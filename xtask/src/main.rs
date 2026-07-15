use std::{
    collections::HashSet,
    env,
    fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result};
use glob::Pattern;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct Config {
    scan: ScanConfig,
    #[serde(rename = "forbidden_term", default)]
    forbidden_terms: Vec<ForbiddenTerm>,
    #[serde(rename = "mandatory_term", default)]
    mandatory_terms: Vec<MandatoryTerm>,
}

#[derive(Debug, Deserialize)]
struct ScanConfig {
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ForbiddenTerm {
    term: String,
    message: String,
    replacement: Option<String>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MandatoryTerm {
    term: String,
    message: String,
    #[serde(default)]
    scope_files: Vec<String>,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Clone)]
struct Violation {
    path: PathBuf,
    line: usize,
    col: usize,
    message: String,
}

#[derive(Debug, Default)]
struct CodeFacts {
    configurator_token_duration: Option<String>,
    endpoint_token_duration: Option<String>,
    refresh_token_table: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("doc-lint error: {e:#}");
        process::exit(2);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let subcommand = args
        .next()
        .unwrap_or_else(|| "doc-lint".to_string());
    if subcommand != "doc-lint" {
        anyhow::bail!("unknown subcommand: {subcommand}. usage: cargo xtask doc-lint [paths...]");
    }
    let paths: Vec<String> = args.collect();

    let repo_root = env::current_dir()?;
    let config_path = repo_root.join(".opencode").join("doc-lint.toml");
    let config: Config = load_config(&config_path)?;

    let facts = extract_code_facts(&repo_root)?;

    let files = if paths.is_empty() {
        collect_files(&repo_root, &config.scan)?
    } else {
        collect_paths(&repo_root, &paths)?
    };

    let mut violations = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("reading {}", file.display()))?;
        check_file(&repo_root, file, &content, &config, &facts, &mut violations)?;
    }

    // Source-of-truth path checks only apply to AGENTS.md.
    let agents_md = repo_root.join("AGENTS.md");
    if files.contains(&agents_md) {
        let content = fs::read_to_string(&agents_md)
            .with_context(|| format!("reading {}", agents_md.display()))?;
        check_source_of_truth_paths(&repo_root, &content, &mut violations)?;
    }

    if violations.is_empty() {
        println!("doc-lint: no violations found");
        return Ok(());
    }

    violations.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });
    for v in &violations {
        let rel = v.path.strip_prefix(&repo_root).unwrap_or(&v.path);
        eprintln!("{}:{}:{}: {}", rel.display(), v.line, v.col, v.message);
    }
    eprintln!("\ndoc-lint: {} violation(s)", violations.len());
    process::exit(1);
}

fn load_config(path: &Path) -> Result<Config> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing config {}", path.display()))
}

fn collect_files(root: &Path, scan: &ScanConfig) -> Result<Vec<PathBuf>> {
    let include_patterns: Vec<Pattern> = scan
        .include
        .iter()
        .map(|p| Pattern::new(p).context("invalid include glob"))
        .collect::<Result<_>>()?;
    let exclude_patterns: Vec<Pattern> = scan
        .exclude
        .iter()
        .map(|p| Pattern::new(p).context("invalid exclude glob"))
        .collect::<Result<_>>()?;

    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let rel = path.strip_prefix(root).unwrap_or(path);
            let rel_str = rel.to_string_lossy();
            !exclude_patterns.iter().any(|p| p.matches(&rel_str))
        })
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();
        if include_patterns.iter().any(|p| p.matches(&rel_str)) {
            files.push(path.to_path_buf());
        }
    }
    Ok(files)
}

fn collect_paths(root: &Path, args: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = HashSet::new();
    for arg in args {
        let path = root.join(arg);
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            files.insert(path);
        } else if path.is_dir() {
            for entry in WalkDir::new(&path).into_iter() {
                let entry = entry?;
                if entry.file_type().is_file()
                    && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
                {
                    files.insert(entry.path().to_path_buf());
                }
            }
        } else {
            eprintln!("warning: path not found or not a markdown file: {}", path.display());
        }
    }
    Ok(files.into_iter().collect())
}

fn check_file(
    root: &Path,
    path: &Path,
    content: &str,
    config: &Config,
    facts: &CodeFacts,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    check_forbidden_terms(path, content, &config.forbidden_terms, violations)?;
    check_mandatory_terms(path, content, &config.mandatory_terms, violations)?;
    check_links(root, path, content, violations)?;
    check_code_facts(path, content, facts, violations)?;
    Ok(())
}

fn check_forbidden_terms(
    path: &Path,
    content: &str,
    terms: &[ForbiddenTerm],
    violations: &mut Vec<Violation>,
) -> Result<()> {
    for rule in terms {
        let re = Regex::new(&rule.term)
            .with_context(|| format!("invalid forbidden_term regex: {}", rule.term))?;
        for (line_no, line) in content.lines().enumerate() {
            if let Some(ref ctx) = rule.context {
                let ctx_re = Regex::new(&format!("(?i){}", regex::escape(ctx)))?;
                if !ctx_re.is_match(line) {
                    continue;
                }
            }
            for m in re.find_iter(line) {
                let matched = &line[m.start()..m.end()];
                let clean_line = line.replace('`', "");
                if clean_line.to_lowercase().contains(&format!("not {}", matched.to_lowercase())) {
                    continue;
                }
                if line[m.end()..].starts_with(".rs") {
                    continue;
                }
                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: line_no + 1,
                    col: m.start() + 1,
                    message: rule.message.clone(),
                });
            }
        }
    }
    Ok(())
}

fn check_mandatory_terms(
    path: &Path,
    content: &str,
    terms: &[MandatoryTerm],
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let rel = path.to_string_lossy();
    for rule in terms {
        if !rule.scope_files.is_empty() {
            let patterns: Vec<Pattern> = rule
                .scope_files
                .iter()
                .map(|p| Pattern::new(p).context("invalid scope_files glob"))
                .collect::<Result<_>>()?;
            if !patterns.iter().any(|p| p.matches(&rel) || p.matches(file_name)) {
                continue;
            }
        }

        let condition_re = rule
            .condition
            .as_ref()
            .map(|c| Regex::new(&format!("(?i){}", regex::escape(c))).unwrap());
        if let Some(ref re) = condition_re {
            if !re.is_match(content) {
                continue;
            }
        }

        if let Some(ref ctx) = rule.context {
            let ctx_re = Regex::new(&format!("(?i){}", regex::escape(ctx)))?;
            if !ctx_re.is_match(content) {
                continue;
            }
        }

        let term_re = Regex::new(&format!("(?i){}", regex::escape(&rule.term)))?;
        if !term_re.is_match(content) {
            violations.push(Violation {
                path: path.to_path_buf(),
                line: 1,
                col: 1,
                message: rule.message.clone(),
            });
        }
    }
    Ok(())
}

fn check_links(root: &Path, path: &Path, content: &str, violations: &mut Vec<Violation>) -> Result<()> {
    let parser = Parser::new(content);

    for (event, range) in parser.into_offset_iter() {
        let Event::Start(Tag::Link { dest_url, .. }, ..) = event else {
            continue;
        };
        let url = dest_url.as_ref();

        // External or special URLs.
        if url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("mailto:")
            || url.starts_with('#')
            || url.is_empty()
        {
            continue;
        }

        let (target_path, fragment) = if let Some(hash_pos) = url.find('#') {
            (&url[..hash_pos], Some(&url[hash_pos + 1..]))
        } else {
            (url, None)
        };

        let resolved = if target_path.is_empty() {
            path.to_path_buf()
        } else {
            let base = path.parent().unwrap_or(root);
            base.join(target_path)
        };

        let (line, col) = offset_to_line_col(content, range.start);
        if !resolved.exists() {
            violations.push(Violation {
                path: path.to_path_buf(),
                line,
                col,
                message: format!("broken link: {}", resolved.display()),
            });
            continue;
        }

        if let Some(fragment) = fragment {
            if !fragment.is_empty() {
                let target_content = fs::read_to_string(&resolved)
                    .with_context(|| format!("reading {}", resolved.display()))?;
                let target_headings = extract_headings(&target_content);
                if !target_headings.iter().any(|h| h == fragment) {
                    violations.push(Violation {
                        path: path.to_path_buf(),
                        line,
                        col,
                        message: format!("missing anchor: #{fragment}"),
                    });
                }
            }
        }
    }

    // Also validate same-file anchors.
    let same_file_anchor_re = Regex::new(r"\]\(#([^)]+)\)")?;
    for m in same_file_anchor_re.find_iter(content) {
        let fragment = &content[m.start() + 3..m.end() - 1];
        let headings = extract_headings(content);
        if !headings.iter().any(|h| h == fragment) {
            let (line, col) = offset_to_line_col(content, m.start());
            violations.push(Violation {
                path: path.to_path_buf(),
                line,
                col,
                message: format!("missing same-file anchor: #{fragment}"),
            });
        }
    }

    Ok(())
}

fn extract_headings(content: &str) -> HashSet<String> {
    let mut headings = HashSet::new();
    let parser = Parser::new(content);
    let mut current_heading = String::new();
    let mut in_heading = false;

    for (event, _range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }, ..) => {
                in_heading = true;
                current_heading.clear();
            }
            Event::End(TagEnd::Heading(..), ..) => {
                in_heading = false;
                headings.insert(anchor_slug(&current_heading));
                current_heading.clear();
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                current_heading.push_str(&text);
            }
            _ => {}
        }
    }
    headings
}

fn anchor_slug(text: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = true;
    for ch in text.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() || ch == ' ' || ch == '-' {
            let normalized = if ch == ' ' { '-' } else { ch };
            if normalized == '-' {
                if !prev_dash {
                    slug.push('-');
                    prev_dash = true;
                }
            } else {
                slug.push(normalized);
                prev_dash = false;
            }
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn check_source_of_truth_paths(
    root: &Path,
    content: &str,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let sections = [("## Source of Truth", 1_usize), ("## Task Index", 1_usize)];
    for (heading, col) in sections {
        let Some(start) = content.find(heading) else {
            continue;
        };
        let section = &content[start..];
        let section_end = section.find("\n## ").unwrap_or(section.len());
        let section = &section[..section_end];
        let code_re = Regex::new(r"`([^`]+)`")?;
        for m in code_re.find_iter(section) {
            let path_str = m.as_str().trim_matches('`');
            // Only check strings that look like file paths.
            if path_str.contains('/') || path_str.contains(".") {
                let resolved = root.join(path_str);
                if !resolved.exists() {
                    let (line, _) = offset_to_line_col(content, start + m.start());
                    violations.push(Violation {
                        path: root.join("AGENTS.md"),
                        line,
                        col,
                        message: format!("source-of-truth path does not exist: {path_str}"),
                    });
                }
            }
        }
    }
    Ok(())
}

fn extract_code_facts(root: &Path) -> Result<CodeFacts> {
    let mut facts = CodeFacts::default();

    let users_rs = fs::read_to_string(root.join("core/src/db/model/users.rs"))?;
    facts.configurator_token_duration =
        extract_duration(&users_rs, r"configurator_access")
            .or_else(|| extract_duration(&users_rs, r"(?s:.)"));

    let clients_rs = fs::read_to_string(root.join("core/src/db/model/clients.rs"))?;
    facts.endpoint_token_duration =
        extract_duration(&clients_rs, r"endpoint_access")
            .or_else(|| extract_duration(&clients_rs, r"(?s:.)"));

    let refresh_rs = fs::read_to_string(root.join("core/src/db/model/refresh_tokens.rs"))?;
    if let Some(caps) = Regex::new(r"DEFINE TABLE IF NOT EXISTS\s+(\w+)")
        .unwrap()
        .captures(&refresh_rs)
    {
        facts.refresh_token_table = Some(caps[1].to_string());
    }

    Ok(facts)
}

fn extract_duration(text: &str, context: &str) -> Option<String> {
    let pattern = format!(r"(?s){}.*?DURATION FOR TOKEN\s+(\S+);", context);
    let re = Regex::new(&pattern).ok()?;
    re.captures(text).map(|c| c[1].to_string())
}

fn check_code_facts(
    path: &Path,
    content: &str,
    facts: &CodeFacts,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let lower = content.to_lowercase();

    let tokenish = |s: &str| {
        s.contains("token") || s.contains("duration") || s.contains("expir")
    };

    if let Some(ref duration) = facts.configurator_token_duration {
        let needs_duration = content.lines().any(|line| {
            let lower_line = line.to_lowercase();
            (lower_line.contains("configurator_access") || lower_line.contains("configurator token"))
                && tokenish(&lower_line)
        });
        if needs_duration && !content.contains(duration) {
            violations.push(Violation {
                path: path.to_path_buf(),
                line: 1,
                col: 1,
                message: format!(
                    "configurator_access token duration mismatch: code has {duration}, docs must mention it"
                ),
            });
        }
    }

    if let Some(ref duration) = facts.endpoint_token_duration {
        let needs_duration = content.lines().any(|line| {
            let lower_line = line.to_lowercase();
            (lower_line.contains("endpoint_access") || lower_line.contains("endpoint token"))
                && tokenish(&lower_line)
        });
        if needs_duration && !content.contains(duration) {
            violations.push(Violation {
                path: path.to_path_buf(),
                line: 1,
                col: 1,
                message: format!(
                    "endpoint_access token duration mismatch: code has {duration}, docs must mention it"
                ),
            });
        }
    }

    if lower.contains("refresh_token") || lower.contains("refresh token") {
        if let Some(ref table) = facts.refresh_token_table {
            if !content.contains(table) {
                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: 1,
                    col: 1,
                    message: format!(
                        "refresh token table name mismatch: code uses {table}, docs must use that name"
                    ),
                });
            }
        }
    }

    Ok(())
}

fn offset_to_line_col(content: &str, offset: usize) -> (usize, usize) {
    let clamped = offset.min(content.len());
    let prefix = &content[..clamped];
    let line = prefix.matches('\n').count() + 1;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = clamped - last_newline + 1;
    (line, col)
}
