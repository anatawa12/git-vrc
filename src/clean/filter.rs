use crate::clean::filter::ParserErr::EOF;
use crate::clean::ObjectReference;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::ControlFlow;
use std::ops::ControlFlow::Continue;
use std::str::Chars;
use yaml_rust::scanner::*;
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

struct Context<'a> {
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
        if mark.begin().index() == mark.end().index() {
            // it's position token
            self.yaml[..mark.begin().index()].trim_end().len()
        } else {
            // it's a token
            mark.end().index()
        }
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

pub(crate) fn filter_yaml(yaml: &str) -> ParserResult<Cow<str>> {
    assert!(!yaml.is_empty());
    let mut ctx = Context::new(&yaml);

    expect_token!(ctx.next()?, StreamStart(_));
    expect_token!(ctx.next()?, BlockMappingStart);
    expect_token!(ctx.next()?, Key);
    let object_type = ctx.next_scalar()?.0;
    expect_token!(ctx.next()?, Value);
    let omit_current_value = match object_type.as_str() {
        "MonoBehaviour" => mono_behaviour(&mut ctx)?,
        "PrefabInstance" => prefab_instance(&mut ctx)?,
        "RenderSettings" => render_settings(&mut ctx)?,
        _ => {
            // nothing to do fot this object. print all and return
            return Ok(yaml.into());
        }
    };

    if omit_current_value {
        return Ok("".into());
    }

    // closings
    assert!(matches!(ctx.next()?, BlockEnd), "MappingEnd expected");
    assert!(matches!(ctx.next()?, StreamEnd), "StreamEnd expected");

    Ok(ctx.finish().into())
}

/// MonoBehaviour
fn mono_behaviour(ctx: &mut Context) -> ParserResult<bool> {
    ctx.mapping(|ctx| {
        let name = ctx.next_scalar()?.0;
        expect_token!(ctx.next()?, Value);
        match name.as_str() {
            "serializedVersion" => {
                assert_eq!(ctx.next_scalar()?.0, "2", "unknown serializedVersion")
            }
            "serializedUdonProgramAsset" | "serializedProgramAsset" => {
                // for serializedUdonProgramAsset or serializedProgramAsset with mapping,
                // this tool assume the value as reference to SerializedUdonPrograms/<guid>.asset
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" {fileID: 0}");
                ctx.skip_until_current_token()?;
            }
            "DynamicMaterials" | "DynamicPrefabs" => {
                // DynamicMaterials or DynamicPrefabs of -17141911:661092b4961be7145bfbe56e1e62337b
                // (VRC_WorldDescriptor) is runtime (build-time) generated field so
                // it should not be tracked via git
                // https://github.com/anatawa12/git-vrc/issues/5
                ctx.write_until_current_token()?;
                ctx.append_str(" []");
                ctx.skip_next_value()?;
                ctx.skip_until_current_token()?;
            }
            _ => ctx.skip_next_value()?,
        }
        Ok(Continue(()))
    })
}

/// PrefabInstance
fn prefab_instance(ctx: &mut Context) -> ParserResult<bool> {
    ctx.mapping(|ctx| {
        let key = ctx.next_scalar()?.0;
        expect_token!(ctx.next()?, Value);
        match key.as_str() {
            "serializedVersion" => {
                assert_eq!(ctx.next_scalar()?.0, "2", "unknown serializedVersion")
            }
            "m_Modification" => prefab_instance_modification(ctx)?,
            _ => ctx.skip_next_value()?,
        }
        Ok(Continue(()))
    })
}

fn prefab_instance_modification(ctx: &mut Context) -> ParserResult {
    ctx.mapping(|ctx| {
        let key = ctx.next_scalar()?.0;
        expect_token!(ctx.next()?, Value);
        match key.as_str() {
            "m_Modifications" => prefab_instance_modifications_sequence(ctx)?,
            _ => ctx.skip_next_value()?,
        }
        Ok(Continue(()))
    })
}

