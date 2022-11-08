use super::super::ObjectReference;
use super::context::{Context, ParserResult};
use crate::clean::YamlSection;
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::ops::ControlFlow::{Break, Continue};
use yaml_rust::scanner::*;
use TokenType::*;

pub(in super::super) fn filter(sections: &mut [YamlSection]) -> ParserResult {
    for section in sections {
        match &section.filtered {
            Cow::Borrowed(b) => {
                section.filtered = filter_yaml(&b)?;
            }
            Cow::Owned(o) => {
                section.filtered = match filter_yaml(&o)? {
                    Cow::Borrowed(b) => b.to_owned().into(),
                    Cow::Owned(o) => o.into(),
                }
            }
        }
    }
    Ok(())
}

fn filter_yaml(yaml: &str) -> ParserResult<Cow<str>> {
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

lazy_static! {
    static ref PIPELINE_SAVER_REFERENCE: ObjectReference =
        ObjectReference::new(229740497, "4ecd63eff847044b68db9453ce219299".to_owned(), 3);
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
            "m_Script" => {
                let object_reference = ctx.parse_object_reference()?;
                if object_reference == *PIPELINE_SAVER_REFERENCE {
                    // PipelineSaver is short-time generated & will be removed on next save so
                    // remove this object immediately
                    // https://github.com/anatawa12/git-vrc/issues/3
                    return Ok(Break(true));
                }
            }
            "serializedUdonProgramAsset" | "serializedProgramAsset" => {
                // for serializedUdonProgramAsset or serializedProgramAsset with mapping,
                // this tool assume the value as reference to SerializedUdonPrograms/<guid>.asset
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" {fileID: 0}");
                ctx.skip_until_current_token()?;
            }
            "fallbackStatus" => {
                // fallbackStatus of PipelineManager is automatically computed.
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" 0");
                ctx.skip_until_current_token()?;
            }
            "animationHashSet" => {
                // animationHashSet of VRCAvatarDescriptor is automatically computed.
                // https://github.com/anatawa12/git-vrc/issues/13
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" []");
                ctx.skip_until_current_token()?;
            }
            "layerCollisionArr" => {
                // layerCollisionArr of VRC_SceneDescriptor is automatically computed.
                // https://github.com/anatawa12/git-vrc/issues/12
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                // 32 * 32 = 64 of bool
                ctx.append_str(concat!(
                    " ",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 4
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 8
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 12
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 16
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 20
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 24
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 28
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000", // 32
                ));
                ctx.skip_until_current_token()?;
            }
            "completedSDKPipeline" => {
                // completedSDKPipeline of PipelineManager is automatically computed.
                // https://github.com/anatawa12/git-vrc/issues/17
                ctx.write_until_current_token()?;
                ctx.skip_next_value()?;
                ctx.append_str(" 0");
                ctx.skip_until_current_token()?;
            }
            // baseAnimationLayers of VRCAvatarDescriptor
            "baseAnimationLayers" => mono_behaviour_base_animation_layers(ctx)?,
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

