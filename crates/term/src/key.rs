use std::fmt;
use std::ops::{BitOr, BitOrAssign};
use std::str::FromStr;

#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Tab,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

const NAMED: [(&str, KeyCode); 15] = [
    ("space", KeyCode::Char(' ')),
    ("ret", KeyCode::Enter),
    ("esc", KeyCode::Esc),
    ("backspace", KeyCode::Backspace),
    ("tab", KeyCode::Tab),
    ("del", KeyCode::Delete),
    ("ins", KeyCode::Insert),
    ("home", KeyCode::Home),
    ("end", KeyCode::End),
    ("pageup", KeyCode::PageUp),
    ("pagedown", KeyCode::PageDown),
    ("up", KeyCode::Up),
    ("down", KeyCode::Down),
    ("left", KeyCode::Left),
    ("right", KeyCode::Right),
];

const PREFIXES: [(&str, Modifiers); 3] = [
    ("C", Modifiers::CONTROL),
    ("A", Modifiers::ALT),
    ("S", Modifiers::SHIFT),
];

impl Key {
    pub const fn new(code: KeyCode, modifiers: Modifiers) -> Self {
        let modifiers = match code {
            KeyCode::Char(_) => modifiers.without(Modifiers::SHIFT),
            _ => modifiers,
        };
        Self { code, modifiers }
    }

    pub const fn char(c: char) -> Self {
        Self::new(KeyCode::Char(c), Modifiers::NONE)
    }

    pub const fn ctrl(c: char) -> Self {
        Self::new(KeyCode::Char(c), Modifiers::CONTROL)
    }

    pub const fn alt(c: char) -> Self {
        Self::new(KeyCode::Char(c), Modifiers::ALT)
    }

    pub fn as_char(self) -> Option<char> {
        match self.code {
            KeyCode::Char(c) if self.modifiers.is_empty() => Some(c),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseKeyError(String);

impl fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid key {:?}", self.0)
    }
}

impl std::error::Error for ParseKeyError {}

impl FromStr for Key {
    type Err = ParseKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = || ParseKeyError(s.to_owned());
        let mut modifiers = Modifiers::NONE;
        let mut rest = s;
        while let Some((prefix, tail)) = rest.split_once('-') {
            if tail.is_empty() {
                break;
            }
            let (_, modifier) = PREFIXES
                .into_iter()
                .find(|&(name, _)| name == prefix)
                .ok_or_else(error)?;
            modifiers |= modifier;
            rest = tail;
        }
        let code = if rest == "minus" {
            KeyCode::Char('-')
        } else if let Some(&(_, code)) = NAMED.iter().find(|&&(name, _)| name == rest) {
            code
        } else {
            let mut chars = rest.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                (Some('F'), Some(_)) => match rest[1..].parse() {
                    Ok(n @ 1..=24) => KeyCode::F(n),
                    _ => return Err(error()),
                },
                _ => return Err(error()),
            }
        };
        Ok(Self::new(code, modifiers))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (prefix, modifier) in PREFIXES {
            if self.modifiers.contains(modifier) {
                write!(f, "{prefix}-")?;
            }
        }
        if let Some((name, _)) = NAMED.iter().find(|&&(_, code)| code == self.code) {
            return f.write_str(name);
        }
        match self.code {
            KeyCode::Char('-') if !self.modifiers.is_empty() => f.write_str("minus"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::F(n) => write!(f, "F{n}"),
            _ => unreachable!("every other key is named"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        s.parse().unwrap()
    }

    #[test]
    fn parses_plain_characters() {
        assert_eq!(key("a"), Key::char('a'));
        assert_eq!(key("A"), Key::char('A'));
        assert_eq!(key("%"), Key::char('%'));
        assert_eq!(key("-"), Key::char('-'));
    }

    #[test]
    fn parses_modifiers() {
        assert_eq!(key("C-s"), Key::ctrl('s'));
        assert_eq!(key("A-x"), Key::alt('x'));
        assert_eq!(
            key("C-A-x"),
            Key::new(KeyCode::Char('x'), Modifiers::CONTROL | Modifiers::ALT)
        );
        assert_eq!(key("S-tab"), Key::new(KeyCode::Tab, Modifiers::SHIFT));
    }

    #[test]
    fn parses_named_keys() {
        assert_eq!(key("space"), Key::char(' '));
        assert_eq!(key("minus"), Key::char('-'));
        assert_eq!(key("A-minus"), Key::alt('-'));
        assert_eq!(key("ret"), Key::new(KeyCode::Enter, Modifiers::NONE));
        assert_eq!(key("esc"), Key::new(KeyCode::Esc, Modifiers::NONE));
        assert_eq!(key("pageup"), Key::new(KeyCode::PageUp, Modifiers::NONE));
        assert_eq!(key("F5"), Key::new(KeyCode::F(5), Modifiers::NONE));
    }

    #[test]
    fn characters_imply_shift() {
        assert_eq!(
            Key::new(KeyCode::Char('A'), Modifiers::SHIFT),
            Key::char('A')
        );
        assert_eq!(
            Key::new(KeyCode::Char('A'), Modifiers::SHIFT | Modifiers::ALT),
            Key::alt('A')
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        for bad in ["", "C-", "foo", "F99", "X-a"] {
            assert!(bad.parse::<Key>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn displays_in_config_notation() {
        assert_eq!(Key::ctrl('s').to_string(), "C-s");
        assert_eq!(Key::char(' ').to_string(), "space");
        assert_eq!(Key::alt('-').to_string(), "A-minus");
        assert_eq!(Key::char('-').to_string(), "-");
        assert_eq!(
            Key::new(KeyCode::Tab, Modifiers::SHIFT).to_string(),
            "S-tab"
        );
        assert_eq!(Key::new(KeyCode::F(1), Modifiers::NONE).to_string(), "F1");
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "a", "C-s", "A-x", "S-tab", "space", "ret", "F12", "A-minus", "C-A-del",
        ] {
            assert_eq!(key(s).to_string(), s);
        }
    }

    #[test]
    fn plain_characters_are_insertable() {
        assert_eq!(Key::char('x').as_char(), Some('x'));
        assert_eq!(Key::ctrl('x').as_char(), None);
        assert_eq!(Key::new(KeyCode::Enter, Modifiers::NONE).as_char(), None);
    }
}
