use std::mem;
use std::str::Chars;
use log::trace;
use yaml_rust::scanner::{Marker, Scanner, Token, TokenType, TScalarStyle};
use yaml_rust::scanner::TokenType::*;
use crate::clean::{ObjectReference, ParserResult};
use crate::clean::ParserErr::EOF;

pub(crate) struct Context<'a> {
    printed: usize,
    yaml: &'a str,
    scanner: Scanner<Chars<'a>>,
    last_mark: Option<Marker>,
    mark: Option<Marker>,
    next_token: Option<Token>,
    result: String,
}

impl<'a> Context<'a> {
    pub(crate) fn mapping<'b>(
        &'b mut self,
        mut block: impl FnMut(&mut Context<'a>) -> ParserResult,
    ) -> ParserResult {
        match self.next()? {
            BlockMappingStart => loop {
                match self.next()? {
                    Key => block(self)?,
                    BlockEnd => return Ok(()),
                    e => unexpected_token!(e),
                }
            },
            FlowMappingStart => loop {
                match self.next()? {
                    Key => block(self)?,
                    FlowMappingEnd => return Ok(()),
                    e => unexpected_token!(e),
                }
                match self.next()? {
                    FlowEntry => {}
                    FlowMappingEnd => return Ok(()),
                    e => unexpected_token!(e),
                }
            },
            e => unexpected_token!(e),
        }
    }

    pub(crate) fn sequence<'b>(
        &'b mut self,
        mut block: impl FnMut(&mut Context<'a>) -> ParserResult,
    ) -> ParserResult {
        match self.next()? {
            BlockEntry => {
                block(self)?;
                while let BlockEntry = self.peek()? {
                    self.next()?;
                    block(self)?;
                }
                return Ok(());
            }
            FlowSequenceStart => loop {
                if let FlowSequenceEnd = self.peek()? {
                    self.next()?;
                    return Ok(());
                }
                block(self)?;
                match self.next()? {
                    FlowEntry => {}
                    FlowSequenceEnd => return Ok(()),
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
                    Ok(())
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
            Ok(())
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

    // write until current token. including current token with margin
    pub(crate) fn write_until_current_token(&mut self) -> ParserResult {
        trace!("write_until_current_token");
        self.peek()?;
        self.append(self.next_token.as_ref().unwrap().0.begin().index());
        Ok(())
    }

    pub(crate) fn write_until_current_token0(&mut self) -> ParserResult {
        trace!("write_until_current_token0");
        self.append(self.mark.unwrap().end().index());
        Ok(())
    }

    pub(crate) fn write_until_last_token(&mut self) -> ParserResult {
        trace!("write_until_current_token_pre");
        self.append(self.last_mark.unwrap().end().index());
        Ok(())
    }

    pub(crate) fn skip_until_last_token(&mut self) -> ParserResult {
        trace!("skip_until_last_token");
        self.printed = self.last_mark.unwrap().end().index();
        Ok(())
    }

    pub(crate) fn skip_until_current_token(&mut self) -> ParserResult {
        trace!("skip_until_current_token");
        let mark = self.mark.unwrap();
        if mark.begin().index() == mark.end().index() {
            // it's position tokentrim
            self.printed = self.yaml[..mark.begin().index()].trim_end().len()
        } else {
            // it's a token
            self.printed = self.mark.unwrap().end().index();
        }
        Ok(())
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