fn mono_behaviour_base_animation_layers(ctx: &mut Context) -> ParserResult {
    ctx.write_until_current_token()?;

    ctx.sequence(|ctx| {
        ctx.mapping(|ctx| {
            let key = ctx.next_scalar()?.0;
            expect_token!(ctx.next()?, Value);

            match key.as_str() {
                "mask" => {
                    // baseAnimationLayers[*].mask of VRCAvatarDescriptor
                    // https://github.com/anatawa12/git-vrc/issues/19
                    ctx.write_until_current_token()?;
                    ctx.skip_next_value()?;
                    ctx.append_str(" {fileID: 0}");
                    ctx.skip_until_current_token()?;
                }
                _ => ctx.skip_next_value()?,
            }

            Ok(Continue(()))
        })?;
        Ok(Continue(()))
    })?;
    Ok(())
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
    if property_path == "fallbackStatus" && object_reference.is_null() {
        return true;
    }
    if property_path == "layerCollisionArr" && object_reference.is_null() {
        // layerCollisionArr of VRC_SceneDescriptor is automatically computed.
        // https://github.com/anatawa12/git-vrc/issues/12
        return true;
    }
    if property_path == "completedSDKPipeline" && object_reference.is_null() {
        // completedSDKPipeline of PipelineManager is automatically computed.
        // https://github.com/anatawa12/git-vrc/issues/17
        return true;
    }
    if property_path.starts_with("DynamicMaterials.Array")
        || property_path.starts_with("DynamicPrefabs.Array")
        || property_path.starts_with("animationHashSet.Array")
    {
        // https://github.com/anatawa12/git-vrc/issues/5
        // https://github.com/anatawa12/git-vrc/issues/13
        return true;
    }
    if property_path.starts_with("baseAnimationLayers.Array.data[")
        && property_path.ends_with("].mask")
    {
        // baseAnimationLayers[*].mask of VRCAvatarDescriptor
        // https://github.com/anatawa12/git-vrc/issues/19
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

#[cfg(test)]
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

#[cfg(test)]
mod test_fallback_status {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/14

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        assert_eq!(
            filter_yaml(concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -1427037861, guid: 4ecd63eff847044b68db9453ce219299, type: 3}\n",
            "  m_Name: \n",
            "  m_EditorClassIdentifier: \n",
            "  launchedFromSDKPipeline: 0\n",
            "  completedSDKPipeline: 0\n",
            "  blueprintId: \n",
            "  contentType: 0\n",
            "  assetBundleUnityVersion: \n",
            "  fallbackStatus: 3\n",
            ))?,
            concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -1427037861, guid: 4ecd63eff847044b68db9453ce219299, type: 3}\n",
            "  m_Name: \n",
            "  m_EditorClassIdentifier: \n",
            "  launchedFromSDKPipeline: 0\n",
            "  completedSDKPipeline: 0\n",
            "  blueprintId: \n",
            "  contentType: 0\n",
            "  assetBundleUnityVersion: \n",
            "  fallbackStatus: 0\n",
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
            "    - target: {fileID: 973945594870973798, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: fallbackStatus\n",
            "      value: 3\n",
            "      objectReference: {fileID: 0}\n",
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

#[cfg(test)]
mod test_animation_hash_set {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/13

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        // many fields are omitted
        assert_eq!(
            filter_yaml(concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: 542108242, guid: 67cc4cb7839cd3741b63733d5adf0442, type: 3}\n",
            "  m_Name:\n",
            "  m_EditorClassIdentifier: \n",
            "  AnimationPreset: {fileID: 0}\n",
            "  animationHashSet:\n",
            "  - hash: 1127303507\n",
            "    name: Prone\n",
            "  - hash: -510014593\n",
            "    name: Standing\n",
            "  - hash: 1277114970\n",
            "    name: Crouching\n",
            "  - hash: -946967196\n",
            "    name: Supine\n",
            "  - hash: -1752920240\n",
            "    name: Left Side\n",
            "  - hash: -1868992157\n",
            "    name: Right Side\n",
            "  - hash: 1325891751\n",
            "    name: petan\n",
            "  - hash: -1840526229\n",
            "    name: SitSeiza\n",
            "  - hash: -2137589102\n",
            "    name: JumpAndFall.Short Fall\n",
            "  - hash: -862280380\n",
            "    name: JumpAndFall.HardLand\n",
            "  - hash: 2014011911\n",
            "    name: JumpAndFall.LongFall\n",
            "  - hash: 429917695\n",
            "    name: JumpAndFall.RestoreTracking\n",
            "  - hash: 196538068\n",
            "    name: JumpAndFall.SmallHop\n",
            "  - hash: -1551062404\n",
            "    name: JumpAndFall.RestoreToHop\n",
            "  - hash: -1176220201\n",
            "    name: JumpAndFall.QuickLand\n",
            "  - hash: 1619966539\n",
            "    name: Anchor Enabling\n",
            "  - hash: -676197288\n",
            "    name: Anchor Disabling\n",
            "  - hash: 606185156\n",
            "    name: LockPose Disabling\n",
            "  - hash: 2101233868\n",
            "    name: Anchor Enabled\n",
            "  - hash: -53190478\n",
            "    name: Anchor Disabled\n",
            "  - hash: -619011407\n",
            "    name: Reset\n",
            "  - hash: -1963515926\n",
            "    name: Fist\n",
            "  - hash: -1368897710\n",
            "    name: Point\n",
            "  - hash: 99779634\n",
            "    name: Open\n",
            "  - hash: 282634289\n",
            "    name: Peace\n",
            "  - hash: 935782528\n",
            "    name: RockNRoll\n",
            "  - hash: 420425210\n",
            "    name: Gun\n",
            "  - hash: 1949755150\n",
            "    name: Thumbs up\n",
            "  - hash: 2108076675\n",
            "    name: Idle\n",
            "  - hash: -998029940\n",
            "    name: Peace\n",
            "  - hash: 2050206447\n",
            "    name: Point\n",
            "  - hash: 297357552\n",
            "    name: Gun\n",
            "  - hash: -449848495\n",
            "    name: Open\n",
            "  - hash: -1652822560\n",
            "    name: Idle\n",
            "  - hash: 1781153929\n",
            "    name: Fist\n",
            "  - hash: -488267544\n",
            "    name: Thumbs up\n",
            "  - hash: -1592414362\n",
            "    name: RockNRoll\n",
            "  - hash: 1323185440\n",
            "    name: Remote\n",
            "  - hash: 1074085609\n",
            "    name: LocalCameraHidden\n",
            "  - hash: -2135209553\n",
            "    name: LocalCameraShown\n",
            "  - hash: -1640785050\n",
            "    name: LocalCameraGrabbingLeft\n",
            "  - hash: 1402342511\n",
            "    name: LocalCameraGrabbingRight\n",
            "  - hash: 1252093697\n",
            "    name: LocalCameraShowing\n",
            "  - hash: -1406174080\n",
            "    name: LocalCameraTouchingRight\n",
            "  - hash: 673776831\n",
            "    name: LocalCameraTouchingLeft\n",
            "  - hash: -1449862458\n",
            "    name: LocalCameraTouchingBoth\n",
            "  autoFootsteps: 1\n",
            ))?,
            concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: 542108242, guid: 67cc4cb7839cd3741b63733d5adf0442, type: 3}\n",
            "  m_Name:\n",
            "  m_EditorClassIdentifier: \n",
            "  AnimationPreset: {fileID: 0}\n",
            "  animationHashSet: []\n",
            "  autoFootsteps: 1\n",
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
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.size\n",
            "      value: 46\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[20].hash\n",
            "      value: -619011407\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[20].name\n",
            "      value: Reset\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[21].hash\n",
            "      value: -1963515926\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[21].name\n",
            "      value: Fist\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[22].hash\n",
            "      value: -1368897710\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[22].name\n",
            "      value: Point\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[23].hash\n",
            "      value: 99779634\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[23].name\n",
            "      value: Open\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[24].hash\n",
            "      value: 282634289\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[24].name\n",
            "      value: Peace\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[25].hash\n",
            "      value: 935782528\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[25].name\n",
            "      value: RockNRoll\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[26].hash\n",
            "      value: 420425210\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[26].name\n",
            "      value: Gun\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[27].hash\n",
            "      value: 1949755150\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[27].name\n",
            "      value: Thumbs up\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[28].hash\n",
            "      value: 2108076675\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[28].name\n",
            "      value: Idle\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[29].hash\n",
            "      value: -998029940\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[29].name\n",
            "      value: Peace\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[30].hash\n",
            "      value: 2050206447\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[30].name\n",
            "      value: Point\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[31].hash\n",
            "      value: 297357552\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[31].name\n",
            "      value: Gun\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[32].hash\n",
            "      value: -449848495\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[32].name\n",
            "      value: Open\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[33].hash\n",
            "      value: -1652822560\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[33].name\n",
            "      value: Idle\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[34].hash\n",
            "      value: 1781153929\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[34].name\n",
            "      value: Fist\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[35].hash\n",
            "      value: -488267544\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[35].name\n",
            "      value: Thumbs up\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[36].hash\n",
            "      value: -1592414362\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[36].name\n",
            "      value: RockNRoll\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[37].hash\n",
            "      value: 1323185440\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[37].name\n",
            "      value: Remote\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[38].hash\n",
            "      value: 1074085609\n",
            "      objectReference: {fileID: 0}\n",
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: animationHashSet.Array.data[38].name\n",
            "      value: LocalCameraHidden\n",
            "      objectReference: {fileID: 0}\n",
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