fn prefab_instance_modifications_sequence(ctx: &mut Context) -> ParserResult {
    ctx.write_until_current_token()?;

    let mut some_written = false;

    ctx.sequence(|ctx| {
        let mut target: Option<ObjectReference> = None;
        let mut property_path: Option<String> = None;
        let mut value: Option<String> = None;
        let mut object_reference: Option<ObjectReference> = None;

        ctx.mapping(|ctx| {
            let key = ctx.next_scalar()?.0;
            expect_token!(ctx.next()?, Value);

            match key.as_str() {
                "target" => target = Some(ctx.parse_object_reference()?),
                "propertyPath" => property_path = Some(ctx.next_scalar()?.0),
                "value" => value = Some(ctx.next_scalar()?.0),
                "objectReference" => object_reference = Some(ctx.parse_object_reference()?),
                unknown => panic!("unknown key on PrefabInstance modifications: {}", unknown),
            }

            Ok(Continue(()))
        })?;

        // check if current modification is for keep or remove
        #[allow(unused_variables)]
        {
            let target = target.expect("target not specified in prefab modifications");
            let value = value.expect("value not specified in prefab modifications");
            let property_path =
                property_path.expect("propertyPath not specified in prefab modifications");
            let object_reference =
                object_reference.expect("objectReference not specified in prefab modifications");

            if should_omit(&property_path, &value, &object_reference) {
                // https://github.com/anatawa12/git-vrc/issues/5
                ctx.skip_until_last_token()?
            } else {
                some_written = true;
                ctx.write_until_last_token()?
            }
        }

        Ok(Continue(()))
    })?;

    if !some_written {
        ctx.skip_until_current_token()?;
        ctx.append_str(" []");
    }

    Ok(())
}

#[allow(unused_variables)]
fn should_omit(property_path: &str, value: &str, object_reference: &ObjectReference) -> bool {
    if property_path == "serializedProgramAsset" && value == "~" {
        return true;
    }
    if property_path.starts_with("DynamicMaterials.Array")
        || property_path.starts_with("DynamicPrefabs.Array")
    {
        // https://github.com/anatawa12/git-vrc/issues/5
        return true;
    }
    return false;
}

/// RenderSettings
fn render_settings(ctx: &mut Context) -> ParserResult<bool> {
    ctx.mapping(|ctx| {
        let name = ctx.next_scalar()?.0;
        expect_token!(ctx.next()?, Value);
        match name.as_str() {
            "m_IndirectSpecularColor" => {
                // for m_IndirectSpecularColor of m_IndirectSpecularColor,
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" {r: 0, g: 0, b: 0, a: 1}");
                ctx.skip_until_current_token()?;
            }
            _ => ctx.skip_next_value()?,
        }
        Ok(Continue(()))
    })
}

#[cfg(test)]
mod test_udon_program_asset {
    use super::*;

    #[test]
    fn udon_program_asset() -> anyhow::Result<()> {
        assert_eq!(filter_yaml(concat!(
        "MonoBehaviour:\n",
        "  m_ObjectHideFlags: 0\n",
        "  m_CorrespondingSourceObject: {fileID: 0}\n",
        "  m_PrefabInstance: {fileID: 0}\n",
        "  m_PrefabAsset: {fileID: 0}\n",
        "  m_GameObject: {fileID: 0}\n",
        "  m_Enabled: 1\n",
        "  m_EditorHideFlags: 0\n",
        "  m_Script: {fileID: 11500000, guid: 22203902d63dec94194fefc3e155c43b, type: 3}\n",
        "  m_Name: New Udon Assembly Program Asset\n",
        "  m_EditorClassIdentifier:\n",
        "  serializedUdonProgramAsset: {fileID: 11400000, guid: aa8a5233c74e54f108dfb136df564958,\n",
        "    type: 2}\n",
        "  udonAssembly:\n",
        "  assemblyError:\n",
        ))?, concat!(
        "MonoBehaviour:\n",
        "  m_ObjectHideFlags: 0\n",
        "  m_CorrespondingSourceObject: {fileID: 0}\n",
        "  m_PrefabInstance: {fileID: 0}\n",
        "  m_PrefabAsset: {fileID: 0}\n",
        "  m_GameObject: {fileID: 0}\n",
        "  m_Enabled: 1\n",
        "  m_EditorHideFlags: 0\n",
        "  m_Script: {fileID: 11500000, guid: 22203902d63dec94194fefc3e155c43b, type: 3}\n",
        "  m_Name: New Udon Assembly Program Asset\n",
        "  m_EditorClassIdentifier:\n",
        "  serializedUdonProgramAsset: {fileID: 0}\n",
        "  udonAssembly:\n",
        "  assemblyError:\n",
        ));
        Ok(())
    }
}

