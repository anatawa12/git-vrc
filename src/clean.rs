use log::{info, trace};
use std::fmt::Debug;
use std::io::stdin;
use std::io::Read;
use std::panic::catch_unwind;
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
        let mut parser = Parser::new(yaml.chars());

        let mut states = Vec::<Box<dyn EventReceiver>>::new();
        states.push(Box::new(root::Start));
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
        print!("{}", context.finish());
        Ok(())
    }
}

struct Context<'a> {
    reference: Option<ObjectReference>,
    printed: usize,
    yaml: &'a str,
    mark: Marker,
    last_mark: Marker,
    copy_disabled_next: bool,
    result: String,
}

impl<'a> Context<'a> {
    pub(crate) fn new(yaml: &'a str, (e, mark): (Event, Marker)) -> (Self, Event) {
        (
            Self {
                reference: None,
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
            self.result.push_str(&self.yaml[self.printed..mark.index()]);
            self.copy_disabled_next = false;
        }

        return e;
    }

    pub(crate) fn disable_copy_yaml_since_next_token(&mut self) {
        trace!("disable_copy_yaml_since_next_token");
        self.copy_disabled_next = true;
    }

    pub(crate) fn append_str(&mut self, str: &str) {
        trace!("append_str: {}", str);
        self.result.push_str(str);
    }

    pub(crate) fn enable_copy_yaml_since_last_token(&mut self) {
        trace!("enable_copy_yaml_since_last_token");
        self.printed = self.last_mark.index();
    }

    pub(crate) fn finish(mut self) -> String {
        self.result.push_str(&self.yaml[self.printed..]);
        self.result
    }

    pub(crate) fn reference(&mut self) -> ObjectReference {
        self.reference.take().expect("ObjectReference not parsed")
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct ObjectReference {
    file_id: String,
    guid: Option<String>,
    obj_type: u32,
}

impl ObjectReference {
    pub fn new(file_id: String, guid: String, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: Some(guid),
            obj_type,
        }
    }
    pub fn local(file_id: String, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: None,
            obj_type,
        }
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

/// generic states
mod generic {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct PostKey<R>(pub R);

    impl<R: EventReceiver + 'static> EventReceiver for PostKey<R> {
        fn on_event(self: Box<Self>, _ctx: &mut Context, _ev: &Event) -> ReceiveResult {
            next_and_push_with_same((*self).0, skip_value::SkipValue)
        }
    }
}

/// root (list of document) layer
mod root {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct Start;

    impl EventReceiver for Start {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if matches!(ev, StreamStart) {
                next(FileRoot)
            } else {
                panic!("no StreamStart at first")
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct FileRoot;

    impl EventReceiver for FileRoot {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                StreamEnd => pop_state(),
                DocumentStart => next_and_push(*self, document::Root),
                _ => panic!("unexpected state {:?}", ev),
            }
        }
    }
}

/// document (contains one mapping) layer
mod document {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct Root;

    impl EventReceiver for Root {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                MappingStart(_) => next_and_push(End, root_mapping::PreKey),
                _ => next_and_push(End, skip_value::SkipValue),
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct End;

    impl EventReceiver for End {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if !matches!(ev, DocumentEnd) {
                panic!("DocumentEnd expected")
            }
            pop_state()
        }
    }
}

/// root mapping layer (generic)
mod root_mapping {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct PreKey;

    impl EventReceiver for PreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None) if name == "MonoBehaviour" => {
                    next(MonoBehaviourPostKey)
                }
                _ => next_and_push_with_same(generic::PostKey(EndOfRoot), skip_value::SkipValue),
            }
        }
    }

    #[derive(Debug)]
    struct EndOfRoot;

    impl EventReceiver for EndOfRoot {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if !matches!(ev, MappingEnd) {
                panic!("MappingEnd expected")
            }
            pop_state()
        }
    }

    ////////////////////////////////////////////////

    #[derive(Debug)]
    struct MonoBehaviourPostKey;

    impl EventReceiver for MonoBehaviourPostKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                MappingStart(_) => next_and_push(EndOfRoot, mono_behaviour_mapping::PreKey),
                _ => panic!("the value of MonoBehaviour is not Mapping: {:?}", ev),
            }
        }
    }
}

/// MonoBehaviour mapping layer
mod mono_behaviour_mapping {
    use super::*;
    use log::warn;

    #[derive(Debug)]
    pub(crate) struct PreKey;

    impl EventReceiver for PreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None) if name == "m_Script" => {
                    next_and_push(PostMScriptKey, object_reference::Parse)
                }
                MappingEnd => pop_state(),
                _ => next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue),
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct PostMScriptKey;
    impl EventReceiver for PostMScriptKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, _ev: &Event) -> ReceiveResult {
            let script_refrence = _ctx.reference();
            next_with_same(PostMScriptPreKey(script_refrence))
        }
    }

    #[derive(Debug)]
    struct PostMScriptPreKey(ObjectReference);
    impl EventReceiver for PostMScriptPreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None)
                    if name == "serializedUdonProgramAsset" =>
                {
                    _ctx.disable_copy_yaml_since_next_token();
                    next_and_push(PostSerialized((*self).0), skip_value::SkipValue)
                }
                MappingEnd => pop_state(),
                _ => next_and_push_with_same(
                    generic::PostKey(PostMScriptPreKey((*self).0)),
                    skip_value::SkipValue,
                ),
            }
        }
    }

    #[derive(Debug)]
    struct PostSerialized(ObjectReference);
    impl EventReceiver for PostSerialized {
        fn on_event(self: Box<Self>, _ctx: &mut Context, _ev: &Event) -> ReceiveResult {
            _ctx.append_str("{fileID: 0");
            _ctx.enable_copy_yaml_since_last_token();
            next_with_same(PostMScriptPreKey((*self).0))
        }
    }
}

