use include_dir::{Dir, include_dir};
use syntect::LoadingError;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxReference, SyntaxSet, SyntaxSetBuilder};
use syntect::util::LinesWithEndings;

use std::sync::OnceLock;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

static SYNTAX_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/syntax");

pub fn load_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(|| {
        let mut builder = SyntaxSetBuilder::new();
        builder
            .add_from_include_dir(&SYNTAX_DIR, true)
            .expect("Failed to load embedded syntaxes");
        builder.build()
    })
}

pub fn load_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(|| ThemeSet::load_defaults())
}

pub fn find_glsl_syntax<'a>(ps: &'a SyntaxSet) -> &'a SyntaxReference {
    ps.find_syntax_by_extension("glsl")
        .unwrap_or_else(|| ps.find_syntax_plain_text())
}

pub fn get_theme(name: &str) -> &'static Theme {
    let ts = load_theme_set();
    ts.themes
        .get(name)
        .unwrap_or_else(|| &ts.themes["InspiredGitHub"])
}

pub fn highlight_glsl_lines<'a>(
    code: &'a str,
    ps: &'a SyntaxSet,
    theme: &'a Theme,
) -> Vec<(syntect::highlighting::Style, &'a str)> {
    let syntax = find_glsl_syntax(ps);
    let mut h = HighlightLines::new(syntax, theme);

    let mut out = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ranges = h
            .highlight_line(line, ps)
            .expect("Failed to highlight line");
        out.extend(ranges.into_iter().map(|(style, text)| (style, text)));
    }

    out
}

pub trait SyntaxSetBuilderExt {
    fn add_from_include_dir(
        &mut self,
        dir: &Dir,
        lines_include_newline: bool,
    ) -> Result<(), LoadingError>;
}

impl SyntaxSetBuilderExt for SyntaxSetBuilder {
    fn add_from_include_dir(
        &mut self,
        dir: &Dir,
        lines_include_newline: bool,
    ) -> Result<(), LoadingError> {
        for file in dir.files() {
            if file
                .path()
                .extension()
                .map_or(false, |e| e == "sublime-syntax")
            {
                let content = file.contents_utf8().unwrap_or_default();

                let syntax =
                    SyntaxDefinition::load_from_str(content, lines_include_newline, None).unwrap();
                self.add(syntax);
            }
        }

        for sub in dir.dirs() {
            self.add_from_include_dir(sub, lines_include_newline)?;
        }

        Ok(())
    }
}