#[cfg(test)]
mod test_udon_behaviour {
    use super::*;

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        assert_eq!(filter_yaml(concat!(
        "MonoBehaviour:\n",
        "  m_ObjectHideFlags: 2\n",
        "  m_CorrespondingSourceObject: {fileID: 0}\n",
        "  m_PrefabInstance: {fileID: 0}\n",
        "  m_PrefabAsset: {fileID: 0}\n",
        "  m_GameObject: {fileID: 543750916}\n",
        "  m_Enabled: 1\n",
        "  m_EditorHideFlags: 0\n",
        "  m_Script: {fileID: 11500000, guid: 45115577ef41a5b4ca741ed302693907, type: 3}\n",
        "  m_Name:\n",
        "  m_EditorClassIdentifier:\n",
        "  interactTextPlacement: {fileID: 0}\n",
        "  interactText: Use\n",
        "  interactTextGO: {fileID: 0}\n",
        "  proximity: 2\n",
        "  SynchronizePosition: 0\n",
        "  AllowCollisionOwnershipTransfer: 0\n",
        "  Reliable: 0\n",
        "  _syncMethod: 2\n",
        "  serializedProgramAsset: {fileID: 11400000, guid: c6a719d47b234de46a0d92f561e78003,\n",
        "    type: 2}\n",
        "  programSource: {fileID: 11400000, guid: dcb91414824c30d4fbd7b30116027c36, type: 2}\n",
        "  serializedPublicVariablesBytesString: Ai8AAAAAATIAAABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlAFQAYQBiAGwAZQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AAAAAAAYBAAAAAAAAACcBBAAAAHQAeQBwAGUAAWgAAABTAHkAcwB0AGUAbQAuAEMAbwBsAGwAZQBjAHQAaQBvAG4AcwAuAEcAZQBuAGUAcgBpAGMALgBMAGkAcwB0AGAAMQBbAFsAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBJAG4AdABlAHIAZgBhAGMAZQBzAC4ASQBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AXQBdACwAIABtAHMAYwBvAHIAbABpAGIAAQEJAAAAVgBhAHIAaQBhAGIAbABlAHMALwEAAAABaAAAAFMAeQBzAHQAZQBtAC4AQwBvAGwAbABlAGMAdABpAG8AbgBzAC4ARwBlAG4AZQByAGkAYwAuAEwAaQBzAHQAYAAxAFsAWwBWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAEkAbgB0AGUAcgBmAGEAYwBlAHMALgBJAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlACwAIABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgBdAF0ALAAgAG0AcwBjAG8AcgBsAGkAYgABAAAABgMAAAAAAAAAAi8CAAAAAWEAAABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlAGAAMQBbAFsAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4ARwBhAG0AZQBPAGIAagBlAGMAdAAsACAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4AQwBvAHIAZQBNAG8AZAB1AGwAZQBdAF0ALAAgAFYAUgBDAC4AVQBkAG8AbgAuAEMAbwBtAG0AbwBuAAIAAAAGAgAAAAAAAAAnAQQAAAB0AHkAcABlAAEXAAAAUwB5AHMAdABlAG0ALgBTAHQAcgBpAG4AZwAsACAAbQBzAGMAbwByAGwAaQBiACcBCgAAAFMAeQBtAGIAbwBsAE4AYQBtAGUAAQYAAABlAG4AYQBiAGwAZQAnAQQAAAB0AHkAcABlAAEXAAAAUwB5AHMAdABlAG0ALgBPAGIAagBlAGMAdAAsACAAbQBzAGMAbwByAGwAaQBiAC0BBQAAAFYAYQBsAHUAZQAHBQIvAwAAAAFjAAAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQBgADEAWwBbAFUAbgBpAHQAeQBFAG4AZwBpAG4AZQAuAEcAYQBtAGUATwBiAGoAZQBjAHQAWwBdACwAIABVAG4AaQB0AHkARQBuAGcAaQBuAGUALgBDAG8AcgBlAE0AbwBkAHUAbABlAF0AXQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AAwAAAAYCAAAAAAAAACcBBAAAAHQAeQBwAGUAARcAAABTAHkAcwB0AGUAbQAuAFMAdAByAGkAbgBnACwAIABtAHMAYwBvAHIAbABpAGIAJwEKAAAAUwB5AG0AYgBvAGwATgBhAG0AZQABCAAAAGQAaQBzAGEAYgBsAGUAcwAnAQQAAAB0AHkAcABlAAEwAAAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4ARwBhAG0AZQBPAGIAagBlAGMAdABbAF0ALAAgAFUAbgBpAHQAeQBFAG4AZwBpAG4AZQAuAEMAbwByAGUATQBvAGQAdQBsAGUAAQEFAAAAVgBhAGwAdQBlAC8EAAAAATAAAABVAG4AaQB0AHkARQBuAGcAaQBuAGUALgBHAGEAbQBlAE8AYgBqAGUAYwB0AFsAXQAsACAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4AQwBvAHIAZQBNAG8AZAB1AGwAZQAEAAAABgAAAAAAAAAABwUHBQIvBQAAAAFJAAAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQBgADEAWwBbAFMAeQBzAHQAZQBtAC4ASQBuAHQAMwAyACwAIABtAHMAYwBvAHIAbABpAGIAXQBdACwAIABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAFAAAABgIAAAAAAAAAJwEEAAAAdAB5AHAAZQABFwAAAFMAeQBzAHQAZQBtAC4AUwB0AHIAaQBuAGcALAAgAG0AcwBjAG8AcgBsAGkAYgAnAQoAAABTAHkAbQBiAG8AbABOAGEAbQBlAAEfAAAAXwBfAF8AVQBkAG8AbgBTAGgAYQByAHAAQgBlAGgAYQB2AGkAbwB1AHIAVgBlAHIAcwBpAG8AbgBfAF8AXwAnAQQAAAB0AHkAcABlAAEWAAAAUwB5AHMAdABlAG0ALgBJAG4AdAAzADIALAAgAG0AcwBjAG8AcgBsAGkAYgAXAQUAAABWAGEAbAB1AGUAAgAAAAcFBwUHBQ==\n",
        "  publicVariablesUnityEngineObjects: []\n",
        "  publicVariablesSerializationDataFormat: 0\n",
        ))?, concat!(
        "MonoBehaviour:\n",
        "  m_ObjectHideFlags: 2\n",
        "  m_CorrespondingSourceObject: {fileID: 0}\n",
        "  m_PrefabInstance: {fileID: 0}\n",
        "  m_PrefabAsset: {fileID: 0}\n",
        "  m_GameObject: {fileID: 543750916}\n",
        "  m_Enabled: 1\n",
        "  m_EditorHideFlags: 0\n",
        "  m_Script: {fileID: 11500000, guid: 45115577ef41a5b4ca741ed302693907, type: 3}\n",
        "  m_Name:\n",
        "  m_EditorClassIdentifier:\n",
        "  interactTextPlacement: {fileID: 0}\n",
        "  interactText: Use\n",
        "  interactTextGO: {fileID: 0}\n",
        "  proximity: 2\n",
        "  SynchronizePosition: 0\n",
        "  AllowCollisionOwnershipTransfer: 0\n",
        "  Reliable: 0\n",
        "  _syncMethod: 2\n",
        "  serializedProgramAsset: {fileID: 0}\n",
        "  programSource: {fileID: 11400000, guid: dcb91414824c30d4fbd7b30116027c36, type: 2}\n",
        "  serializedPublicVariablesBytesString: Ai8AAAAAATIAAABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlAFQAYQBiAGwAZQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AAAAAAAYBAAAAAAAAACcBBAAAAHQAeQBwAGUAAWgAAABTAHkAcwB0AGUAbQAuAEMAbwBsAGwAZQBjAHQAaQBvAG4AcwAuAEcAZQBuAGUAcgBpAGMALgBMAGkAcwB0AGAAMQBbAFsAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBJAG4AdABlAHIAZgBhAGMAZQBzAC4ASQBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AXQBdACwAIABtAHMAYwBvAHIAbABpAGIAAQEJAAAAVgBhAHIAaQBhAGIAbABlAHMALwEAAAABaAAAAFMAeQBzAHQAZQBtAC4AQwBvAGwAbABlAGMAdABpAG8AbgBzAC4ARwBlAG4AZQByAGkAYwAuAEwAaQBzAHQAYAAxAFsAWwBWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAEkAbgB0AGUAcgBmAGEAYwBlAHMALgBJAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlACwAIABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgBdAF0ALAAgAG0AcwBjAG8AcgBsAGkAYgABAAAABgMAAAAAAAAAAi8CAAAAAWEAAABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAuAFUAZABvAG4AVgBhAHIAaQBhAGIAbABlAGAAMQBbAFsAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4ARwBhAG0AZQBPAGIAagBlAGMAdAAsACAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4AQwBvAHIAZQBNAG8AZAB1AGwAZQBdAF0ALAAgAFYAUgBDAC4AVQBkAG8AbgAuAEMAbwBtAG0AbwBuAAIAAAAGAgAAAAAAAAAnAQQAAAB0AHkAcABlAAEXAAAAUwB5AHMAdABlAG0ALgBTAHQAcgBpAG4AZwAsACAAbQBzAGMAbwByAGwAaQBiACcBCgAAAFMAeQBtAGIAbwBsAE4AYQBtAGUAAQYAAABlAG4AYQBiAGwAZQAnAQQAAAB0AHkAcABlAAEXAAAAUwB5AHMAdABlAG0ALgBPAGIAagBlAGMAdAAsACAAbQBzAGMAbwByAGwAaQBiAC0BBQAAAFYAYQBsAHUAZQAHBQIvAwAAAAFjAAAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQBgADEAWwBbAFUAbgBpAHQAeQBFAG4AZwBpAG4AZQAuAEcAYQBtAGUATwBiAGoAZQBjAHQAWwBdACwAIABVAG4AaQB0AHkARQBuAGcAaQBuAGUALgBDAG8AcgBlAE0AbwBkAHUAbABlAF0AXQAsACAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4AAwAAAAYCAAAAAAAAACcBBAAAAHQAeQBwAGUAARcAAABTAHkAcwB0AGUAbQAuAFMAdAByAGkAbgBnACwAIABtAHMAYwBvAHIAbABpAGIAJwEKAAAAUwB5AG0AYgBvAGwATgBhAG0AZQABCAAAAGQAaQBzAGEAYgBsAGUAcwAnAQQAAAB0AHkAcABlAAEwAAAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4ARwBhAG0AZQBPAGIAagBlAGMAdABbAF0ALAAgAFUAbgBpAHQAeQBFAG4AZwBpAG4AZQAuAEMAbwByAGUATQBvAGQAdQBsAGUAAQEFAAAAVgBhAGwAdQBlAC8EAAAAATAAAABVAG4AaQB0AHkARQBuAGcAaQBuAGUALgBHAGEAbQBlAE8AYgBqAGUAYwB0AFsAXQAsACAAVQBuAGkAdAB5AEUAbgBnAGkAbgBlAC4AQwBvAHIAZQBNAG8AZAB1AGwAZQAEAAAABgAAAAAAAAAABwUHBQIvBQAAAAFJAAAAVgBSAEMALgBVAGQAbwBuAC4AQwBvAG0AbQBvAG4ALgBVAGQAbwBuAFYAYQByAGkAYQBiAGwAZQBgADEAWwBbAFMAeQBzAHQAZQBtAC4ASQBuAHQAMwAyACwAIABtAHMAYwBvAHIAbABpAGIAXQBdACwAIABWAFIAQwAuAFUAZABvAG4ALgBDAG8AbQBtAG8AbgAFAAAABgIAAAAAAAAAJwEEAAAAdAB5AHAAZQABFwAAAFMAeQBzAHQAZQBtAC4AUwB0AHIAaQBuAGcALAAgAG0AcwBjAG8AcgBsAGkAYgAnAQoAAABTAHkAbQBiAG8AbABOAGEAbQBlAAEfAAAAXwBfAF8AVQBkAG8AbgBTAGgAYQByAHAAQgBlAGgAYQB2AGkAbwB1AHIAVgBlAHIAcwBpAG8AbgBfAF8AXwAnAQQAAAB0AHkAcABlAAEWAAAAUwB5AHMAdABlAG0ALgBJAG4AdAAzADIALAAgAG0AcwBjAG8AcgBsAGkAYgAXAQUAAABWAGEAbAB1AGUAAgAAAAcFBwUHBQ==\n",
        "  publicVariablesUnityEngineObjects: []\n",
        "  publicVariablesSerializationDataFormat: 0\n",
        ));
        Ok(())
    }

    #[test]
    fn prefab() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 9122363655180540528, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: serializedProgramAsset\n",
            "      value:\n",
            "      objectReference: {fileID: 11400000, guid: 7f6636ec3d2154e059e383d146a28a59,\n",
            "        type: 2}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications: []\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            )
        );
        Ok(())
    }
}

