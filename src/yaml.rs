use std::str::Chars;
use yaml_rust::scanner::Marker;
use yaml_rust::{Event, ScanError};

/// a wrapper of rust_yaml
pub struct YamlParser<'a> {
    src: &'a str,
    upstream: yaml_rust::parser::Parser<Chars<'a>>,
}

impl<'a> YamlParser<'a> {
    pub fn new(src: &'a str) -> Self {
        let upstream = yaml_rust::parser::Parser::new(src.chars());
        Self { src, upstream }
    }

    pub fn next(&mut self) -> Result<(Event, Marker), ScanError> {
        let (event, marker) = self.upstream.next()?;
        let (_, next_marker) = self.upstream.peek()?;
        match (&event, self.src[marker.begin().index()..].chars().nth(0)) {
            (Event::MappingStart(_), Some(':')) => Ok((event, Marker::emtpy(*next_marker.begin()))),
            _ => Ok((event, marker)),
        }
    }
}
