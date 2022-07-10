use crate::clean::ParserErr::EOF;
use crate::clean::{ObjectReference, ParserResult};
use log::trace;
use std::mem;
use std::ops::ControlFlow;
use std::str::Chars;
use yaml_rust::scanner::TokenType::*;
use yaml_rust::scanner::{Marker, Scanner, TScalarStyle, Token, TokenType};
use ControlFlow::Continue;

pub(crate) struct Context<'a> {
    printed: usize,
    yaml: &'a str,
    scanner: Scanner<Chars<'a>>,
    last_mark: Option<Marker>,
    mark: Option<Marker>,
    next_token: Option<Token>,
    result: String,
}

macro_rules! return_ok_if_break {
    ($controlflow: expr) => {
        match $controlflow {
            ControlFlow::Break(v) => return Ok(v),
            ControlFlow::Continue(()) => {}
        }
    };
}

impl<'a> Context<'a> {
    pub(crate) fn mapping<'b, R: Default>(
        &'b mut self,
        mut block: impl FnMut(&mut Context<'a>) -> ParserResult<ControlFlow<R>>,
    ) -> ParserResult<R> {
        match self.next()? {
            BlockMappingStart => loop {
                match self.next()? {
                    Key => return_ok_if_break!(block(self)?),
                    BlockEnd => return Ok(R::default()),
                    e => unexpected_token!(e),
                }
            },
            FlowMappingStart => loop {
                match self.next()? {
                    Key => return_ok_if_break!(block(self)?),
                    FlowMappingEnd => return Ok(R::default()),
                    e => unexpected_token!(e),
                }
                match self.next()? {
                    FlowEntry => {}
                    FlowMappingEnd => return Ok(R::default()),
                    e => unexpected_token!(e),
                }
            },
            e => unexpected_token!(e),
        }
    }

    pub(crate) fn sequence<'b, R: Default>(
        &'b mut self,
        mut block: impl FnMut(&mut Context<'a>) -> ParserResult<ControlFlow<R>>,
    ) -> ParserResult<R> {
        match self.next()? {
            BlockEntry => {
                return_ok_if_break!(block(self)?);
                while let BlockEntry = self.peek()? {
                    self.next()?;
                    return_ok_if_break!(block(self)?);
                }
                return Ok(R::default());
            }
            FlowSequenceStart => loop {
                if let FlowSequenceEnd = self.peek()? {
                    self.next()?;
                    return Ok(R::default());
                }
                return_ok_if_break!(block(self)?);
                match self.next()? {
                    FlowEntry => {}
                    FlowSequenceEnd => return Ok(R::default()),
                    e => unexpected_token!(e),
                }
            },
            e => unexpected_token!(e),
        }
    }

    pub(crate) fn next_scalar(&mut self) -> ParserResult<(String, TScalarStyle)> {
        match self.peek()? {
            BlockEnd | FlowMappingEnd | Key | Value => {
                return Ok(("~".to_owned(), TScalarStyle::Plain))
            }
            Scalar(_, _) => {
                if let Scalar(style, value) = self.next()? {
                    Ok((value, style))
                } else {
                    unreachable!()
                }
            }
            e => panic!("scalar expected but was: {:?}", e),
        }
    }

    pub(crate) fn skip_next_value(&mut self) -> ParserResult {
        loop {
            return match self.peek()? {
                BlockEnd | FlowMappingEnd | Key | Value => return Ok(()),
                BlockMappingStart | FlowMappingStart => self.mapping(|ctx| {
                    ctx.skip_next_value()?;
                    expect_token!(ctx.next()?, Value);
                    ctx.skip_next_value()?;
                    Ok(Continue(()))
                }),

                BlockEntry => Ok(while let BlockEntry = self.peek()? {
                    self.next()?;
                    self.skip_next_value()?;
                }),

                FlowSequenceStart => {
                    self.next()?;
                    expect_token!(self.next()?, FlowSequenceEnd);
                    Ok(())
                }

                Scalar(_, _) => {
                    self.next()?;
                    Ok(())
                }

                e => unexpected_token!(e),
            };
        }
    }