#[cfg(test)]
mod test_prefab_modifications {
    use super::*;

    #[test]
    fn with_other_modification_at_heading() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 690848371401817423, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: m_Name\n",
            "      value: GameObject\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 9122363655180540528, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: serializedProgramAsset\n",
            "      value:\n",
            "      objectReference: {fileID: 11400000, guid: 7f6636ec3d2154e059e383d146a28a59,\n",
            "        type: 2}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 690848371401817423, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: m_Name\n",
            "      value: GameObject\n",
            "      objectReference: {fileID: 0}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            )
        );
        Ok(())
    }

    #[test]
    fn with_other_modification_at_last() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 9122363655180540528, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: serializedProgramAsset\n",
            "      value:\n",
            "      objectReference: {fileID: 11400000, guid: 7f6636ec3d2154e059e383d146a28a59,\n",
            "        type: 2}\n",
            "    - target: {fileID: 690848371401817423, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: m_Name\n",
            "      value: GameObject\n",
            "      objectReference: {fileID: 0}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 690848371401817423, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: m_Name\n",
            "      value: GameObject\n",
            "      objectReference: {fileID: 0}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            )
        );
        Ok(())
    }

    #[test]
    fn without_other_modification() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 9122363655180540528, guid: 26db88bf250934ccca835bd9318c0eeb,\n",
            "        type: 3}\n",
            "      propertyPath: serializedProgramAsset\n",
            "      value:\n",
            "      objectReference: {fileID: 11400000, guid: 7f6636ec3d2154e059e383d146a28a59,\n",
            "        type: 2}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications: []\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            )
        );
        Ok(())
    }

    #[test]
    fn without_any_modification() -> anyhow::Result<()> {
        //simple_logger::init_with_level(log::Level::Trace)?;
        // TODO
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications: []\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications: []\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 26db88bf250934ccca835bd9318c0eeb, type: 3}\n",
            )
        );
        Ok(())
    }
}