#[cfg(test)]
mod test_layer_collision_arr {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/12

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        // many fields are omitted
        assert_eq!(
            filter_yaml(concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 4306160767114150802}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -17141911, guid: 661092b4961be7145bfbe56e1e62337b, type: 3}\n",
            "  m_Name:\n",
            "  m_EditorClassIdentifier:\n",
            "  layerCollisionArr: 01010101010001010101010100010001010101010101010101010101010101010101010101000101010101010001000101010101010101010101010101010101010101010100010101010101000100010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101000101010101010001000101010101010101010101010101010101000000010000010100000000000000000000000000000101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010100010101010101000100010101010101010101010101010101010101010101010001010100000100000000000001010101010101010101010101010101010101000101010000010000000000000101010101010101010101010101010101010100010101010101000100010101010101010101010101010101010100000001000001010000000000000000000000000000010101010101010101010101010101000101010000010001010101010000000001010101010101010101000000010000010100000000000100000000000000000101010101010101010101010101010001010100000100010001010101010101010101010101010101010101010101000101010000010001000101010101010101010101010101010101010101010100010101000001000100010101010101010101010101010101010101010101010001010101010100000001010101010101010101010101010101010101010101000101010101010000000101010101010101010101010101010101010101010100010101010101000000010101010101010101010101010101010101010101010001010101010100000001010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101\n",
            ))?,
            concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 4306160767114150802}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -17141911, guid: 661092b4961be7145bfbe56e1e62337b, type: 3}\n",
            "  m_Name:\n",
            "  m_EditorClassIdentifier:\n",
            "  layerCollisionArr: 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\n",
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
            "    - target: {fileID: 973945594870973799, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: layerCollisionArr\n",
            "      value: 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\n",
            "      objectReference: {fileID: 0}\n",
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

