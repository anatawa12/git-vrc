use crate::clean::ObjectReference;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::ControlFlow;
use std::ops::ControlFlow::Continue;
use std::str::Chars;
use yaml_rust::scanner::*;
use ParserErr::EOF;
use TokenType::*;

pub(crate) type ParserResult<T = ()> = Result<T, ParserErr>;

pub(crate) enum ParserErr {
    Scan(ScanError),
    EOF,
}

impl Debug for ParserErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserErr::Scan(e) => Debug::fmt(e, f),
            EOF => f.write_str("EOF"),
        }
    }
}

impl Display for ParserErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserErr::Scan(e) => Display::fmt(e, f),
            EOF => f.write_str("EOF"),
        }
    }
}

impl Error for ParserErr {}

impl From<ScanError> for ParserErr {
    fn from(e: ScanError) -> Self {
        Self::Scan(e)
    }
}

pub(crate) struct Context<'a> {
    printed: usize,
    yaml: &'a str,
    scanner: Scanner<Chars<'a>>,
    last_mark: Option<Marker>,
    mark: Option<Marker>,
    next_token: Option<Token>,
    will_write: Option<(usize, NonZeroUsize)>,
    result: Vec<&'a str>,
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
                return Ok((String::new(), TScalarStyle::Plain))
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
                    if !matches!(ctx.peek(), Ok(FlowEntry)) {
                        ctx.skip_next_value()?;
                    }
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
            Ok(ObjectReference::local(file_id))
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
            will_write: None,
            result: Vec::new(),
        }
    }

    pub(crate) fn peek(&mut self) -> ParserResult<&TokenType> {
        // because get_or_insert_with cannot return result,
        // this reimplement get_or_insert_with.
        if matches!(self.next_token, None) {
            std::mem::forget(std::mem::replace(
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
            log::trace!("{:?}", token);
            Ok(token.1)
        } else {
            let token = self.scanner.next_token()?.ok_or(EOF)?;
            self.mark = Some(token.0);
            log::trace!("{:?}", token);
            Ok(token.1)
        }
    }

    // write until current token. including current token but not with suffix
    pub(crate) fn write_until_current_token(&mut self) -> ParserResult {
        log::trace!("write_until_current_token");
        self.append(self.mark_pos(self.mark.unwrap()));
        Ok(())
    }

    pub(crate) fn write_until_last_token(&mut self) -> ParserResult {
        log::trace!("write_until_last_token");
        self.append(self.mark_pos(self.last_mark.unwrap()));
        Ok(())
    }

    pub(crate) fn skip_until_last_token(&mut self) -> ParserResult {
        log::trace!("skip_until_last_token");
        self.printed = self.mark_pos(self.last_mark.unwrap());
        Ok(())
    }

    pub(crate) fn skip_until_current_token(&mut self) -> ParserResult {
        log::trace!("skip_until_current_token");
        self.printed = self.mark_pos(self.mark.unwrap());
        Ok(())
    }

    fn mark_pos(&self, mark: Marker) -> usize {
        self.yaml[..mark.end().index()].trim_end().len()
    }

    pub(crate) fn append_str(&mut self, str: &'a str) {
        log::trace!("append_str: {}", str);
        if !str.is_empty() {
            self.clear_will_write();
            self.result.push(str);
        }
    }

    fn clear_will_write(&mut self) {
        if let Some((first, end)) = self.will_write.take() {
            self.result.push(&self.yaml[first..end.get()]);
        }
    }

    fn append(&mut self, index: usize) {
        if index == self.printed {
            return;
        }
        assert!(self.printed < index);
        let index = unsafe { NonZeroUsize::new_unchecked(index) };
        if let Some((first, end)) = self.will_write.as_mut() {
            if end.get() == self.printed {
                *end = unsafe {
                    NonZeroUsize::new_unchecked(end.get() + (index.get() - self.printed))
                };
            } else {
                self.result.push(&self.yaml[*first..end.get()]);
                self.will_write = Some((self.printed, index));
            }
        } else {
            self.will_write = Some((self.printed, index));
        }
        self.printed = index.get();
    }

    pub(crate) fn finish(mut self) -> Cow<'a, str> {
        self.append(self.yaml.len());
        self.clear_will_write();
        if self.result.len() == 1 {
            return Cow::Borrowed(self.result[0]);
        }
        log::trace!("realloc for finish");
        self.result.push(&self.yaml[self.printed..]);
        Cow::Owned(self.result.join(""))
    }
}
