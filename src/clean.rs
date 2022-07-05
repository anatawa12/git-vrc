use log::trace;
use std::any::Any;
use std::borrow::Cow;
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

        let initial_state: Box<dyn EventReceiver> = match object_type.as_str() {
            "MonoBehaviour" => Box::new(mono_behaviour_mapping::PreKey),
            "PrefabInstance" => Box::new(prefab_instance_mapping::PreKey),
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

    // write until last token. including current token with margin
    pub(crate) fn write_until_last_token(&mut self) {
        trace!("write_until_last_token");
        self.result.push_str(&self.yaml[self.printed..self.last_mark.begin().index()]);
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
    file_id: String,
    guid: Option<String>,
    obj_type: u32,
}

impl ObjectReference {
    #[allow(dead_code)]
    pub fn new(file_id: String, guid: String, obj_type: u32) -> Self {
        Self {
            file_id,
            guid: Some(guid),
            obj_type,
        }
    }
    #[allow(dead_code)]
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

/// MonoBehaviour mapping layer
mod mono_behaviour_mapping {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct PreKey;

    impl EventReceiver for PreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None)
                    if name == "serializedUdonProgramAsset" || name == "serializedProgramAsset" =>
                {
                    _ctx.write_until_current_token();
                    next_and_push(PostSerialized, skip_value::SkipValue)
                }
                MappingEnd => pop_state(),
                _ => next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue),
            }
        }
    }

    #[derive(Debug)]
    struct PostSerialized;
    impl EventReceiver for PostSerialized {
        fn on_event(self: Box<Self>, _ctx: &mut Context, _ev: &Event) -> ReceiveResult {
            _ctx.append_str("{fileID: 0}");
            _ctx.skip_until_last_token();
            next_with_same(PreKey)
        }
    }
}

/// PrefabInstance mapping layer
mod prefab_instance_mapping {
    use super::*;

    #[derive(Debug)]
    pub(crate) struct PreKey;

    impl EventReceiver for PreKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            match ev {
                Scalar(name, TScalarStyle::Plain, _, None) if name == "m_Modification" => {
                    next(PostModificationKey)
                }
                MappingEnd => pop_state(),
                _ => next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue),
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct PostModificationKey;
    impl EventReceiver for PostModificationKey {
        fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
            assert!(
                matches!(ev, MappingStart(_)),
                "MappingStart required but was {:?}",
                ev
            );
            next_and_push(PreKey, modifications::PreKey)
        }
    }

    mod modifications {
        use super::*;

        #[derive(Debug)]
        pub(crate) struct PreKey;

        impl EventReceiver for PreKey {
            fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
                match ev {
                    Scalar(name, TScalarStyle::Plain, _, None) if name == "m_Modifications" => {
                        _ctx.write_until_current_token();
                        next(PostModificationsKey)
                    }
                    MappingEnd => pop_state(),
                    _ => next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue),
                }
            }
        }

        #[derive(Debug)]
        pub(crate) struct PostModificationsKey;

        impl EventReceiver for PostModificationsKey {
            fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
                assert!(
                    matches!(ev, SequenceStart(_)),
                    "SequenceStart required but was {:?}",
                    ev
                );
                next_and_push(PreKey, ModificationSequence(false))
            }
        }

        #[derive(Debug)]
        // true if some element are written
        struct ModificationSequence(bool);

        impl EventReceiver for ModificationSequence {
            fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
                // TODO: pop values?
                // TODO: write?
                if matches!(ev, SequenceEnd) {
                    return pop_state();
                }
                assert!(
                    matches!(ev, MappingStart(_)),
                    "MappingStart required but was {:?}",
                    ev
                );
                _ctx.inter_state = Some(Box::new(false));
                next_and_push(
                    ModificationSequenceAfter((*self).0),
                    modification_mapping::PreKey,
                )
            }
        }

        #[derive(Debug)]
        struct ModificationSequenceAfter(bool);

        impl EventReceiver for ModificationSequenceAfter {
            fn on_event(mut self: Box<Self>, _ctx: &mut Context, _ev: &Event) -> ReceiveResult {
                if _ctx.bool() {
                    // わかったこと: MappingStart の marker は非ブロックだと:のいちになるので、次のトークンよりあとになる。謎仕様。
                    trace!("ModificationSequenceAfter:skip_until_last_token: {:?}", _ev);
                    // the last element is serializedProgramAsset
                    _ctx.skip_until_last_token()
                } else {
                    trace!(
                        "ModificationSequenceAfter:write_until_last_token: {:?}",
                        _ev
                    );
                    _ctx.write_until_last_token();
                    (*self).0 = true;
                }
                next_with_same(ModificationSequence((*self).0))
            }
        }

        mod modification_mapping {
            use super::*;

            #[derive(Debug)]
            pub(crate) struct PreKey;

            impl EventReceiver for PreKey {
                fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
                    match ev {
                        Nothing => unreachable!(),
                        StreamStart => unreachable!(),
                        StreamEnd => unreachable!(),
                        DocumentStart => unreachable!(),
                        DocumentEnd => unreachable!(),
                        SequenceEnd => unreachable!(),
                        MappingEnd => pop_state(),
                        Scalar(key, TScalarStyle::Plain, _, _) if key == "propertyPath" => {
                            next(PostPropertyPathKey)
                        }
                        Alias(_) => next(generic::PostKey(PreKey)),
                        Scalar(_, _, _, _) => next(generic::PostKey(PreKey)),
                        SequenceStart(_) => {
                            next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue)
                        }
                        MappingStart(_) => {
                            next_and_push_with_same(generic::PostKey(PreKey), skip_value::SkipValue)
                        }
                    }
                }
            }

            #[derive(Debug)]
            struct PostPropertyPathKey;

            impl EventReceiver for PostPropertyPathKey {
                fn on_event(self: Box<Self>, _ctx: &mut Context, ev: &Event) -> ReceiveResult {
                    if let Scalar(name, _, _, _) = ev {
                        if name == "serializedProgramAsset" {
                            _ctx.inter_state = Some(Box::new(true));
                            next(PreKey)
                        } else {
                            next(PreKey)
                        }
                    } else {
                        panic!("Scalar required")
                    }
                }
            }
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
                    _ctx.inter_state = Some(Box::new(ObjectReference {
                        file_id: self.0.file_id.unwrap(),
                        guid: self.0.guid,
                        obj_type: self.0.obj_type.unwrap(),
                    }));
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
