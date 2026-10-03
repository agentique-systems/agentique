//! A small JSON reader and writer, so the example has no dependencies.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        *self == Json::Null
    }

    pub fn object(fields: Vec<(&str, Json)>) -> Json {
        Json::Object(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{n}"));
                }
            }
            Json::String(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(map) => {
                out.push('{');
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    Json::String(key.clone()).write(out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }

    pub fn parse(text: &str) -> Result<Json, String> {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        let value = parse_value(&chars, &mut i)?;
        skip_space(&chars, &mut i);
        if i != chars.len() {
            return Err(format!("unexpected text at {i}"));
        }
        Ok(value)
    }
}

fn skip_space(chars: &[char], i: &mut usize) {
    while *i < chars.len() && chars[*i].is_whitespace() {
        *i += 1;
    }
}

fn parse_value(chars: &[char], i: &mut usize) -> Result<Json, String> {
    skip_space(chars, i);
    match chars.get(*i) {
        Some('{') => {
            *i += 1;
            let mut map = BTreeMap::new();
            skip_space(chars, i);
            if chars.get(*i) == Some(&'}') {
                *i += 1;
                return Ok(Json::Object(map));
            }
            loop {
                skip_space(chars, i);
                let Json::String(key) = parse_value(chars, i)? else {
                    return Err("an object key must be a string".into());
                };
                skip_space(chars, i);
                if chars.get(*i) != Some(&':') {
                    return Err("expected `:`".into());
                }
                *i += 1;
                let value = parse_value(chars, i)?;
                // A key named twice would lose one of its values.
                if map.insert(key.clone(), value).is_some() {
                    return Err(format!("the key `{key}` appears twice"));
                }
                skip_space(chars, i);
                match chars.get(*i) {
                    Some(',') => *i += 1,
                    Some('}') => {
                        *i += 1;
                        return Ok(Json::Object(map));
                    }
                    _ => return Err("expected `,` or `}`".into()),
                }
            }
        }
        Some('[') => {
            *i += 1;
            let mut items = Vec::new();
            skip_space(chars, i);
            if chars.get(*i) == Some(&']') {
                *i += 1;
                return Ok(Json::Array(items));
            }
            loop {
                items.push(parse_value(chars, i)?);
                skip_space(chars, i);
                match chars.get(*i) {
                    Some(',') => *i += 1,
                    Some(']') => {
                        *i += 1;
                        return Ok(Json::Array(items));
                    }
                    _ => return Err("expected `,` or `]`".into()),
                }
            }
        }
        Some('"') => {
            *i += 1;
            let mut out = String::new();
            while let Some(&c) = chars.get(*i) {
                *i += 1;
                match c {
                    '"' => return Ok(Json::String(out)),
                    '\\' => {
                        let escaped = chars.get(*i).copied().ok_or("unfinished escape")?;
                        *i += 1;
                        match escaped {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            'r' => out.push('\r'),
                            'u' => {
                                let hex: String =
                                    chars[*i..(*i + 4).min(chars.len())].iter().collect();
                                *i += 4;
                                let code =
                                    u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                                out.push(char::from_u32(code).unwrap_or('?'));
                            }
                            other => out.push(other),
                        }
                    }
                    c => out.push(c),
                }
            }
            Err("unfinished string".into())
        }
        Some('t') if chars[*i..].starts_with(&['t', 'r', 'u', 'e']) => {
            *i += 4;
            Ok(Json::Bool(true))
        }
        Some('f') if chars[*i..].starts_with(&['f', 'a', 'l', 's', 'e']) => {
            *i += 5;
            Ok(Json::Bool(false))
        }
        Some('n') if chars[*i..].starts_with(&['n', 'u', 'l', 'l']) => {
            *i += 4;
            Ok(Json::Null)
        }
        Some(c) if *c == '-' || c.is_ascii_digit() => {
            let start = *i;
            *i += 1;
            while chars
                .get(*i)
                .is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
            {
                *i += 1;
            }
            let text: String = chars[start..*i].iter().collect();
            text.parse::<f64>()
                .map(Json::Number)
                .map_err(|e| format!("`{text}`: {e}"))
        }
        _ => Err(format!("unexpected text at {i}")),
    }
}
