use std::fmt::Write as _;

use thiserror::Error;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_script::{Script, UnicodeScript};

use crate::config::{AppConfig, StyleConfig};

const DEFAULT_MARGIN_L: u32 = 48;
const DEFAULT_MARGIN_R: u32 = 48;
const MIN_FONT_SIZE: u32 = 24;
const MAX_FONT_SIZE: u32 = 140;
const MIN_OUTLINE_SIZE: u32 = 4;
const MIN_SHADOW_SIZE: u32 = 2;
const EVENT_TEXT_INDEX: usize = 9;
const EVENT_MARGIN_LEFT_INDEX: usize = 5;
const EVENT_MARGIN_RIGHT_INDEX: usize = 6;

const OPENING_PUNCTUATION: &[char] = &[
    '(', '[', '{', '<', '（', '［', '｛', '〈', '《', '「', '『', '【', '〔', '“', '‘',
];
const PREFERRED_BREAK_PUNCTUATION: &[char] =
    &['，', '。', ',', '.', '!', '?', '！', '？', '；', '、'];
const FORCE_BREAK_PUNCTUATION: &[char] = &[
    ',', '.', ';', ':', '!', '?', ')', ']', '}', '/', '-', '–', '—', '，', '。', '．', '、', '；',
    '：', '！', '？', '》', '」', '』', '】', '〕', '）',
];

const FULLWIDTH_RANGES: &[(u32, u32)] = &[(0xff01, 0xff60), (0xffe0, 0xffe6)];

const WIDE_RANGES: &[(u32, u32)] = &[
    (0x1100, 0x115f),
    (0x231a, 0x231b),
    (0x2329, 0x232a),
    (0x23e9, 0x23ec),
    (0x23f0, 0x23f0),
    (0x23f3, 0x23f3),
    (0x25fd, 0x25fe),
    (0x2614, 0x2615),
    (0x2648, 0x2653),
    (0x267f, 0x267f),
    (0x2693, 0x2693),
    (0x26a1, 0x26a1),
    (0x26aa, 0x26ab),
    (0x26bd, 0x26be),
    (0x26c4, 0x26c5),
    (0x26ce, 0x26ce),
    (0x26d4, 0x26d4),
    (0x26ea, 0x26ea),
    (0x26f2, 0x26f3),
    (0x26f5, 0x26f5),
    (0x26fa, 0x26fa),
    (0x26fd, 0x26fd),
    (0x2705, 0x2705),
    (0x270a, 0x270b),
    (0x2728, 0x2728),
    (0x274c, 0x274c),
    (0x274e, 0x274e),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27b0, 0x27b0),
    (0x27bf, 0x27bf),
    (0x2b1b, 0x2b1c),
    (0x2b50, 0x2b50),
    (0x2b55, 0x2b55),
    (0x2e80, 0x2ffb),
    (0x3000, 0x303e),
    (0x3041, 0x33ff),
    (0x3400, 0x4dbf),
    (0x4e00, 0xa4c6),
    (0xa960, 0xa97c),
    (0xac00, 0xd7a3),
    (0xf900, 0xfaff),
    (0xfe10, 0xfe19),
    (0xfe30, 0xfe6b),
    (0x1f004, 0x1f004),
    (0x1f0cf, 0x1f0cf),
    (0x1f18e, 0x1f18e),
    (0x1f191, 0x1f19a),
    (0x1f200, 0x1f251),
    (0x1f300, 0x1f64f),
    (0x1f680, 0x1f6ff),
    (0x1f900, 0x1f9ff),
    (0x20000, 0x2fffd),
    (0x30000, 0x3fffd),
];

