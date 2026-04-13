use base64::prelude::*;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EmbeddedImage {
    pub bytes: Vec<u8>,
    pub is_svg: bool,
    pub is_template: bool,
}

#[derive(Debug, Clone)]
pub struct ItemParams {
    pub href: Option<String>,
    pub shell: Option<String>,
    pub refresh: bool,
    pub terminal: bool,
    pub dropdown: bool,
    pub alternate: bool,
    pub disabled: bool,
    pub trim: bool,
    pub color: Option<String>,
    pub image: Option<EmbeddedImage>,
    pub params: Vec<String>,
}

impl Default for ItemParams {
    fn default() -> Self {
        Self {
            href: None,
            shell: None,
            refresh: false,
            terminal: false,
            dropdown: true,
            alternate: false,
            disabled: false,
            trim: true,
            color: None,
            image: None,
            params: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MenuEntry {
    pub level: usize,
    pub text: String,
    pub params: ItemParams,
    pub separator: bool,
    pub alternate: Option<Box<MenuEntry>>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedPlugin {
    pub title: String,
    pub title_params: ItemParams,
    pub cycle_items: Vec<String>,
    pub menu_entries: Vec<MenuEntry>,
}

pub fn parse_plugin_output(output: &str) -> ParsedPlugin {
    let mut cycle_items = Vec::new();
    let mut menu_entries = Vec::new();
    let mut in_menu = false;

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');

        if line.trim().is_empty() {
            continue;
        }

        if !in_menu && line.trim() == "---" {
            in_menu = true;
            continue;
        }

        let trimmed_start = line.trim_start();
        let (prefix, is_separator) = parse_separator(trimmed_start);
        if is_separator {
            if in_menu {
                menu_entries.push(MenuEntry {
                    level: prefix.len() / 2,
                    text: String::new(),
                    params: ItemParams::default(),
                    separator: true,
                    alternate: None,
                });
            }
            continue;
        }

        let level = nesting_level(trimmed_start);
        let content = if trimmed_start.starts_with("--") || trimmed_start.starts_with("---") {
            &trimmed_start[level * 2..]
        } else {
            line
        };

        let (text, params) = parse_params(content);
        if in_menu {
            menu_entries.push(MenuEntry {
                level,
                text: if params.trim {
                    text.trim().to_owned()
                } else {
                    text.to_owned()
                },
                params,
                separator: false,
                alternate: None,
            });
        } else {
            cycle_items.push(if params.trim {
                text.trim().to_owned()
            } else {
                text.to_owned()
            });
        }
    }

    menu_entries = normalize_menu_entries(menu_entries);

    let (title, title_params) = if let Some(first_line) = output
        .lines()
        .map(|raw_line| raw_line.trim_end_matches('\r'))
        .find(|line| !line.trim().is_empty() && line.trim() != "---")
    {
        let (text, params) = parse_params(first_line);
        (
            if params.trim {
                text.trim().to_owned()
            } else {
                text.to_owned()
            },
            params,
        )
    } else {
        (String::new(), ItemParams::default())
    };

    ParsedPlugin {
        title,
        title_params,
        cycle_items,
        menu_entries,
    }
}

pub fn parse_refresh_interval(name: &str) -> Duration {
    let base = name.rsplit('/').next().unwrap_or(name);
    let candidates = base.split('.').collect::<Vec<_>>();

    for candidate in candidates.iter().rev().skip(1) {
        if let Some(duration) = parse_duration_token(candidate) {
            return duration;
        }
    }

    Duration::from_secs(60)
}

fn parse_duration_token(token: &str) -> Option<Duration> {
    let split_at = token.find(|c: char| !c.is_ascii_digit())?;
    let (value, unit) = token.split_at(split_at);
    let value = value.parse::<u64>().ok()?;

    match unit {
        "s" => Some(Duration::from_secs(value)),
        "m" => Some(Duration::from_secs(value * 60)),
        "h" => Some(Duration::from_secs(value * 60 * 60)),
        "d" => Some(Duration::from_secs(value * 60 * 60 * 24)),
        _ => None,
    }
}

fn nesting_level(line: &str) -> usize {
    let mut level = 0;
    let mut rest = line;
    while let Some(next) = rest.strip_prefix("--") {
        level += 1;
        rest = next;
    }
    level
}

fn parse_separator(line: &str) -> (&str, bool) {
    let trimmed = line.trim();
    let mut rest = trimmed;
    let mut nesting_len = 0;

    while rest.len() > 3 {
        let Some(next) = rest.strip_prefix("--") else {
            break;
        };
        nesting_len += 2;
        rest = next;
    }

    if rest == "---" {
        (&trimmed[..nesting_len], true)
    } else {
        (line, false)
    }
}

fn parse_params(line: &str) -> (String, ItemParams) {
    let Some(pipe_index) = line.find('|') else {
        return (line.to_owned(), ItemParams::default());
    };

    let text = line[..pipe_index].to_owned();
    let param_str = &line[pipe_index + 1..];
    let mut params = ItemParams::default();
    let mut indexed_params: BTreeMap<usize, String> = BTreeMap::new();

    let mut rest = param_str;
    while let Some((key, value, next)) = next_param(rest) {
        match key.as_str() {
            "href" => params.href = Some(value),
            "shell" | "bash" => params.shell = Some(value),
            "refresh" => params.refresh = value == "true",
            "terminal" => params.terminal = value == "true",
            "dropdown" => params.dropdown = value == "true",
            "alternate" => params.alternate = value == "true",
            "disabled" => params.disabled = value == "true",
            "trim" => params.trim = value == "true",
            "color" => params.color = Some(value),
            "image" => params.image = decode_image_param(&value, false),
            "templateImage" => params.image = decode_image_param(&value, true),
            _ if key.starts_with("param") => {
                if let Ok(index) = key.trim_start_matches("param").parse::<usize>() {
                    indexed_params.insert(index, value);
                }
            }
            _ => {}
        }

        rest = next;
    }

    params.params = indexed_params.into_values().collect();

    (text, params)
}

fn next_param(input: &str) -> Option<(String, String, &str)> {
    let input = input.trim_start_matches([' ', '|']);
    if input.is_empty() {
        return None;
    }

    let equals_index = input.find('=')?;
    let key = input[..equals_index].trim().to_owned();
    let value_part = &input[equals_index + 1..];

    if value_part.is_empty() {
        return Some((key, String::new(), ""));
    }

    let mut chars = value_part.char_indices();
    let first = chars.next()?.1;

    if matches!(first, '"' | '\'') {
        let quote = first;
        let mut end_index = None;
        for (index, ch) in chars {
            if ch == quote {
                end_index = Some(index);
                break;
            }
        }

        let end_index = end_index.unwrap_or(value_part.len());
        let value = if end_index < value_part.len() {
            value_part[1..end_index].to_owned()
        } else {
            value_part[1..].to_owned()
        };
        let rest = if end_index < value_part.len() {
            &value_part[end_index + 1..]
        } else {
            ""
        };
        return Some((key, value, rest));
    }

    let end_index = value_part.find([' ', '|']).unwrap_or(value_part.len());
    let value = value_part[..end_index].trim().to_owned();
    let rest = &value_part[end_index..];

    Some((key, value, rest))
}

fn decode_image_param(value: &str, is_template: bool) -> Option<EmbeddedImage> {
    let normalized = value.trim().replace(char::is_whitespace, "");
    let bytes = BASE64_STANDARD.decode(normalized).ok()?;
    let is_svg = bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| byte == b'<');

    Some(EmbeddedImage {
        bytes,
        is_svg,
        is_template,
    })
}

fn normalize_menu_entries(entries: Vec<MenuEntry>) -> Vec<MenuEntry> {
    let mut normalized: Vec<MenuEntry> = Vec::new();

    for entry in entries {
        if entry.params.alternate {
            if let Some(previous) = normalized.last_mut() {
                previous.alternate = Some(Box::new(entry));
            }
            continue;
        }

        if !entry.params.dropdown {
            continue;
        }

        normalized.push(entry);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::{parse_plugin_output, parse_refresh_interval};
    use std::time::Duration;

    #[test]
    fn parses_cycle_and_menu_items() {
        let parsed = parse_plugin_output(
            "one\n\
             two\n\
             ---\n\
             item | href=https://example.com\n\
             --child\n\
             -----\n",
        );

        assert_eq!(parsed.title, "one");
        assert!(parsed.title_params.image.is_none());
        assert_eq!(parsed.cycle_items, vec!["one", "two"]);
        assert_eq!(parsed.menu_entries.len(), 3);
        assert_eq!(parsed.menu_entries[0].text, "item");
        assert_eq!(
            parsed.menu_entries[0].params.href.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(parsed.menu_entries[1].level, 1);
        assert!(parsed.menu_entries[2].separator);
    }

    #[test]
    fn parses_refresh_interval_from_name() {
        assert_eq!(
            parse_refresh_interval("weather.30s.sh"),
            Duration::from_secs(30)
        );
        assert_eq!(
            parse_refresh_interval("weather.10m.py"),
            Duration::from_secs(600)
        );
        assert_eq!(
            parse_refresh_interval("weather.sh"),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn parses_action_params_with_spaces_and_quotes() {
        let parsed = parse_plugin_output(
            "title\n\
             ---\n\
             Copy | bash='/bin/bash' param1='-c' param2=\"echo -n string | pbcopy\" refresh=true\n",
        );

        let entry = &parsed.menu_entries[0];
        assert_eq!(entry.params.shell.as_deref(), Some("/bin/bash"));
        assert_eq!(entry.params.params, vec!["-c", "echo -n string | pbcopy"]);
        assert!(entry.params.refresh);
    }

    #[test]
    fn handles_dropdown_alternate_and_disabled() {
        let parsed = parse_plugin_output(
            "title\n\
             ---\n\
             visible\n\
             hidden | dropdown=false\n\
             primary\n\
             alt | alternate=true\n\
             disabled | disabled=true\n",
        );

        assert_eq!(parsed.menu_entries.len(), 3);
        assert_eq!(parsed.menu_entries[0].text, "visible");
        assert_eq!(parsed.menu_entries[1].text, "primary");
        assert_eq!(
            parsed.menu_entries[1]
                .alternate
                .as_ref()
                .map(|entry| entry.text.as_str()),
            Some("alt")
        );
        assert!(parsed.menu_entries[2].params.disabled);
    }

    #[test]
    fn decodes_embedded_images() {
        let parsed = parse_plugin_output(
            "title | image=PHN2Zy8+\n\
             ---\n\
             item | templateImage=PHN2Zy8+\n",
        );

        let title_image = parsed
            .title_params
            .image
            .as_ref()
            .expect("title image should decode");
        assert!(title_image.is_svg);
        assert!(!title_image.is_template);

        let menu_image = parsed.menu_entries[0]
            .params
            .image
            .as_ref()
            .expect("menu image should decode");
        assert!(menu_image.is_svg);
        assert!(menu_image.is_template);
    }
}
