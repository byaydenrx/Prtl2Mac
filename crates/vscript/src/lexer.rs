//! Squirrel tokens.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    /// Punctuation and operators, as written (`<-`, `==`, `{`...).
    Op(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
    /// A line break came between this token and the one before.
    pub newline_before: bool,
}

const OPS: &[&str] = &[
    "<=>", "<<=", ">>=", "...", "<-", "::", "==", "!=", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "<<", ">>",
    "{", "}", "(", ")", "[", "]", ".", ",", ";", ":", "?", "+", "-", "*", "/", "%", "=", "<", ">", "!", "~", "&", "|", "^", "@",
];

pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut line = 1u32;
    let mut out = Vec::new();
    let mut nl = true;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            nl = true;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Comments.
        if c == b'#' || (c == b'/' && b.get(i + 1) == Some(&b'/')) {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                if b[i] == b'\n' {
                    line += 1;
                    nl = true;
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        let start_line = line;
        let tok = if c.is_ascii_alphabetic() || c == b'_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            Tok::Ident(src[s..i].to_string())
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let s = i;
            if c == b'0' && matches!(b.get(i + 1), Some(b'x') | Some(b'X')) {
                i += 2;
                let h = i;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    i += 1;
                }
                Tok::Int(i64::from_str_radix(&src[h..i], 16).map_err(|e| format!("line {line}: {e}"))?)
            } else {
                let mut float = false;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || ((b[i] == b'e' || b[i] == b'E') && float_exp(b, i))) {
                    if b[i] == b'.' {
                        // `1..` isn't a thing in Squirrel, but `x.len()` after a number is.
                        if !b.get(i + 1).is_some_and(|d| d.is_ascii_digit()) && float {
                            break;
                        }
                        float = true;
                    }
                    if b[i] == b'e' || b[i] == b'E' {
                        float = true;
                        if matches!(b.get(i + 1), Some(b'+') | Some(b'-')) {
                            i += 1;
                        }
                    }
                    i += 1;
                }
                let text = &src[s..i];
                if float {
                    Tok::Float(text.parse().map_err(|e| format!("line {line}: bad number {text}: {e}"))?)
                } else {
                    Tok::Int(text.parse().map_err(|e| format!("line {line}: bad number {text}: {e}"))?)
                }
            }
        } else if c == b'"' || (c == b'@' && b.get(i + 1) == Some(&b'"')) {
            let verbatim = c == b'@';
            i += if verbatim { 2 } else { 1 };
            let mut s = String::new();
            loop {
                let Some(&ch) = b.get(i) else { return Err(format!("line {start_line}: unterminated string")) };
                if ch == b'"' {
                    if verbatim && b.get(i + 1) == Some(&b'"') {
                        s.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                if ch == b'\n' {
                    line += 1;
                }
                if ch == b'\\' && !verbatim {
                    let e = b.get(i + 1).copied().unwrap_or(b'\\');
                    i += 2;
                    match e {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'0' => s.push('\0'),
                        b'a' => s.push('\x07'),
                        b'b' => s.push('\x08'),
                        b'f' => s.push('\x0c'),
                        b'v' => s.push('\x0b'),
                        b'x' => {
                            let h = i;
                            while i < b.len() && i < h + 4 && b[i].is_ascii_hexdigit() {
                                i += 1;
                            }
                            let v = u32::from_str_radix(&src[h..i], 16).unwrap_or(0);
                            s.push(char::from_u32(v).unwrap_or('?'));
                        }
                        other => s.push(other as char),
                    }
                    continue;
                }
                // Copy one UTF-8 character.
                let len = utf8_len(ch);
                s.push_str(&src[i..(i + len).min(b.len())]);
                i += len;
            }
            Tok::Str(s)
        } else if c == b'\'' {
            // A character constant is an integer.
            let (v, len) = if b.get(i + 1) == Some(&b'\\') {
                let e = b.get(i + 2).copied().unwrap_or(0);
                (
                    match e {
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'0' => 0,
                        other => other,
                    },
                    4,
                )
            } else {
                (b.get(i + 1).copied().unwrap_or(0), 3)
            };
            i += len;
            Tok::Int(v as i64)
        } else {
            let mut found = None;
            for op in OPS {
                if src[i..].starts_with(op) {
                    found = Some(*op);
                    break;
                }
            }
            let Some(op) = found else { return Err(format!("line {line}: unexpected character {:?}", c as char)) };
            i += op.len();
            Tok::Op(op)
        };
        out.push(Token { tok, line: start_line, newline_before: nl });
        nl = false;
    }
    out.push(Token { tok: Tok::Eof, line, newline_before: true });
    Ok(out)
}

fn float_exp(b: &[u8], i: usize) -> bool {
    // `1e5`, `1.5e-3`: an exponent only when digits follow.
    let mut j = i + 1;
    if matches!(b.get(j), Some(b'+') | Some(b'-')) {
        j += 1;
    }
    b.get(j).is_some_and(|d| d.is_ascii_digit())
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        let t = lex("local x <- 0x10 + 1.5e2; // c\n@\"a\"\"b\" 'a'").unwrap();
        let toks: Vec<Tok> = t.into_iter().map(|t| t.tok).collect();
        assert_eq!(
            toks,
            vec![
                Tok::Ident("local".into()),
                Tok::Ident("x".into()),
                Tok::Op("<-"),
                Tok::Int(16),
                Tok::Op("+"),
                Tok::Float(150.0),
                Tok::Op(";"),
                Tok::Str("a\"b".into()),
                Tok::Int(97),
                Tok::Eof
            ]
        );
    }
}
