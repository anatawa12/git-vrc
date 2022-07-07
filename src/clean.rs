use log::trace;
use std::any::Any;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Debug;
use std::io::stdin;
use std::io::Read;
use yaml_rust::parser::*;
use yaml_rust::scanner::*;
use Event::*;

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
            print!("{}", heading);
            trace!("start: {}", heading);
            print!("{}", App::parse_one(body)?);
        }

        Ok(())
    }

    fn parse_one(yaml: &str) -> anyhow::Result<Cow<str>> {
        let mut parser = crate::yaml::YamlParser::new(yaml);

        assert!(
            matches!(parser.next()?, (StreamStart, _)),
            "StreamStart expected"
        );
        assert!(
            matches!(parser.next()?, (DocumentStart, _)),
            "DocumentStart expected"
        );
        assert!(
            matches!(parser.next()?, (MappingStart(_), _)),
            "MappingStart expected"
        );
        let object_type =
            if let (Scalar(object_type, TScalarStyle::Plain, _, _), _) = parser.next()? {
                object_type
            } else {
                panic!("Scalar Key to explain the type of value expected")
            };
        assert!(
            matches!(parser.next()?, (MappingStart(_), _)),
            "MappingStart expected"
        );

        let initial_state = match object_type.as_str() {
            _ => {
                // nothing to do fot this object. print all and return
                return Ok(yaml.into());
            }
        };

        let mut states = Vec::<Box<dyn EventReceiver>>::new();
        states.push(initial_state);
        let (mut context, mut e) = Context::new(&yaml, parser.next()?);

        while !states.is_empty() {
            if log::log_enabled!(log::Level::Trace) {
                trace!("");
                for x in &states {
                    trace!("stat:  {:?}", x);
                }
                trace!("token: {:?}, {:?}", e, context.mark);
            }

            match states.pop().unwrap().on_event(&mut context, &e) {
                ReceiveResult::Next(s) => {
                    states.push(s);
                    e = context.next_token(parser.next()?);
                }
                ReceiveResult::NextAndPush(s0, s1) => {
                    states.push(s0);
                    states.push(s1);
                    e = context.next_token(parser.next()?);
                }
                ReceiveResult::NextWithSame(s) => {
                    states.push(s);
                }
                ReceiveResult::NextAndPushWithSame(s0, s1) => {
                    states.push(s0);
                    states.push(s1);
                }
                ReceiveResult::PopState => {
                    e = context.next_token(parser.next()?);
                }
            }
        }

        // closings
        assert!(matches!(e, MappingEnd), "MappingEnd expected");
        assert!(
            matches!(parser.next()?, (DocumentEnd, _)),
            "DocumentEnd expected"
        );
        assert!(
            matches!(parser.next()?, (StreamEnd, _)),
            "StreamEnd expected"
        );

        Ok(context.finish().into())
    }
}

struct YamlSeparated<'a> {
    str: &'a str,
}

impl<'a> YamlSeparated<'a> {
    fn new(str: &'a str) -> Self {
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

struct Context<'a> {
    inter_state: Option<Box<dyn Any>>,
    printed: usize,
    yaml: &'a str,
    mark: Marker,
    last_mark: Marker,
    copy_disabled_next: bool,
    result: String,
}

type ParserResult<T = ()> = Result<T, Infallible>;

impl<'a> Context<'a> {
    pub(crate) fn new(yaml: &'a str, (e, mark): (Event, Marker)) -> (Self, Event) {
        (
            Self {
                inter_state: None,
                printed: 0,
                yaml,
                mark,
                last_mark: mark,
                copy_disabled_next: false,
                result: String::with_capacity(yaml.len()),
            },
            e,
        )
    }

    pub(crate) fn next_token(&mut self, (e, mark): (Event, Marker)) -> Event {
        self.last_mark = self.mark;
        self.mark = mark;

        if self.copy_disabled_next {
            self.result
                .push_str(&self.yaml[self.printed..mark.begin().index()]);
            self.printed = mark.begin().index();
            self.copy_disabled_next = false;
        }

        return e;
    }

    pub(crate) fn peek(&mut self) -> ParserResult<&Event> {
        todo!()
    }

    pub(crate) fn next(&mut self) -> ParserResult<Event> {
        todo!()
    }

    pub(crate) fn next_scalar(
        &mut self,
    ) -> ParserResult<(String, TScalarStyle, usize, Option<TokenType>)> {
        match self.next()? {
            Scalar(value, style, anchor_id, tag) => Ok((value, style, anchor_id, tag)),
            e => panic!("scalar expected but was: {:?}", e),
        }
    }

    // write until last token. including current token with margin
    pub(crate) fn write_until_last_token(&mut self) {
        trace!("write_until_last_token");
        self.result
            .push_str(&self.yaml[self.printed..self.last_mark.begin().index()]);
        self.printed = self.last_mark.begin().index();
    }

    // write until current token. including current token with margin
    pub(crate) fn write_until_current_token(&mut self) {
        trace!("write_until_current_token");
        self.copy_disabled_next = true;
    }