#[cfg(test)]
mod test_dynamic_materials_and_prefab {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/5

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        assert_eq!(
            filter_yaml(concat!(
                "MonoBehaviour:\n",
                // many fields omitted
                "  useAssignedLayers: 0\n",
                "  DynamicPrefabs: \n",
                "  - {fileID: 2100000, guid: 3f13a5d1eb038764b804d1aabffed55f, type: 2}\n",
                "  - {fileID: 2100000, guid: 48f32ce8d7140f045a2c568df3a8d9bd, type: 2}\n",
                "  - {fileID: 2100000, guid: 09418b03dc9fc469f8d23aca7b180691, type: 2}\n",
                "  - {fileID: 2100000, guid: 43d0ae848fdfe6d4495a87f8e80e386b, type: 2}\n",
                "  - {fileID: 2100000, guid: c2af845bdfb561149b08ba13167ff040, type: 2}\n",
                "  - {fileID: 2180264, guid: 8f586378b4e144a9851e7b34d9b748ee, type: 2}\n",
                "  DynamicMaterials:\n",
                "  - {fileID: 2100000, guid: 3f13a5d1eb038764b804d1aabffed55f, type: 2}\n",
                "  - {fileID: 2100000, guid: 48f32ce8d7140f045a2c568df3a8d9bd, type: 2}\n",
                "  - {fileID: 2100000, guid: 09418b03dc9fc469f8d23aca7b180691, type: 2}\n",
                "  - {fileID: 2100000, guid: 43d0ae848fdfe6d4495a87f8e80e386b, type: 2}\n",
                "  - {fileID: 2100000, guid: c2af845bdfb561149b08ba13167ff040, type: 2}\n",
                "  - {fileID: 2180264, guid: 8f586378b4e144a9851e7b34d9b748ee, type: 2}\n",
                "  - {fileID: 2100000, guid: a59b4d20f3b324ca1aae5fd4f3942cf3, type: 2}\n",
                "  - {fileID: 2100000, guid: 9db9f48f3ee803d448488d4368a140f9, type: 2}\n",
                "  - {fileID: 2100000, guid: dd75a5d3bd47a0c489c0fd71aff39ede, type: 2}\n",
                "  - {fileID: 2100000, guid: 88aa935393607b6409baa45499f5156b, type: 2}\n",
                "  - {fileID: 2100000, guid: a393dafb2990e2c4fa0628ace4444efa, type: 2}\n",
                "  - {fileID: 2100000, guid: b24ed807dd7dc224baf5390f46738647, type: 2}\n",
                "  - {fileID: 2100000, guid: 254a177cd9c57e84683d0fd3bd1be46d, type: 2}\n",
                "  - {fileID: 10303, guid: 0000000000000000f000000000000000, type: 0}\n",
                "  - {fileID: 2100000, guid: e01134920adbcf549ac7f52ceeb583a2, type: 2}\n",
                "  - {fileID: 2100000, guid: 885a01c79ffd5024489a1fb31f3fffb5, type: 2}\n",
                "  - {fileID: 2100000, guid: 87529c80faca0ef4a881efba652815f3, type: 2}\n",
                "  - {fileID: 2100000, guid: 49c7ed6d767622b4fadcf200017fd44f, type: 2}\n",
                "  - {fileID: 2100000, guid: e86e7281176dae945bd655f34805ed55, type: 2}\n",
                "  - {fileID: 2100000, guid: 51d72acecdb1ba249957953415f8e29b, type: 2}\n",
                "  - {fileID: 2100000, guid: 419ae9fed5372564c995339c60fd7ebf, type: 2}\n",
                "  - {fileID: 2100000, guid: b3889ddf2a4bd9346a4843eb47e0acb1, type: 2}\n",
                "  - {fileID: 2100000, guid: 56778de2f4060f14fb06bc8cba7e30b7, type: 2}\n",
                "  - {fileID: 2100000, guid: 5b91c5c74862dba4d9fc2e8ae3e07b70, type: 2}\n",
                "  LightMapsNear: []\n",
                // many fields omitted
            ))?,
            concat!(
                "MonoBehaviour:\n",
                // many fields omitted
                "  useAssignedLayers: 0\n",
                "  DynamicPrefabs: []\n",
                "  DynamicMaterials: []\n",
                "  LightMapsNear: []\n",
                // many fields omitted
            ),
        );
        Ok(())
    }

    #[test]
    fn prefab() -> anyhow::Result<()> {
        assert_eq!(
            filter_yaml(concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications:\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicMaterials.Array.size\n",
            "      value: 3\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicMaterials.Array.data[0]\n",
            "      value: \n",
            "      objectReference: {fileID: 2100000, guid: 3e749d8edb4501f488bf37401bec19cf, type: 2}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicMaterials.Array.data[1]\n",
            "      value: \n",
            "      objectReference: {fileID: 10303, guid: 0000000000000000f000000000000000, type: 0}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicMaterials.Array.data[2]\n",
            "      value: \n",
            "      objectReference: {fileID: 10308, guid: 0000000000000000f000000000000000, type: 0}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicPrefabs.Array.size\n",
            "      value: 3\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicPrefabs.Array.data[0]\n",
            "      value: \n",
            "      objectReference: {fileID: 2100000, guid: 3e749d8edb4501f488bf37401bec19cf, type: 2}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicPrefabs.Array.data[1]\n",
            "      value: \n",
            "      objectReference: {fileID: 10303, guid: 0000000000000000f000000000000000, type: 0}\n",
            "    - target: {fileID: 6759095419728963412, guid: 8894fa7e4588a5c4fab98453e558847d,\n",
            "        type: 3}\n",
            "      propertyPath: DynamicPrefabs.Array.data[2]\n",
            "      value: \n",
            "      objectReference: {fileID: 10308, guid: 0000000000000000f000000000000000, type: 0}\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 8894fa7e4588a5c4fab98453e558847d, type: 3}\n",
            ))?,
            concat!(
            "PrefabInstance:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 2\n",
            "  m_Modification:\n",
            "    m_TransformParent: {fileID: 0}\n",
            "    m_Modifications: []\n",
            "    m_RemovedComponents: []\n",
            "  m_SourcePrefab: {fileID: 100100000, guid: 8894fa7e4588a5c4fab98453e558847d, type: 3}\n",
            ),
        );
        Ok(())
    }
}

