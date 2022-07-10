use crate::yaml::YamlSeparated;
use log::trace;
use std::io::stdin;
use std::io::Read;

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

        while let Some((heading, body)) = iter.next() {
            trace!("start: {}", heading);
            let filtered = filter::filter_yaml(body)?;
            if !filtered.is_empty() {
                print!("{}", heading);
                print!("{}", filtered);
            }
        }

        Ok(())
    }
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