    pub(crate) fn parse_object_reference(&mut self) -> ParserResult<ObjectReference> {
        let mut file_id: Option<i64> = None;
        let mut guid: Option<String> = None;
        let mut object_type: Option<u32> = None;

        self.mapping(|ctx| {
            let name = ctx.next_scalar()?.0;
            expect_token!(ctx.next()?, Value);
            match name.as_str() {
                "fileID" => file_id = Some(ctx.next_scalar()?.0.parse().unwrap()),
                "guid" => guid = Some(ctx.next_scalar()?.0),
                "type" => object_type = Some(ctx.next_scalar()?.0.parse().unwrap()),
                unknown => panic!("unknown key for object reference: {}", unknown),
            }
            Ok(Continue(()))
        })?;

        let file_id = file_id.expect("fileID does not exist");
        if file_id == 0 {
            Ok(ObjectReference::null())
        } else if let Some(guid) = guid {
            Ok(ObjectReference::new(
                file_id,
                guid,
                object_type.expect("type does not exist"),
            ))
        } else {
            Ok(ObjectReference::local(
                file_id,
                object_type.expect("type does not exist"),
            ))
        }
    }
}

impl<'a> Context<'a> {
    pub(crate) fn new(yaml: &'a str) -> Self {
        Self {
            printed: 0,
            yaml,
            scanner: Scanner::new(yaml.chars()),
            last_mark: None,
            mark: None,
            next_token: None,
            result: String::with_capacity(yaml.len()),
        }
    }

    pub(crate) fn peek(&mut self) -> ParserResult<&TokenType> {
        // because get_or_insert_with cannot return result,
        // this reimplement get_or_insert_with.
        if matches!(self.next_token, None) {
            mem::forget(mem::replace(
                &mut self.next_token,
                Some(self.scanner.next_token()?.ok_or(EOF)?),
            ));
        }
        unsafe { Ok(&self.next_token.as_ref().unwrap_unchecked().1) }
    }

    pub(crate) fn next(&mut self) -> ParserResult<TokenType> {
        self.last_mark = self.mark;
        if let Some(token) = self.next_token.take() {
            self.mark = Some(token.0);
            trace!("{:?}", token);
            Ok(token.1)
        } else {
            let token = self.scanner.next_token()?.ok_or(EOF)?;
            self.mark = Some(token.0);
            trace!("{:?}", token);
            Ok(token.1)
        }
    }

    // write until current token. including current token but not with suffix
    pub(crate) fn write_until_current_token(&mut self) -> ParserResult {
        trace!("write_until_current_token");
        self.append(self.mark_pos(self.mark.unwrap()));
        Ok(())
    }

    pub(crate) fn write_until_last_token(&mut self) -> ParserResult {
        trace!("write_until_last_token");
        self.append(self.mark_pos(self.last_mark.unwrap()));
        Ok(())
    }

    pub(crate) fn skip_until_last_token(&mut self) -> ParserResult {
        trace!("skip_until_last_token");
        self.printed = self.mark_pos(self.last_mark.unwrap());
        Ok(())
    }

    pub(crate) fn skip_until_current_token(&mut self) -> ParserResult {
        trace!("skip_until_current_token");
        self.printed = self.mark_pos(self.mark.unwrap());
        Ok(())
    }

    fn mark_pos(&self, mark: Marker) -> usize {
        if mark.begin().index() == mark.end().index() {
            // it's position token
            self.yaml[..mark.begin().index()].trim_end().len()
        } else {
            // it's a token
            mark.end().index()
        }
    }

    pub(crate) fn append_str(&mut self, str: &str) {
        trace!("append_str: {}", str);
        self.result.push_str(str);
    }

    fn append(&mut self, index: usize) {
        self.result.push_str(&self.yaml[self.printed..index]);
        self.printed = index;
    }

    pub(crate) fn finish(mut self) -> String {
        self.result.push_str(&self.yaml[self.printed..]);
        self.result
    }
}