mod test_render_settings {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/5

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        assert_eq!(
            filter_yaml(concat!(
            "RenderSettings:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 9\n",
            "  m_Fog: 0\n",
            "  m_FogColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_FogMode: 3\n",
            "  m_FogDensity: 0.01\n",
            "  m_LinearFogStart: 0\n",
            "  m_LinearFogEnd: 300\n",
            "  m_AmbientSkyColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientEquatorColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientGroundColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientIntensity: 1\n",
            "  m_AmbientMode: 0\n",
            "  m_SubtractiveShadowColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_SkyboxMaterial: {fileID: 10304, guid: 0000000000000000f000000000000000, type: 0}\n",
            "  m_HaloStrength: 0.5\n",
            "  m_FlareStrength: 1\n",
            "  m_FlareFadeSpeed: 3\n",
            "  m_HaloTexture: {fileID: 0}\n",
            "  m_SpotCookie: {fileID: 10001, guid: 0000000000000000e000000000000000, type: 0}\n",
            "  m_DefaultReflectionMode: 0\n",
            "  m_DefaultReflectionResolution: 128\n",
            "  m_ReflectionBounces: 1\n",
            "  m_ReflectionIntensity: 1\n",
            "  m_CustomReflection: {fileID: 0}\n",
            "  m_Sun: {fileID: 0}\n",
            "  m_IndirectSpecularColor: {r: 0.18028305, g: 0.22571313, b: 0.3069213, a: 1}\n",
            "  m_UseRadianceAmbientProbe: 0\n",
            // many fields omitted
            ))?,
            concat!(
            "RenderSettings:\n",
            "  m_ObjectHideFlags: 0\n",
            "  serializedVersion: 9\n",
            "  m_Fog: 0\n",
            "  m_FogColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_FogMode: 3\n",
            "  m_FogDensity: 0.01\n",
            "  m_LinearFogStart: 0\n",
            "  m_LinearFogEnd: 300\n",
            "  m_AmbientSkyColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientEquatorColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientGroundColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_AmbientIntensity: 1\n",
            "  m_AmbientMode: 0\n",
            "  m_SubtractiveShadowColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_SkyboxMaterial: {fileID: 10304, guid: 0000000000000000f000000000000000, type: 0}\n",
            "  m_HaloStrength: 0.5\n",
            "  m_FlareStrength: 1\n",
            "  m_FlareFadeSpeed: 3\n",
            "  m_HaloTexture: {fileID: 0}\n",
            "  m_SpotCookie: {fileID: 10001, guid: 0000000000000000e000000000000000, type: 0}\n",
            "  m_DefaultReflectionMode: 0\n",
            "  m_DefaultReflectionResolution: 128\n",
            "  m_ReflectionBounces: 1\n",
            "  m_ReflectionIntensity: 1\n",
            "  m_CustomReflection: {fileID: 0}\n",
            "  m_Sun: {fileID: 0}\n",
            "  m_IndirectSpecularColor: {r: 0, g: 0, b: 0, a: 1}\n",
            "  m_UseRadianceAmbientProbe: 0\n",
            ),
        );
        Ok(())
    }
}