const AMBIGUOUS_RANGES: &[(u32, u32)] = &[
    (0x00a1, 0x00a1),
    (0x00a4, 0x00a4),
    (0x00a7, 0x00a8),
    (0x00aa, 0x00aa),
    (0x00ad, 0x00ae),
    (0x00b0, 0x00b4),
    (0x00b6, 0x00ba),
    (0x00bc, 0x00bf),
    (0x00c6, 0x00c6),
    (0x00d0, 0x00d0),
    (0x00d7, 0x00d8),
    (0x00de, 0x00e1),
    (0x00e6, 0x00e6),
    (0x00e8, 0x00ea),
    (0x00ec, 0x00ed),
    (0x00f0, 0x00f0),
    (0x00f2, 0x00f3),
    (0x00f7, 0x00fa),
    (0x00fc, 0x00fc),
    (0x00fe, 0x00fe),
    (0x0101, 0x0101),
    (0x0111, 0x0111),
    (0x0113, 0x0113),
    (0x011b, 0x011b),
    (0x0126, 0x0127),
    (0x012b, 0x012b),
    (0x0131, 0x0133),
    (0x0138, 0x0138),
    (0x013f, 0x0142),
    (0x0144, 0x0144),
    (0x0148, 0x014b),
    (0x014d, 0x014d),
    (0x0152, 0x0153),
    (0x0166, 0x0167),
    (0x016b, 0x016b),
    (0x01ce, 0x01ce),
    (0x01d0, 0x01d0),
    (0x01d2, 0x01d2),
    (0x01d4, 0x01d4),
    (0x01d6, 0x01d6),
    (0x01d8, 0x01d8),
    (0x01da, 0x01da),
    (0x01dc, 0x01dc),
    (0x0251, 0x0251),
    (0x0261, 0x0261),
    (0x02c4, 0x02c4),
    (0x02c7, 0x02c7),
    (0x02c9, 0x02cb),
    (0x02cd, 0x02cd),
    (0x02d0, 0x02d0),
    (0x02d8, 0x02db),
    (0x02dd, 0x02dd),
    (0x02df, 0x02df),
    (0x0300, 0x036f),
    (0x0391, 0x03a1),
    (0x03a3, 0x03a9),
    (0x03b1, 0x03c1),
    (0x03c3, 0x03c9),
    (0x0401, 0x0401),
    (0x0410, 0x044f),
    (0x0451, 0x0451),
    (0x2010, 0x2010),
    (0x2013, 0x2016),
    (0x2018, 0x2019),
    (0x201c, 0x201d),
    (0x2020, 0x2022),
    (0x2024, 0x2027),
    (0x2030, 0x2030),
    (0x2032, 0x2033),
    (0x2035, 0x2035),
    (0x203b, 0x203b),
    (0x203e, 0x203e),
    (0x2074, 0x2074),
    (0x207f, 0x207f),
    (0x2081, 0x2084),
    (0x20ac, 0x20ac),
    (0x2103, 0x2103),
    (0x2105, 0x2105),
    (0x2109, 0x2109),
    (0x2113, 0x2113),
    (0x2116, 0x2116),
    (0x2121, 0x2122),
    (0x2126, 0x2126),
    (0x212b, 0x212b),
    (0x2153, 0x2154),
    (0x215b, 0x215e),
    (0x2160, 0x216b),
    (0x2170, 0x2179),
    (0x2189, 0x2189),
    (0x2190, 0x2199),
    (0x21b8, 0x21b9),
    (0x21d2, 0x21d2),
    (0x21d4, 0x21d4),
    (0x21e7, 0x21e7),
    (0x2200, 0x2200),
    (0x2202, 0x2203),
    (0x2207, 0x2208),
    (0x220b, 0x220b),
    (0x220f, 0x220f),
    (0x2211, 0x2211),
    (0x2215, 0x2215),
    (0x221a, 0x221a),
    (0x221d, 0x2220),
    (0x2223, 0x2223),
    (0x2225, 0x2225),
    (0x2227, 0x222c),
    (0x222e, 0x222e),
    (0x2234, 0x2237),
    (0x223c, 0x223d),
    (0x2248, 0x2248),
    (0x224c, 0x224c),
    (0x2252, 0x2252),
    (0x2260, 0x2261),
    (0x2264, 0x2267),
    (0x226a, 0x226b),
    (0x226e, 0x226f),
    (0x2282, 0x2283),
    (0x2286, 0x2287),
    (0x2295, 0x2295),
    (0x2299, 0x2299),
    (0x22a5, 0x22a5),
    (0x22bf, 0x22bf),
    (0x2312, 0x2312),
    (0x2460, 0x24e9),
    (0x24eb, 0x254b),
    (0x2550, 0x2573),
    (0x2580, 0x258f),
    (0x2592, 0x2595),
    (0x25a0, 0x25a1),
    (0x25a3, 0x25a9),
    (0x25b2, 0x25b3),
    (0x25b6, 0x25b7),
    (0x25bc, 0x25bd),
    (0x25c0, 0x25c1),
    (0x25c6, 0x25c8),
    (0x25cb, 0x25cb),
    (0x25ce, 0x25d1),
    (0x25e2, 0x25e5),
    (0x25ef, 0x25ef),
    (0x2605, 0x2606),
    (0x2609, 0x2609),
    (0x260e, 0x260f),
    (0x261c, 0x261c),
    (0x261e, 0x261e),
    (0x2640, 0x2640),
    (0x2642, 0x2642),
    (0x2660, 0x2661),
    (0x2663, 0x2665),
    (0x2667, 0x266a),
    (0x266c, 0x266d),
    (0x266f, 0x266f),
    (0x269e, 0x269f),
    (0x26bf, 0x26bf),
    (0x26c6, 0x26cd),
    (0x26cf, 0x26d3),
    (0x26d5, 0x26e1),
    (0x26e3, 0x26e3),
    (0x26e8, 0x26e9),
    (0x26eb, 0x26f1),
    (0x26f4, 0x26f4),
    (0x26f6, 0x26f9),
    (0x26fb, 0x26fc),
    (0x26fe, 0x26ff),
    (0x273d, 0x273d),
    (0x2776, 0x277f),
    (0xe000, 0xf8ff),
    (0xfe00, 0xfe0f),
    (0xfffd, 0xfffd),
    (0x1f100, 0x1f10a),
    (0x1f110, 0x1f12d),
    (0x1f130, 0x1f169),
    (0x1f170, 0x1f18d),
    (0x1f18f, 0x1f190),
    (0x1f19b, 0x1f1ac),
    (0xe0100, 0xe01ef),
    (0xf0000, 0xffffd),
    (0x100000, 0x10fffd),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubtitlePlan {
    pub(crate) requires_ass_staging: bool,
    pub(crate) wraps_dialogue_lines: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubtitleLayout {
    pub(crate) font_size: u32,
    pub(crate) margin_l: u32,
    pub(crate) margin_r: u32,
    pub(crate) margin_v: u32,
    pub(crate) outline_size: u32,
    pub(crate) shadow_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Tag,
    Newline,
    HardSpace,
    Text,
    Space,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    raw: String,
    width: f64,
    visible: bool,
    break_char: Option<char>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EastAsianWidth {
    Fullwidth,
    Wide,
    Narrow,
    Ambiguous,
    Neutral,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum SubtitleError {
    #[error("converted ASS is missing an [Events] section")]
    MissingEventsSection,
    #[error("converted ASS contains a malformed dialogue line: {line}")]
    MalformedDialogue { line: String },
}

pub(crate) fn plan(_config: &AppConfig) -> SubtitlePlan {
    SubtitlePlan {
        requires_ass_staging: true,
        wraps_dialogue_lines: true,
    }
}

pub(crate) fn resolve_layout(style: &StyleConfig, video_height: u32) -> SubtitleLayout {
    let font_size = style
        .font_size
        .unwrap_or_else(|| ((video_height * 70) / 1080).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE));

    let margin_v = style.margin_v.unwrap_or_else(|| {
        let computed = (video_height * 80) / 3000;
        computed.max(font_size / 2)
    });

    let outline_size = style
        .outline_size
        .unwrap_or_else(|| ((font_size * 8) / 100).max(MIN_OUTLINE_SIZE));

    let shadow_size = style
        .shadow_size
        .unwrap_or_else(|| (outline_size / 2).max(MIN_SHADOW_SIZE));

    SubtitleLayout {
        font_size,
        margin_l: DEFAULT_MARGIN_L,
        margin_r: DEFAULT_MARGIN_R,
        margin_v,
        outline_size,
        shadow_size,
    }
}

pub(crate) fn render_ass(
    raw_ass: &str,
    style: &StyleConfig,
    video_width: u32,
    video_height: u32,
) -> Result<String, SubtitleError> {
    let layout = resolve_layout(style, video_height);
    let normalized = raw_ass.replace("\r\n", "\n").replace('\r', "\n");
    let dialogue_lines = extract_dialogue_lines(&normalized)?;

    let mut output = String::new();
    writeln!(output, "[Script Info]").expect("writing String should not fail");
    writeln!(output, "ScriptType: v4.00+").expect("writing String should not fail");
    writeln!(output, "WrapStyle: 0").expect("writing String should not fail");
    writeln!(output, "ScaledBorderAndShadow: yes").expect("writing String should not fail");
    writeln!(output, "PlayResX: {video_width}").expect("writing String should not fail");
    writeln!(output, "PlayResY: {video_height}").expect("writing String should not fail");
    writeln!(output).expect("writing String should not fail");
    writeln!(output, "[V4+ Styles]").expect("writing String should not fail");
    writeln!(output, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").expect("writing String should not fail");
    writeln!(
        output,
        "Style: Default,{},{},&H0000FFFF,&H0000FFFF,&H00000000,&H64000000,0,0,0,0,100,100,0,0,1,{},{},2,{},{},{},1",
        style.font_name,
        layout.font_size,
        layout.outline_size,
        layout.shadow_size,
        layout.margin_l,
        layout.margin_r,
        layout.margin_v,
    )
    .expect("writing String should not fail");
    writeln!(output).expect("writing String should not fail");
    writeln!(output, "[Events]").expect("writing String should not fail");
    writeln!(
        output,
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"
    )
    .expect("writing String should not fail");

    for line in dialogue_lines {
        writeln!(
            output,
            "{}",
            wrap_dialogue_line(line, video_width, &layout)?
        )
        .expect("writing String should not fail");
    }

    Ok(output)
}

fn extract_dialogue_lines(raw_ass: &str) -> Result<Vec<&str>, SubtitleError> {
    let mut in_events = false;
    let mut saw_events = false;
    let mut dialogue_lines = Vec::new();

    for line in raw_ass.lines() {
        if line.trim() == "[Events]" {
            in_events = true;
            saw_events = true;
            continue;
        }

        if in_events && line.starts_with("Dialogue:") {
            dialogue_lines.push(line);
        }
    }

    if !saw_events {
        return Err(SubtitleError::MissingEventsSection);
    }

    Ok(dialogue_lines)
}

fn wrap_dialogue_line(
    line: &str,
    video_width: u32,
    layout: &SubtitleLayout,
) -> Result<String, SubtitleError> {
    let Some(dialogue_body) = line.strip_prefix("Dialogue:") else {
        return Err(SubtitleError::MalformedDialogue {
            line: line.to_owned(),
        });
    };

    let mut fields = split_dialogue_fields(dialogue_body, EVENT_TEXT_INDEX);
    if fields.len() <= EVENT_TEXT_INDEX {
        return Err(SubtitleError::MalformedDialogue {
            line: line.to_owned(),
        });
    }

    let margin_left = fields
        .get(EVENT_MARGIN_LEFT_INDEX)
        .map(|value| parse_positive_u32(value, layout.margin_l))
        .unwrap_or(layout.margin_l);
    let margin_right = fields
        .get(EVENT_MARGIN_RIGHT_INDEX)
        .map(|value| parse_positive_u32(value, layout.margin_r))
        .unwrap_or(layout.margin_r);
    let max_units = compute_max_units(
        video_width,
        margin_left,
        margin_right,
        layout.font_size,
        layout.outline_size,
        layout.shadow_size,
    );

    fields[EVENT_TEXT_INDEX] = wrap_ass_text(&fields[EVENT_TEXT_INDEX], max_units);
    Ok(format!("Dialogue:{}", fields.join(",")))
}

fn split_dialogue_fields(dialogue_body: &str, text_index: usize) -> Vec<String> {
    let mut fields = Vec::with_capacity(text_index + 1);
    let mut start = 0;
    let mut splits = 0;

    for (index, current) in dialogue_body.char_indices() {
        if current == ',' && splits < text_index {
            fields.push(dialogue_body[start..index].to_owned());
            start = index + current.len_utf8();
            splits += 1;
        }
    }

    fields.push(dialogue_body[start..].to_owned());
    fields
}

fn parse_positive_u32(value: &str, fallback: u32) -> u32 {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|parsed| *parsed > 0)
        .unwrap_or(fallback)
}

fn wrap_ass_text(text: &str, max_units: f64) -> String {
    let tokens = tokenize_ass_text(text);
    let mut wrapped = String::new();
    let mut segment = Vec::new();

    for token in tokens {
        if token.kind == TokenKind::Newline {
            wrapped.push_str(&wrap_segment(&segment, max_units));
            wrapped.push_str(&token.raw);
            segment.clear();
        } else {
            segment.push(token);
        }
    }

    wrapped.push_str(&wrap_segment(&segment, max_units));
    wrapped
}

fn tokenize_ass_text(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < text.len() {
        let remainder = &text[index..];

        if remainder.starts_with('{')
            && let Some(tag_end) = remainder.find('}')
        {
            tokens.push(Token {
                kind: TokenKind::Tag,
                raw: remainder[..=tag_end].to_owned(),
                width: 0.0,
                visible: false,
                break_char: None,
            });
            index += tag_end + 1;
            continue;
        }

        if remainder.starts_with("\\N") || remainder.starts_with("\\n") {
            tokens.push(Token {
                kind: TokenKind::Newline,
                raw: remainder[..2].to_owned(),
                width: 0.0,
                visible: false,
                break_char: None,
            });
            index += 2;
            continue;
        }

        if remainder.starts_with("\\h") {
            tokens.push(Token {
                kind: TokenKind::HardSpace,
                raw: "\\h".to_owned(),
                width: 0.35,
                visible: false,
                break_char: None,
            });
            index += 2;
            continue;
        }

        if remainder.starts_with('\\') && remainder.len() > 1 {
            let mut chars = remainder[1..].chars();
            let literal = chars.next().expect("remainder is non-empty");
            tokens.push(Token {
                kind: TokenKind::Text,
                raw: format!("\\{literal}"),
                width: estimate_char_width(literal),
                visible: !literal.is_whitespace(),
                break_char: Some(literal),
            });
            index += 1 + literal.len_utf8();
            continue;
        }

        let value = remainder.chars().next().expect("remainder is non-empty");
        let kind = if value.is_whitespace() {
            TokenKind::Space
        } else {
            TokenKind::Text
        };
        let is_text = kind == TokenKind::Text;
        tokens.push(Token {
            kind,
            raw: value.to_string(),
            width: estimate_char_width(value),
            visible: is_text,
            break_char: is_text.then_some(value),
        });
        index += value.len_utf8();
    }

    tokens
}

fn wrap_segment(tokens: &[Token], max_units: f64) -> String {
    if tokens.is_empty() || max_units <= 0.0 {
        return tokens.iter().map(|token| token.raw.as_str()).collect();
    }

    let mut wrapped = String::new();
    let mut start = 0;

    while start < tokens.len() {
        let mut width = 0.0;
        let mut preferred_break = None;
        let mut force_break = None;
        let mut index = start;

        while index < tokens.len() {
            let token = &tokens[index];
            width += token.width;

            match token.kind {
                TokenKind::Space => {
                    if has_visible_text(tokens, start, index) {
                        force_break = Some((index, index + 1));
                    }
                }
                TokenKind::Text => {
                    if let Some(current) = token.break_char {
                        if is_preferred_break_char(current) {
                            preferred_break = Some((index + 1, index + 1));
                            force_break = Some((index + 1, index + 1));
                        } else if is_force_break_char(current) || is_cjk_force_break_char(current) {
                            force_break = Some((index + 1, index + 1));
                        }
                    }
                }
                TokenKind::Tag | TokenKind::Newline | TokenKind::HardSpace => {}
            }

            if width > max_units && has_visible_text(tokens, start, index + 1) {
                let (mut split_at, mut next_start) = if let Some(breakpoint) =
                    preferred_break.filter(|(split_at, _)| *split_at > start)
                {
                    breakpoint
                } else if let Some(breakpoint) =
                    force_break.filter(|(split_at, _)| *split_at > start)
                {
                    breakpoint
                } else {
                    let split_at = if has_visible_text(tokens, start, index) {
                        index
                    } else {
                        index + 1
                    };
                    let split_at = if split_at <= start {
                        index + 1
                    } else {
                        split_at
                    };
                    (split_at, split_at)
                };

                (split_at, next_start) =
                    keep_punctuation_with_previous(tokens, split_at, next_start);
                for token in &tokens[start..split_at] {
                    wrapped.push_str(&token.raw);
                }
                wrapped.push_str("\\N");
                start = next_start;
                break;
            }

            index += 1;
        }

        if index >= tokens.len() {
            for token in &tokens[start..] {
                wrapped.push_str(&token.raw);
            }
            break;
        }
    }

    wrapped
}

fn has_visible_text(tokens: &[Token], start: usize, end: usize) -> bool {
    tokens[start..end].iter().any(|token| token.visible)
}

fn skip_leading_spaces(tokens: &[Token], mut start: usize) -> usize {
    while start < tokens.len() && tokens[start].kind == TokenKind::Space {
        start += 1;
    }
    start
}

fn keep_punctuation_with_previous(
    tokens: &[Token],
    mut split_at: usize,
    mut next_start: usize,
) -> (usize, usize) {
    next_start = skip_leading_spaces(tokens, next_start);

    while next_start < tokens.len()
        && tokens[next_start].kind == TokenKind::Text
        && tokens[next_start]
            .break_char
            .map(is_force_break_char)
            .unwrap_or(false)
    {
        split_at = next_start + 1;
        next_start = skip_leading_spaces(tokens, split_at);
    }

    (split_at, next_start)
}

fn compute_max_units(
    video_width: u32,
    margin_left: u32,
    margin_right: u32,
    font_size: u32,
    outline_size: u32,
    shadow_size: u32,
) -> f64 {
    let side_padding = (font_size as f64 * 0.75)
        .max((outline_size + shadow_size) as f64 * 6.0)
        .max(24.0);
    let usable_width =
        (video_width as f64 - margin_left as f64 - margin_right as f64 - side_padding * 2.0)
            .max(video_width as f64 * 0.45);
    let fullwidth_pixels = (font_size as f64 * 0.95).max(1.0);

    (usable_width / fullwidth_pixels).clamp(8.0, 60.0)
}

fn estimate_char_width(ch: char) -> f64 {
    match get_general_category(ch) {
        GeneralCategory::NonspacingMark
        | GeneralCategory::SpacingMark
        | GeneralCategory::EnclosingMark
        | GeneralCategory::Format => return 0.0,
        _ => {}
    }

    if matches!(ch, ' ' | '\t') {
        return 0.35;
    }

    if ch as u32 >= 0x1f300 {
        return 2.0;
    }

    let width_class = east_asian_width(ch);
    if matches!(
        width_class,
        EastAsianWidth::Fullwidth | EastAsianWidth::Wide
    ) {
        return 1.0;
    }

    match get_general_category(ch) {
        GeneralCategory::DecimalNumber => 0.56,
        GeneralCategory::ClosePunctuation
        | GeneralCategory::ConnectorPunctuation
        | GeneralCategory::DashPunctuation
        | GeneralCategory::FinalPunctuation
        | GeneralCategory::InitialPunctuation
        | GeneralCategory::OpenPunctuation
        | GeneralCategory::OtherPunctuation
            if width_class == EastAsianWidth::Narrow =>
        {
            0.45
        }
        GeneralCategory::ClosePunctuation
        | GeneralCategory::ConnectorPunctuation
        | GeneralCategory::DashPunctuation
        | GeneralCategory::FinalPunctuation
        | GeneralCategory::InitialPunctuation
        | GeneralCategory::OpenPunctuation
        | GeneralCategory::OtherPunctuation => 0.6,
        GeneralCategory::UppercaseLetter
        | GeneralCategory::LowercaseLetter
        | GeneralCategory::TitlecaseLetter
        | GeneralCategory::ModifierLetter
        | GeneralCategory::OtherLetter
            if ch.script() == Script::Latin =>
        {
            if ch.is_uppercase() {
                0.62
            } else {
                0.52
            }
        }
        GeneralCategory::UppercaseLetter
        | GeneralCategory::LowercaseLetter
        | GeneralCategory::TitlecaseLetter
        | GeneralCategory::ModifierLetter
        | GeneralCategory::OtherLetter => 0.68,
        _ if width_class == EastAsianWidth::Ambiguous => 0.8,
        _ => 0.6,
    }
}

fn east_asian_width(ch: char) -> EastAsianWidth {
    let code_point = ch as u32;
    if in_ranges(code_point, FULLWIDTH_RANGES) {
        EastAsianWidth::Fullwidth
    } else if in_ranges(code_point, WIDE_RANGES) {
        EastAsianWidth::Wide
    } else if (0x20..=0x7e).contains(&code_point) || (0xff61..=0xff9f).contains(&code_point) {
        EastAsianWidth::Narrow
    } else if in_ranges(code_point, AMBIGUOUS_RANGES) {
        EastAsianWidth::Ambiguous
    } else {
        EastAsianWidth::Neutral
    }
}

fn in_ranges(code_point: u32, ranges: &[(u32, u32)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| (*start..=*end).contains(&code_point))
}

fn is_preferred_break_char(ch: char) -> bool {
    !OPENING_PUNCTUATION.contains(&ch) && PREFERRED_BREAK_PUNCTUATION.contains(&ch)
}

fn is_force_break_char(ch: char) -> bool {
    !OPENING_PUNCTUATION.contains(&ch) && FORCE_BREAK_PUNCTUATION.contains(&ch)
}

fn is_cjk_force_break_char(ch: char) -> bool {
    !OPENING_PUNCTUATION.contains(&ch)
        && matches!(
            get_general_category(ch),
            GeneralCategory::UppercaseLetter
                | GeneralCategory::LowercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::OtherLetter
                | GeneralCategory::DecimalNumber
                | GeneralCategory::LetterNumber
                | GeneralCategory::OtherNumber
        )
        && matches!(
            east_asian_width(ch),
            EastAsianWidth::Fullwidth | EastAsianWidth::Wide
        )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn style_config() -> StyleConfig {
        StyleConfig {
            font_name: "LINE Seed TW_OTF Regular".to_owned(),
            font_dir: PathBuf::from("/Library/Fonts"),
            font_size: None,
            margin_v: None,
            outline_size: None,
            shadow_size: None,
        }
    }

    #[test]
    fn plan_requires_ass_staging_and_wrapping() {
        let config = AppConfig {
            video: PathBuf::from("video.mp4"),
            subtitle: PathBuf::from("subtitle.vtt"),
            output: PathBuf::from("output.mp4"),
            overwrite_output: false,
            open_output: false,
            style: style_config(),
        };

        let plan = super::plan(&config);

        assert!(plan.requires_ass_staging);
        assert!(plan.wraps_dialogue_lines);
    }

    #[test]
    fn resolve_layout_scales_defaults_from_video_height() {
        let layout = resolve_layout(&style_config(), 1080);

        assert_eq!(layout.font_size, 70);
        assert_eq!(layout.margin_l, 48);
        assert_eq!(layout.margin_r, 48);
        assert_eq!(layout.margin_v, 35);
        assert_eq!(layout.outline_size, 5);
        assert_eq!(layout.shadow_size, 2);
    }

    #[test]
    fn render_ass_builds_header_and_wraps_dialogue_text() {
        let raw_ass = "[Script Info]\n[Events]\nDialogue: 0,0:00:00.00,0:00:03.00,Default,,0,0,0,,這是一段很長的中文字幕需要自動換行讓畫面更容易閱讀並且保持自然節奏。\n";

        let rendered =
            render_ass(raw_ass, &style_config(), 640, 360).expect("rendered ASS should build");

        assert!(rendered.contains("PlayResX: 640"));
        assert!(rendered.contains("PlayResY: 360"));
        assert!(rendered.contains("Style: Default,LINE Seed TW_OTF Regular,24"));
        assert!(rendered.contains("Dialogue: 0,0:00:00.00,0:00:03.00,Default,,0,0,0,,"));
        assert!(rendered.matches("\\N").count() >= 1);
    }

    #[test]
    fn render_ass_keeps_override_tags_and_existing_breaks() {
        let raw_ass = "[Events]\nDialogue: 0,0:00:00.00,0:00:03.00,Default,,0,0,0,,{\\i1}這是一段測試文字會保留標籤\\N並且在需要時再換行。\n";

        let rendered =
            render_ass(raw_ass, &style_config(), 640, 360).expect("rendered ASS should build");

        assert!(rendered.contains("{\\i1}這是一段測試文字會保留標籤\\N並且在需要時再換行。"));
    }

    #[test]
    fn render_ass_rejects_missing_events_section() {
        let error = render_ass("[Script Info]\n", &style_config(), 640, 360)
            .expect_err("missing events should fail");

        assert_eq!(error, SubtitleError::MissingEventsSection);
    }

    #[test]
    fn wrap_segment_breaks_after_preferred_punctuation() {
        let wrapped = wrap_ass_text("第一句，第二句，第三句，第四句", 6.0);

        assert_eq!(wrapped, "第一句，\\N第二句，\\N第三句，\\N第四句");
    }

    #[test]
    fn parse_positive_u32_falls_back_for_zero_or_invalid_values() {
        assert_eq!(parse_positive_u32("18", 4), 18);
        assert_eq!(parse_positive_u32("0", 4), 4);
        assert_eq!(parse_positive_u32("abc", 4), 4);
    }
}