#[cfg(test)]
mod test_completed_sdk_pipeline {
    use super::*;
    // see https://github.com/anatawa12/git-vrc/issues/17

    #[test]
    fn mono_behaviour() -> anyhow::Result<()> {
        assert_eq!(
            filter_yaml(concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -1427037861, guid: 4ecd63eff847044b68db9453ce219299, type: 3}\n",
            "  m_Name: \n",
            "  m_EditorClassIdentifier: \n",
            "  launchedFromSDKPipeline: 0\n",
            "  completedSDKPipeline: 1\n",
            "  blueprintId: \n",
            "  contentType: 0\n",
            "  assetBundleUnityVersion: \n",
            "  fallbackStatus: 0\n",
            ))?,
            concat!(
            "MonoBehaviour:\n",
            "  m_ObjectHideFlags: 0\n",
            "  m_CorrespondingSourceObject: {fileID: 0}\n",
            "  m_PrefabInstance: {fileID: 0}\n",
            "  m_PrefabAsset: {fileID: 0}\n",
            "  m_GameObject: {fileID: 973945594870973796}\n",
            "  m_Enabled: 1\n",
            "  m_EditorHideFlags: 0\n",
            "  m_Script: {fileID: -1427037861, guid: 4ecd63eff847044b68db9453ce219299, type: 3}\n",
            "  m_Name: \n",
            "  m_EditorClassIdentifier: \n",
            "  launchedFromSDKPipeline: 0\n",
            "  completedSDKPipeline: 0\n",
            "  blueprintId: \n",
            "  contentType: 0\n",
            "  assetBundleUnityVersion: \n",
            "  fallbackStatus: 0\n",
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
            "    - target: {fileID: 973945594870973798, guid: 27c023e317f775c45aca5b55f6eab077,\n",
            "        type: 3}\n",
            "      propertyPath: completedSDKPipeline\n",
            "      value: 1\n",
            "      objectReference: {fileID: 0}\n",
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
