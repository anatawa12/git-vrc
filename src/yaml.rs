use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub(crate) struct YamlSeparated<'a> {
    str: &'a str,
}

impl<'a> YamlSeparated<'a> {
    pub(crate) fn new(str: &'a str) -> Self {
        Self { str }
    }
}

impl<'a> Iterator for YamlSeparated<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.str.len() == 0 {
            return None;
        }

        let heading_line;
        if !self.str.starts_with("---") {
            // heading element: no heading line
            heading_line = "";
        } else {
            let rest;
            if let Some(lf) = self.str.find('\n') {
                (heading_line, rest) = self.str.split_at(lf + 1)
            } else {
                (heading_line, rest) = (self.str, "")
            }
            self.str = rest;
        }

        let str_in = self.str;
        let mut i = 0;

        loop {
            if let Some(triple_hyphen) = self.str.find("---") {
                if self.str[..triple_hyphen].chars().last() == Some('\n') {
                    // it's separator!
                    i += triple_hyphen;
                    break;
                } else {
                    // it's not a separator. find next
                    i += triple_hyphen;
                    let (_, post) = self.str.split_at(triple_hyphen + 3);
                    self.str = post;
                }
            } else {
                i = self.str.len();
                // there's no separator!
                break;
            }
        }
        self.str = &str_in[i..];

        return Some((heading_line, &str_in[..i]));
    }
}

#[derive(Debug)]
pub(crate) struct HeadingLineParsingErr(HeadingLineParsingErrInner);

#[derive(Debug)]
enum HeadingLineParsingErrInner {
    NoSeparator,
    NoFileId,
    UnknownFlags(String),
}

impl Display for HeadingLineParsingErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            HeadingLineParsingErrInner::NoSeparator => f.write_str("no separator found"),
            HeadingLineParsingErrInner::NoFileId => f.write_str("no fileID found"),
            HeadingLineParsingErrInner::UnknownFlags(flg) => write!(f, "unknown flag: {}", flg),
        }
    }
}

impl Error for HeadingLineParsingErr {}

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct ParsedHeadingLine {
    file_id: i64,
    is_stripped: bool,
}

impl ParsedHeadingLine {
    #[allow(dead_code)]
    pub fn new(file_id: i64, is_stripped: bool) -> Self {
        Self {
            file_id,
            is_stripped,
        }
    }

    pub fn file_id(&self) -> i64 {
        self.file_id
    }

    pub fn is_stripped(&self) -> bool {
        self.is_stripped
    }
}

impl FromStr for ParsedHeadingLine {
    type Err = HeadingLineParsingErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use HeadingLineParsingErrInner::*;

        if !s.starts_with("--- ") {
            return Err(HeadingLineParsingErr(NoSeparator));
        }
        let s = s[4..].trim_start();
        let amp = s.find('&').ok_or(HeadingLineParsingErr(NoFileId))?;
        let s = &s[(amp + 1)..]; // +1: skil '&'
        let non_digit = s
            .find(|c: char| !c.is_ascii_digit() && c != '-')
            .unwrap_or(s.len());
        let file_id: i64 = s[..non_digit]
            .parse()
            .map_err(|_| HeadingLineParsingErr(NoFileId))?;
        let mut s = s[non_digit..].trim_start();

        let mut is_stripped = false;

        if s.starts_with("stripped") {
            is_stripped = true;
            s = &s["stripped".len()..].trim_start();
        }

        if !s.is_empty() {
            return Err(HeadingLineParsingErr(UnknownFlags(s.to_owned())));
        }

        Ok(ParsedHeadingLine {
            file_id,
            is_stripped,
        })
    }
}

#[test]
fn parsed_heading_line_parse() {
    assert_eq!(
        ParsedHeadingLine {
            file_id: 1,
            is_stripped: false,
        },
        "--- !u!29 &1".parse().unwrap()
    );

    assert_eq!(
        ParsedHeadingLine {
            file_id: -263184606691600302,
            is_stripped: false,
        },
        "--- !u!114 &-263184606691600302".parse().unwrap()
    );

    assert_eq!(
        ParsedHeadingLine {
            file_id: 484105423,
            is_stripped: true,
        },
        "--- !u!114 &484105423 stripped".parse().unwrap()
    );
}
