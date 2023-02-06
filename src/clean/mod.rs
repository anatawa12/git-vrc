use crate::yaml::{ParsedHeadingLine, YamlSeparated};
use log::trace;
use std::borrow::Cow;
use std::io::Read;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
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
pub(crate) struct App {
    #[clap(long = "file")]
    file: Option<String>,
    #[clap(long = "sort")]
    sort: bool,
}

impl App {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mut yaml = String::new();
        let mut stdin = stdin();
        const HEADER: &[u8] = b"%YAML";
        let mut heading = [0u8; HEADER.len()];
        stdin.read_exact(&mut heading)?;
        if heading != HEADER {
            // work as copy
            let mut stdout = stdout();
            stdout.write(&heading)?;
            std::io::copy(&mut stdin, &mut stdout)?;
            return Ok(());
        }
        yaml.push_str(std::str::from_utf8(HEADER).unwrap());
        stdin.read_to_string(&mut yaml)?;
        let mut iter = YamlSeparated::new(&yaml);
        let first = iter.next().unwrap();
        print!("{}{}", first.0, first.1);

        // filter phase
        let mut sections = iter
            .map(|(heading, body)| -> anyhow::Result<_> {
                trace!("start: {}", heading);
                Ok(YamlSection {
                    heading,
                    filtered: body.into(),
                    parsed: ParsedHeadingLine::from_str(heading)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        filter::main::filter(&mut sections)?;

        // optimization
        optimize_yaml(&mut sections);

        filter::remove_components::filter(&mut sections)?;

        let mut sort = self.sort;
        if let Some(path) = self.file {
            let (path, attr, value) = crate::git::check_attr(&["unity-sort"], &[path.as_str()])?
                .next()
                .expect("failed to get attr");
            if value.as_str() == "set" {
                sort = true
            }
        }

        if sort {
            sections.sort_by_key(|x| x.parsed.file_id())
        }

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
    pub fn local(file_id: i64) -> Self {
        Self {
            file_id,
            guid: None,
            obj_type: 0,
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

    pub(crate) fn is_local(&self) -> bool {
        self.guid.is_none()
    }

    #[allow(dead_code)]
    pub fn is_null(&self) -> bool {
        return self.file_id == 0;
    }
}