// region utilities
mod skip_value {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct SkipValue;

    impl EventReceiver for SkipValue {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Nothing => unreachable!(),
                StreamStart => unreachable!(),
                StreamEnd => unreachable!(),
                DocumentStart => unreachable!(),
                DocumentEnd => unreachable!(),
                Alias(_) => pop_state(),
                Scalar(_, _, _, _) => pop_state(),
                SequenceStart(_) => next(Sequence),
                SequenceEnd => unreachable!(),
                MappingStart(_) => next(MappingPreKey),
                MappingEnd => unreachable!(),
            }
        }
    }

    #[derive(Debug)]
    struct Sequence;
    impl EventReceiver for Sequence {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Nothing => unreachable!(),
                StreamStart => unreachable!(),
                StreamEnd => unreachable!(),
                DocumentStart => unreachable!(),
                DocumentEnd => unreachable!(),
                MappingEnd => unreachable!(),
                SequenceEnd => pop_state(),
                Alias(_) => next(*self),
                Scalar(_, _, _, _) => next(*self),
                SequenceStart(_) => next_and_push(*self, Sequence),
                MappingStart(_) => next_and_push(*self, MappingPreKey),
            }
        }
    }

    #[derive(Debug)]
    struct MappingPreKey;
    impl EventReceiver for MappingPreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Nothing => unreachable!(),
                StreamStart => unreachable!(),
                StreamEnd => unreachable!(),
                DocumentStart => unreachable!(),
                DocumentEnd => unreachable!(),
                SequenceEnd => unreachable!(),
                MappingEnd => pop_state(),
                Alias(_) => next(MappingPostKey),
                Scalar(_, _, _, _) => next(MappingPostKey),
                SequenceStart(_) => next_and_push(MappingPostKey, Sequence),
                MappingStart(_) => next_and_push(MappingPostKey, MappingPreKey),
            }
        }
    }

    #[derive(Debug)]
    struct MappingPostKey;
    impl EventReceiver for MappingPostKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Nothing => unreachable!(),
                StreamStart => unreachable!(),
                StreamEnd => unreachable!(),
                DocumentStart => unreachable!(),
                DocumentEnd => unreachable!(),
                SequenceEnd => unreachable!(),
                MappingEnd => unreachable!(),
                Alias(_) => next(MappingPreKey),
                Scalar(_, _, _, _) => next(MappingPreKey),
                SequenceStart(_) => next_and_push(MappingPreKey, Sequence),
                MappingStart(_) => next_and_push(MappingPreKey, MappingPreKey),
            }
        }
    }
}

mod object_reference {
    use super::*;
    use std::str::FromStr;

    #[derive(Debug)]
    pub(crate) struct Parse;

    impl EventReceiver for Parse {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if !matches!(ev, MappingStart(_)) {
                panic!("DocumentEnd expected")
            }
            next(PreKey(StateMain::default()))
        }
    }

    #[derive(Default, Debug)]
    struct StateMain {
        file_id: Option<String>,
        guid: Option<String>,
        obj_type: Option<u32>,
    }

    #[derive(Debug)]
    struct PreKey(StateMain);

    impl EventReceiver for PreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None) if name == "fileID" => {
                    next(PostFileIDKey((*self).0))
                }
                Scalar(name, TScalarStyle::Plain, _, None) if name == "guid" => {
                    next(PostGUIDKey((*self).0))
                }
                Scalar(name, TScalarStyle::Plain, _, None) if name == "type" => {
                    next(PostTypeKey((*self).0))
                }
                MappingEnd => {
                    _ctx.reference = Some(ObjectReference {
                        file_id: self.0.file_id.unwrap(),
                        guid: self.0.guid,
                        obj_type: self.0.obj_type.unwrap(),
                    });
                    pop_state()
                }
                _ => panic!("unexpected object reference key: {:?}", ev),
            }
        }
    }

    #[derive(Debug)]
    struct PostFileIDKey(StateMain);

    impl EventReceiver for PostFileIDKey {
        fn on_event(mut self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if let Scalar(id, TScalarStyle::Plain, _, None) = ev {
                self.0.file_id = Some(id.to_owned())
            } else {
                panic!("unexpected value of fileID: {:?}", ev)
            }
            next(PreKey((*self).0))
        }
    }

    #[derive(Debug)]
    struct PostGUIDKey(StateMain);

    impl EventReceiver for PostGUIDKey {
        fn on_event(mut self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if let Scalar(id, TScalarStyle::Plain, _, None) = ev {
                self.0.guid = Some(id.to_owned())
            } else {
                panic!("unexpected value of fileID: {:?}", ev)
            }
            next(PreKey((*self).0))
        }
    }

    #[derive(Debug)]
    struct PostTypeKey(StateMain);

    impl EventReceiver for PostTypeKey {
        fn on_event(mut self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            if let Scalar(id, TScalarStyle::Plain, _, None) = ev {
                self.0.obj_type = Some(u32::from_str(&id).unwrap())
            } else {
                panic!("unexpected value of fileID: {:?}", ev)
            }
            next(PreKey((*self).0))
        }
    }
}
// endregion
