use super::context::{Context, ParserResult};
use crate::clean::YamlSection;
use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::ControlFlow::Continue;
use yaml_rust::scanner::*;
use TokenType::*;

pub(in super::super) fn filter(sections: &mut [YamlSection]) -> ParserResult {
    let mut removed = HashSet::new();

    for x in sections.iter() {
        if x.filtered.is_empty() {
            removed.insert(x.parsed.file_id());
        }
    }

    for section in sections {
        if section.filtered.is_empty() {
            continue;
        }
        match &section.filtered {
            Cow::Borrowed(b) => {
                section.filtered = filter_yaml(&b, |id| removed.contains(&id))?;
            }
            Cow::Owned(o) => {
                section.filtered = match filter_yaml(&o, |id| removed.contains(&id))? {
                    Cow::Borrowed(b) => b.to_owned().into(),
                    Cow::Owned(o) => o.into(),
                }
            }
        }
    }
    Ok(())
}

fn filter_yaml(yaml: &str, is_removed: impl Fn(i64) -> bool) -> ParserResult<Cow<str>> {
    let mut ctx = Context::new(&yaml);

    expect_token!(ctx.next()?, StreamStart(_));
    expect_token!(ctx.next()?, BlockMappingStart);
    expect_token!(ctx.next()?, Key);
    let object_type = ctx.next_scalar()?.0;
    expect_token!(ctx.next()?, Value);
    let omit_current_value = match object_type.as_str() {
        "GameObject" => game_object(&mut ctx, is_removed)?,
        _ => {
            // nothing to do fot this object. print all and return
            return Ok(yaml.into());
        }
    };

    if omit_current_value {
        return Ok("".into());
    }

    // closings
    assert!(matches!(ctx.next()?, BlockEnd), "BlockEnd expected");
    assert!(matches!(ctx.next()?, StreamEnd), "StreamEnd expected");

    Ok(ctx.finish().into())
}

/// GameObject
fn game_object(ctx: &mut Context, is_removed: impl Fn(i64) -> bool) -> ParserResult<bool> {
    ctx.mapping(|ctx| {
        let name = ctx.next_scalar()?.0;
        expect_token!(ctx.next()?, Value);
        match name.as_str() {
            "serializedVersion" => match ctx.next_scalar()?.0.as_str() {
                "5" | "6" => {}
                v => panic!("unknown serializedVersion: {}", v),
            },
            "m_Component" => {
                ctx.write_until_current_token()?;
                // some elements must be written because Transform is required component
                ctx.sequence(|ctx| {
                    expect_token!(ctx.next()?, BlockMappingStart);
                    expect_token!(ctx.next()?, Key);
                    assert_eq!(ctx.next_scalar()?.0, "component");
                    expect_token!(ctx.next()?, Value);
                    let reference = ctx.parse_object_reference()?;
                    if reference.is_local() && is_removed(reference.file_id) {
                        ctx.skip_until_last_token()?
                    } else {
                        ctx.write_until_last_token()?
                    }
                    expect_token!(ctx.next()?, BlockEnd);
                    Ok(Continue(()))
                })?;
            }
            _ => ctx.skip_next_value()?,
        }
        Ok(Continue(()))
    })
}

#[test]
fn test() -> anyhow::Result<()> {
    assert_eq!(
        filter_yaml(
            concat!(
                "GameObject:\n",
                "  m_ObjectHideFlags: 0\n",
                "  m_CorrespondingSourceObject: {fileID: 0}\n",
                "  m_PrefabInstance: {fileID: 0}\n",
                "  m_PrefabAsset: {fileID: 0}\n",
                "  serializedVersion: 6\n",
                "  m_Component:\n",
                "  - component: {fileID: 423630531}\n",
                "  - component: {fileID: 423630534}\n",
                "  - component: {fileID: 423630533}\n",
                "  - component: {fileID: 423630532}\n",
                "  m_Layer: 0\n",
                "  m_Name: Text\n",
                "  m_TagString: Untagged\n",
                "  m_Icon: {fileID: 0}\n",
                "  m_NavMeshLayer: 0\n",
                "  m_StaticEditorFlags: 0\n",
                "  m_IsActive: 1",
            ),
            |id| id == 423630532
        )?,
        concat!(
            "GameObject:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  serializedVersion: 6\n",
            "  m_Component:\n",
            "  - component: {fileID: 423630531}\n",
            "  - component: {fileID: 423630534}\n",
            "  - component: {fileID: 423630533}\n",
            "  m_Layer: 0\n",
            "  m_Name: Text\n",
            "  m_TagString: Untagged\n",
            "  m_Icon: {fileID: 0}\n",
            "  m_NavMeshLayer: 0\n",
            "  m_StaticEditorFlags: 0\n",
            "  m_IsActive: 1",
        )
    );
    Ok(())
}