    pub(crate) fn append_str(&mut self, str: &str) {
        trace!("append_str: {}", str);
        self.result.push_str(str);
    }

    // skip until last token. skip including last token but excludes margin
    pub(crate) fn skip_until_last_token(&mut self) {
        trace!("skip_until_last_token");
        self.printed = self.last_mark.end().index();
    }

    pub(crate) fn finish(mut self) -> String {
        self.result.push_str(&self.yaml[self.printed..]);
        self.result
    }

    #[allow(dead_code)]
    pub(crate) fn reference(&mut self) -> ObjectReference {
        *self
            .inter_state
            .take()
            .and_then(|x| x.downcast().ok())
            .expect("ObjectReference not parsed")
    }

    pub(crate) fn bool(&mut self) -> bool {
        *self
            .inter_state
            .take()
            .and_then(|x| x.downcast().ok())
            .expect("ObjectReference not parsed")
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct ObjectReference {
    file_id: u64,
    guid: Option<String>,
    obj_type: u32,
}

impl ObjectReference {
    #[allow(dead_code)]
    pub fn new(file_id: u64, guid: String, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: Some(guid),
            obj_type,
        }
    }

    #[allow(dead_code)]
    pub fn local(file_id: u64, obj_type: u32) -> Self {
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

    pub fn is_null(&self) -> bool {
        return self.file_id == 0;
    }
}

trait StringOrStr {
    fn to_string(self) -> String;
}

impl StringOrStr for String {
    fn to_string(self) -> String {
        self
    }
}

impl StringOrStr for &str {
    fn to_string(self) -> String {
        self.to_owned()
    }
}

trait EventReceiver: Debug {
    fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult;
}

enum ReceiveResult {
    /// swaps current state with
    Next(Box<dyn EventReceiver>),
    /// swaps current state with first and pushes second state
    NextAndPush(Box<dyn EventReceiver>, Box<dyn EventReceiver>),

    /// same as Next but do current event will be used to call next state
    NextWithSame(Box<dyn EventReceiver>),
    /// same as NextAndPush but do current event will be used to call next state
    NextAndPushWithSame(Box<dyn EventReceiver>, Box<dyn EventReceiver>),

    PopState,
}

fn next(state: impl EventReceiver + 'static) -> ReceiveResult {
    ReceiveResult::Next(Box::new(state))
}

fn next_and_push(
    next: impl EventReceiver + 'static,
    push: impl EventReceiver + 'static,
) -> ReceiveResult {
    ReceiveResult::NextAndPush(Box::new(next), Box::new(push))
}

fn next_with_same(state: impl EventReceiver + 'static) -> ReceiveResult {
    ReceiveResult::NextWithSame(Box::new(state))
}

fn next_and_push_with_same(
    next: impl EventReceiver + 'static,
    push: impl EventReceiver + 'static,
) -> ReceiveResult {
    ReceiveResult::NextAndPushWithSame(Box::new(next), Box::new(push))
}

fn pop_state() -> ReceiveResult {
    ReceiveResult::PopState
}

/// MonoBehaviour
fn mono_behaviour(ctx: &mut Context) -> ParserResult {
    assert!(matches!(ctx.next()?, MappingStart(_)));
    loop {
        match ctx.next()? {
            MappingEnd => return Ok(()),
            Scalar(name, _, _, None) => match name.as_str() {
                "serializedVersion" => {
                    assert_eq!(ctx.next_scalar()?.0, "2", "unknown serializedVersion")
                }
                "serializedUdonProgramAsset" | "serializedProgramAsset"
                    if matches!(ctx.peek()?, MappingStart(_)) =>
                {
                    // for serializedUdonProgramAsset or serializedProgramAsset with mapping,
                    // this tool assume the value as reference to SerializedUdonPrograms/<guid>.asset
                    ctx.write_until_current_token();
                    skip_next_value(ctx)?;
                    ctx.append_str("{fileID: 0}");
                    ctx.skip_until_last_token();
                }
                _ => skip_next_value(ctx)?,
            },
            // skip value
            _ => skip_next_value(ctx)?,
        }
    }
}

/// PrefabInstance
fn prefab_instance(ctx: &mut Context) -> ParserResult {
    assert!(matches!(ctx.next()?, MappingStart(_)));
    loop {
        match ctx.next()? {
            MappingEnd => return Ok(()),
            Scalar(name, _, _, None) => match name.as_str() {
                "serializedVersion" => {
                    assert_eq!(ctx.next_scalar()?.0, "2", "unknown serializedVersion")
                }
                "m_Modification" => prefab_instance_modification(ctx)?,
                _ => skip_next_value(ctx)?,
            },
            // skip value
            _ => skip_next_value(ctx)?,
        }
    }
}

fn prefab_instance_modification(ctx: &mut Context) -> ParserResult {
    assert!(matches!(ctx.next()?, MappingStart(_)));
    loop {
        match ctx.next()? {
            MappingEnd => return Ok(()),
            Scalar(name, _, _, None) if name == "m_Modifications" => {
                prefab_instance_modifications_sequence(ctx)?
            }
            // skip value
            _ => skip_next_value(ctx)?,
        }
    }
}

fn prefab_instance_modifications_sequence(ctx: &mut Context) -> ParserResult {
    ctx.write_until_current_token();

    assert!(matches!(ctx.next()?, SequenceStart(_)));
    while let MappingStart(_) = ctx.peek()? {
        ctx.next()?;
        let mut target: Option<ObjectReference> = None;
        let mut property_path: Option<String> = None;
        let mut value: Option<String> = None;
        let mut object_reference: Option<ObjectReference> = None;
        while let Scalar(_, _, _, _) = ctx.peek()? {
            let (name, _, _, _) = ctx.next_scalar()?;
            match name.as_str() {
                "target" => target = Some(parse_object_reference(ctx)?),
                "propertyPath" => property_path = Some(ctx.next_scalar()?.0),
                "value" => value = Some(ctx.next_scalar()?.0),
                "objectReference" => object_reference = Some(parse_object_reference(ctx)?),
                unknown => panic!("unknown key on PrefabInstance modifications: {}", unknown),
            }
        }
        assert!(matches!(ctx.next()?, MappingEnd));

        // check if current modification is for keep or remove
        #[allow(unused_variables)]
        {
            let target = target.expect("target not specified in prefab modifications");
            let value = value.expect("value not specified in prefab modifications");
            let property_path =
                property_path.expect("propertyPath not specified in prefab modifications");
            let object_reference =
                object_reference.expect("objectReference not specified in prefab modifications");

            match property_path.as_str() {
                "serializedProgramAsset" if value.is_empty() => ctx.skip_until_last_token(),
                _ => ctx.write_until_last_token(),
            }
        }
    }
    assert!(matches!(ctx.next()?, SequenceEnd));
    Ok(())
}

// region utilities

fn skip_next_value(ctx: &mut Context) -> ParserResult {
    match ctx.next()? {
        Nothing => unreachable!(),
        StreamStart => unreachable!(),
        StreamEnd => unreachable!(),
        DocumentStart => unreachable!(),
        DocumentEnd => unreachable!(),
        Alias(_) => Ok(()),
        Scalar(_, _, _, _) => Ok(()),
        SequenceStart(_) => {
            while !matches!(ctx.peek()?, SequenceEnd) {
                skip_next_value(ctx)?;
            }
            assert!(matches!(ctx.next()?, SequenceEnd));
            Ok(())
        }
        SequenceEnd => unreachable!(),
        MappingStart(_) => {
            while !matches!(ctx.peek()?, MappingEnd) {
                // key and value
                skip_next_value(ctx)?;
                skip_next_value(ctx)?;
            }
            assert!(matches!(ctx.next()?, MappingEnd));
            Ok(())
        }
        MappingEnd => unreachable!(),
    }
}

fn parse_object_reference(ctx: &mut Context) -> ParserResult<ObjectReference> {
    assert!(matches!(ctx.next()?, MappingStart(_)));
    let mut file_id: Option<u64> = None;
    let mut guid: Option<String> = None;
    let mut object_type: Option<u32> = None;
    loop {
        match ctx.next()? {
            Nothing => unreachable!(),
            StreamStart => unreachable!(),
            StreamEnd => unreachable!(),
            DocumentStart => unreachable!(),
            DocumentEnd => unreachable!(),
            Alias(_) => unreachable!(),
            Scalar(s, _, _, _) => match s.as_str() {
                "fileID" => file_id = Some(ctx.next_scalar()?.0.parse().unwrap()),
                "guid" => guid = Some(ctx.next_scalar()?.0),
                "type" => object_type = Some(ctx.next_scalar()?.0.parse().unwrap()),
                unknown => panic!("unknown key for object reference: {}", unknown),
            },
            SequenceStart(_) => unreachable!(),
            SequenceEnd => unreachable!(),
            MappingStart(_) => unreachable!(),
            MappingEnd => break,
        }
    }
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
// endregion

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn udon_program_asset() -> anyhow::Result<()> {
        assert_eq!(App::parse_one(concat!(
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

    #[test]
    fn udon_behaviour() -> anyhow::Result<()> {
        assert_eq!(App::parse_one(concat!(
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
    fn prefab_with_other_modification_at_heading() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            App::parse_one(concat!(
        "--- !u!1001 &8809592113139104831\n",
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
        "--- !u!1001 &8809592113139104831\n",
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
    fn prefab_with_other_modification_at_last() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            App::parse_one(concat!(
        "--- !u!1001 &8809592113139104831\n",
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
        "--- !u!1001 &8809592113139104831\n",
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
    fn prefab_without_other_modification() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            App::parse_one(concat!(
        "--- !u!1001 &8809592113139104831\n",
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
        "--- !u!1001 &8809592113139104831\n",
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
    fn prefab_without_any_modification() -> anyhow::Result<()> {
        // TODO
        assert_eq!(
            App::parse_one(concat!(
            "--- !u!1001 &8809592113139104831\n",
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
            "--- !u!1001 &8809592113139104831\n",
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
