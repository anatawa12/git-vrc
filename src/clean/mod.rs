use crate::yaml::{ParsedHeadingLine, YamlSeparated};
use log::trace;
use std::borrow::Cow;
use std::io::stdin;
use std::io::Read;
use std::str::FromStr;

macro_rules! expect_token {
    ($token: expr, $($expect: tt)*) => {
        match $token {
            $($expect)* => {}
            e => unexpected_token!(e, stringify!($($expect)*)),
        }
    };
}

macro_rules! unexpected_token {
    ($token: expr) => {
        panic!("unexpected token: {:?}", $token)
    };
    ($token: expr, $expected: expr) => {
        panic!("expected {} but was {:?}", $expected, $token)
    };
}

mod filter;

#[derive(clap::Parser)]
/// clean file.
pub(crate) struct App {}

impl App {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mut yaml = String::new();
        stdin().read_to_string(&mut yaml)?;
        let mut iter = YamlSeparated::new(&yaml);
        let first = iter.next().unwrap();
        print!("{}{}", first.0, first.1);

        // filter phase
        let mut sections = Vec::new();
        for (heading, body) in iter {
            trace!("start: {}", heading);
            let filtered = filter::first::filter_yaml(body)?;
            sections.push(YamlSection {
                heading,
                filtered,
                parsed: ParsedHeadingLine::from_str(heading)?,
            })
        }

        // optimization
        optimize_yaml(&mut sections);

        for sec in sections {
            if !sec.filtered.is_empty() {
                print!("{}{}", sec.heading, sec.filtered);
            }
        }

        Ok(())
    }
}

/// optimize yaml. remove unused stripped object
fn optimize_yaml(sections: &mut [YamlSection]) {
    for i in 0..sections.len() {
        let sec = &mut sections[i];

        if sec.parsed.is_stripped() {
            let find = format!("{{fileID: {}}}", sec.parsed.file_id());
            // find `{fileID: <file-id>}`

            let mut found = false;
            for j in 0..sections.len() {
                if sections[j].filtered.contains(&find) {
                    found = true;
                    break;
                }
            }
            if !found {
                sections[i].filtered = Cow::Borrowed("");
            }
        }
    }
}

#[test]
fn optimize_yaml_test() {
    macro_rules! test {
        ($expect: expr, $input: expr) => {{
            let mut slice = $input;
            optimize_yaml(&mut slice);
            assert_eq!($expect, slice);
        }};
    }

    // do not optimize if exists
    test!(
        [
            YamlSection {
                heading: "--- !u!114 &484105423 stripped",
                parsed: ParsedHeadingLine::new(484105423, true),
                filtered: Cow::Borrowed("MonoBehaviour:\n"),
            },
            YamlSection {
                heading: "--- !u!114 &2087762956",
                parsed: ParsedHeadingLine::new(2087762956, false),
                filtered: Cow::Borrowed("MonoBehaviour:\n  script: {fileID: 484105423}\n"),
            }
        ],
        [
            YamlSection {
                heading: "--- !u!114 &484105423 stripped",
                parsed: ParsedHeadingLine::new(484105423, true),
                filtered: Cow::Borrowed("MonoBehaviour:\n"),
            },
            YamlSection {
                heading: "--- !u!114 &2087762956",
                parsed: ParsedHeadingLine::new(2087762956, false),
                filtered: Cow::Borrowed("MonoBehaviour:\n  script: {fileID: 484105423}\n"),
            }
        ]
    );

    // remove that if no reference found
    test!(
        [
            YamlSection {
                heading: "--- !u!114 &484105423 stripped",
                parsed: ParsedHeadingLine::new(484105423, true),
                filtered: Cow::Borrowed(""),
            },
            YamlSection {
                heading: "--- !u!114 &2087762956",
                parsed: ParsedHeadingLine::new(2087762956, false),
                filtered: Cow::Borrowed("MonoBehaviour:\n"),
            }
        ],
        [
            YamlSection {
                heading: "--- !u!114 &484105423 stripped",
                parsed: ParsedHeadingLine::new(484105423, true),
                filtered: Cow::Borrowed("MonoBehaviour:\n"),
            },
            YamlSection {
                heading: "--- !u!114 &2087762956",
                parsed: ParsedHeadingLine::new(2087762956, false),
                filtered: Cow::Borrowed("MonoBehaviour:\n"),
            }
        ]
    );
}

#[derive(Eq, PartialEq, Debug)]
struct YamlSection<'a> {
    heading: &'a str,
    parsed: ParsedHeadingLine,
    filtered: Cow<'a, str>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub(crate) struct ObjectReference {
    file_id: i64,
    guid: Option<String>,
    obj_type: u32,
}

impl ObjectReference {
    #[allow(dead_code)]
    pub fn new(file_id: i64, guid: String, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: Some(guid),
            obj_type,
        }
    }

    #[allow(dead_code)]
    pub fn local(file_id: i64, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: None,
            obj_type,
        }
    }

    #[allow(dead_code)]
    pub fn null() -> Self {
        Self {
            file_id: 0,
            guid: None,
            obj_type: 0,
        }
    }

    #[allow(dead_code)]
    pub fn is_null(&self) -> bool {
        return self.file_id == 0;
    }
}
