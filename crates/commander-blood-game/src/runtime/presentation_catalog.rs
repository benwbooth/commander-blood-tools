//! Mutable flat presentation-line names assembled from executable templates and DESCRIPT.

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{
    BLOODPRG_PRESENTATION_LINE_COUNT, BloodprgPresentationCatalog,
};
use commander_blood_formats::descript::DescriptCharacterBackground;

use crate::native::bloodprg::{DescriptPresentationAssets, PresentationResourceId};

use super::RuntimePresentationRequest;

const SEQUENCE_PRESENTATION_LINE: usize = 2;
const LOCATION_PRESENTATION_LINE: usize = 3;
const HYPERSPACE_PRESENTATION_LINE: usize = 6;
const SCRIPT_SEQUENCE_PRESENTATION_LINE: usize = 7;
const CHARACTER_IDLE_PRESENTATION_LINE: usize = 8;
const FIRST_CHARACTER_TALK_PRESENTATION_LINE: usize = 9;
const CHARACTER_TALK_PRESENTATION_LINE_COUNT: usize = 32;
const CHARACTER_RIGHT_PRESENTATION_LINE: usize = 39;
const CHARACTER_LEFT_PRESENTATION_LINE: usize = 40;
const OBJECT_PRESENTATION_LINE: usize = 43;

const SEQUENCE_RESOURCE_DIRECTORY: &[u8] = b"SQ\\";
const LOCATION_RESOURCE_DIRECTORY: &[u8] = b"PL\\";
const CHARACTER_RESOURCE_DIRECTORY: &[u8] = b"PE\\";
const OBJECT_RESOURCE_DIRECTORY: &[u8] = b"OB\\";

/// Scene background selected beside a character presentation line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimePresentationBackground {
    /// The presentation has no cached DESCRIPT background.
    None,
    /// One of the four DESCRIPT background slots is selected.
    Cached(commander_blood_formats::descript::DescriptBackgroundSlot),
}

impl From<DescriptCharacterBackground> for RuntimePresentationBackground {
    fn from(background: DescriptCharacterBackground) -> Self {
        match background {
            DescriptCharacterBackground::None => Self::None,
            DescriptCharacterBackground::Cached(slot) => Self::Cached(slot),
        }
    }
}

/// Runtime-owned presentation names after applying mutable DESCRIPT fields.
///
/// The native executable stored mutable names inside descriptor structs and
/// background IDs inside adjacent pointer words. This catalog represents both
/// as ordinary owned values indexed by the same 45 semantic line IDs.
pub struct RuntimePresentationCatalog {
    names: [Option<BloodResourceName>; BLOODPRG_PRESENTATION_LINE_COUNT],
    backgrounds: [RuntimePresentationBackground; BLOODPRG_PRESENTATION_LINE_COUNT],
    flags: [u8; BLOODPRG_PRESENTATION_LINE_COUNT],
    variants: [u8; BLOODPRG_PRESENTATION_LINE_COUNT],
}

impl RuntimePresentationCatalog {
    /// Clone fixed executable templates and clear every dynamically authored line.
    pub fn new(initial: &BloodprgPresentationCatalog) -> Self {
        let mut names =
            std::array::from_fn(|line| Some(initial.lines()[line].resource_name().clone()));
        for line in dynamic_presentation_lines() {
            names[line] = None;
        }
        Self {
            names,
            backgrounds: [RuntimePresentationBackground::None; BLOODPRG_PRESENTATION_LINE_COUNT],
            flags: std::array::from_fn(|line| initial.lines()[line].flags()),
            variants: std::array::from_fn(|line| initial.lines()[line].variant()),
        }
    }

    /// Replace all location, object, and character fields selected by one DESCRIPT record.
    pub fn apply_descript_assets(&mut self, assets: &DescriptPresentationAssets) -> Result<()> {
        self.names[LOCATION_PRESENTATION_LINE] = optional_prefixed_name(
            LOCATION_RESOURCE_DIRECTORY,
            assets.location_scene_video(),
            "location presentation",
        )?;
        self.names[CHARACTER_IDLE_PRESENTATION_LINE] = optional_prefixed_name(
            CHARACTER_RESOURCE_DIRECTORY,
            assets.idle_clip().map(|clip| clip.video().as_bytes()),
            "character idle presentation",
        )?;
        self.backgrounds[CHARACTER_IDLE_PRESENTATION_LINE] = assets
            .idle_clip()
            .map(|clip| clip.background().into())
            .unwrap_or(RuntimePresentationBackground::None);

        if assets.talk_clips().len() > CHARACTER_TALK_PRESENTATION_LINE_COUNT {
            bail!(
                "DESCRIPT has {} talk clips; presentation table holds {CHARACTER_TALK_PRESENTATION_LINE_COUNT}",
                assets.talk_clips().len()
            );
        }
        for line in FIRST_CHARACTER_TALK_PRESENTATION_LINE
            ..FIRST_CHARACTER_TALK_PRESENTATION_LINE + CHARACTER_TALK_PRESENTATION_LINE_COUNT
        {
            self.names[line] = None;
            self.backgrounds[line] = RuntimePresentationBackground::None;
        }
        // Opcodes 09/0A write these names before opcode-07 talk entries in every
        // shipped character record. Talk entries 31 and 32 intentionally alias
        // lines 39 and 40, so preserve that native last-write-wins ordering.
        self.names[CHARACTER_RIGHT_PRESENTATION_LINE] = optional_prefixed_name(
            CHARACTER_RESOURCE_DIRECTORY,
            assets.character_right_scene_video(),
            "right character presentation",
        )?;
        self.names[CHARACTER_LEFT_PRESENTATION_LINE] = optional_prefixed_name(
            CHARACTER_RESOURCE_DIRECTORY,
            assets.character_left_scene_video(),
            "left character presentation",
        )?;
        for (index, clip) in assets.talk_clips().iter().enumerate() {
            let line = FIRST_CHARACTER_TALK_PRESENTATION_LINE + index;
            self.names[line] = Some(prefixed_name(
                CHARACTER_RESOURCE_DIRECTORY,
                clip.video().as_bytes(),
                "character talk presentation",
            )?);
            self.backgrounds[line] = clip.background().into();
        }

        self.names[OBJECT_PRESENTATION_LINE] = optional_prefixed_name(
            OBJECT_RESOURCE_DIRECTORY,
            assets.object_scene_video(),
            "object presentation",
        )?;
        Ok(())
    }

    /// Select the current line-2 sequence emitted by a presentation DESCRIPT record.
    pub fn select_sequence_video(&mut self, basename: &[u8]) -> Result<()> {
        self.names[SEQUENCE_PRESENTATION_LINE] = Some(prefixed_name(
            SEQUENCE_RESOURCE_DIRECTORY,
            basename,
            "DESCRIPT sequence presentation",
        )?);
        Ok(())
    }

    /// Select the current line-6 hyperspace clip emitted by the camera coordinator.
    pub fn select_hyperspace_video(&mut self, basename: &[u8]) -> Result<()> {
        self.names[HYPERSPACE_PRESENTATION_LINE] = Some(prefixed_name(
            SEQUENCE_RESOURCE_DIRECTORY,
            basename,
            "hyperspace presentation",
        )?);
        Ok(())
    }

    /// Select the current line-7 HNM basename emitted by BloodScript A8.
    pub fn select_script_sequence_video(&mut self, basename: &[u8]) -> Result<()> {
        self.names[SCRIPT_SEQUENCE_PRESENTATION_LINE] = Some(prefixed_name(
            SEQUENCE_RESOURCE_DIRECTORY,
            basename,
            "BloodScript sequence presentation",
        )?);
        Ok(())
    }

    /// Build one request from the resolved name and executable-authored flags.
    pub fn request(&self, line: PresentationResourceId) -> Result<RuntimePresentationRequest> {
        let index = usize::from(line.get());
        let resource_name = self
            .names
            .get(index)
            .context("presentation line is outside the executable catalog")?
            .clone()
            .with_context(|| format!("presentation line {index} has no selected resource"))?;
        let mut request = RuntimePresentationRequest::new(resource_name);
        request.descriptor_flags = self.flags[index];
        request.variant = self.variants[index];
        Ok(request)
    }

    /// Return the scene background selected for one resolved line.
    pub fn background(
        &self,
        line: PresentationResourceId,
    ) -> Option<RuntimePresentationBackground> {
        self.backgrounds.get(usize::from(line.get())).copied()
    }

    /// Return the selected name without constructing a playback request.
    pub fn resource_name(&self, line: PresentationResourceId) -> Option<&BloodResourceName> {
        self.names
            .get(usize::from(line.get()))
            .and_then(Option::as_ref)
    }
}

fn dynamic_presentation_lines() -> impl Iterator<Item = usize> {
    [
        SEQUENCE_PRESENTATION_LINE,
        LOCATION_PRESENTATION_LINE,
        HYPERSPACE_PRESENTATION_LINE,
        SCRIPT_SEQUENCE_PRESENTATION_LINE,
        CHARACTER_IDLE_PRESENTATION_LINE,
        CHARACTER_RIGHT_PRESENTATION_LINE,
        CHARACTER_LEFT_PRESENTATION_LINE,
        OBJECT_PRESENTATION_LINE,
    ]
    .into_iter()
    .chain(
        FIRST_CHARACTER_TALK_PRESENTATION_LINE
            ..FIRST_CHARACTER_TALK_PRESENTATION_LINE + CHARACTER_TALK_PRESENTATION_LINE_COUNT,
    )
}

fn optional_prefixed_name(
    directory: &[u8],
    basename: Option<&[u8]>,
    context: &str,
) -> Result<Option<BloodResourceName>> {
    basename
        .map(|basename| prefixed_name(directory, basename, context))
        .transpose()
}

fn prefixed_name(directory: &[u8], basename: &[u8], context: &str) -> Result<BloodResourceName> {
    if basename.contains(&b'/') || basename.contains(&b'\\') {
        return BloodResourceName::new(basename).with_context(|| context.to_owned());
    }
    let mut path = Vec::with_capacity(directory.len() + basename.len());
    path.extend_from_slice(directory);
    path.extend_from_slice(basename);
    BloodResourceName::new(path).with_context(|| context.to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::native::bloodprg::{ScriptClock, TextPresentationState};
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths, RuntimeScriptBackend};

    use super::*;

    const OPENING_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(0);
    const FIRST_DYNAMIC_PRESENTATION_LINE: PresentationResourceId =
        PresentationResourceId::new(SEQUENCE_PRESENTATION_LINE as u16);
    type MissingAuthoredResource = (Box<[u8]>, Box<[u8]>);

    #[test]
    fn fixed_templates_are_retained_and_mutable_templates_start_unresolved() {
        let Some(data) = original_data() else {
            return;
        };
        let catalog = RuntimePresentationCatalog::new(data.presentation_catalog());
        let opening = catalog.request(OPENING_PRESENTATION_LINE).unwrap();
        assert_eq!(opening.resource_name.as_bytes(), b"sq\\mind.HNM");
        assert_eq!(opening.descriptor_flags, 0);
        assert_eq!(opening.variant, 16);
        assert!(catalog.request(FIRST_DYNAMIC_PRESENTATION_LINE).is_err());
    }

    #[test]
    fn every_shipped_descript_video_resolves_or_is_an_explicit_authored_defect() {
        let Some(data) = original_data() else {
            return;
        };
        let names: Vec<Box<[u8]>> = data
            .descript_database()
            .records()
            .iter()
            .map(|record| Box::from(record.name()))
            .collect();
        let mut backend = RuntimeScriptBackend::new(
            &data,
            ScriptClock {
                hour: 12,
                day: 1,
                month: 1,
            },
        );
        let mut catalog = RuntimePresentationCatalog::new(data.presentation_catalog());
        let mut text = TextPresentationState::default();
        let mut missing_resources: Vec<MissingAuthoredResource> = Vec::new();

        for name in names {
            backend
                .apply_description(&name, true, &mut text)
                .unwrap_or_else(|error| {
                    panic!("{} failed: {error:#}", String::from_utf8_lossy(&name))
                });
            let assets = backend.assets();
            catalog
                .apply_descript_assets(assets)
                .unwrap_or_else(|error| {
                    panic!("{} failed: {error:#}", String::from_utf8_lossy(&name))
                });

            let mut resolved_lines = vec![
                LOCATION_PRESENTATION_LINE,
                CHARACTER_IDLE_PRESENTATION_LINE,
                OBJECT_PRESENTATION_LINE,
            ];
            resolved_lines.extend(
                FIRST_CHARACTER_TALK_PRESENTATION_LINE
                    ..FIRST_CHARACTER_TALK_PRESENTATION_LINE
                        + CHARACTER_TALK_PRESENTATION_LINE_COUNT,
            );
            for line in resolved_lines {
                let line = PresentationResourceId::new(line as u16);
                let Some(resource_name) = catalog.resource_name(line) else {
                    continue;
                };
                if !data
                    .resource_store()
                    .resource_exists(resource_name)
                    .unwrap()
                {
                    missing_resources.push((name.clone(), Box::from(resource_name.as_bytes())));
                }
            }

            if name.as_ref() == b"Beauregard" {
                assert_eq!(
                    catalog
                        .resource_name(PresentationResourceId::new(
                            CHARACTER_RIGHT_PRESENTATION_LINE as u16,
                        ))
                        .unwrap()
                        .as_bytes(),
                    b"PE\\zhbol.hnm"
                );
                assert_eq!(
                    catalog
                        .resource_name(PresentationResourceId::new(
                            CHARACTER_LEFT_PRESENTATION_LINE as u16,
                        ))
                        .unwrap()
                        .as_bytes(),
                    b"PE\\zhbolmor.hnm"
                );
            }
            for sequence in assets.sequence_videos() {
                catalog.select_sequence_video(sequence.as_bytes()).unwrap();
                let resource_name = catalog
                    .resource_name(FIRST_DYNAMIC_PRESENTATION_LINE)
                    .unwrap();
                if !data
                    .resource_store()
                    .resource_exists(resource_name)
                    .unwrap()
                {
                    missing_resources.push((name.clone(), Box::from(resource_name.as_bytes())));
                }
            }
        }
        assert_eq!(
            missing_resources,
            vec![
                (Box::from(&b"year"[..]), Box::from(&b"SQ\\puven1.hnm"[..])),
                (
                    Box::from(&b"trompo"[..]),
                    Box::from(&b"PL\\maluss20.hnm"[..])
                ),
            ],
            "all unresolved names must be proven defects in the shipped DESCRIPT data"
        );
    }

    fn original_data() -> Option<OriginalGameData> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let roots = [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ];
        roots.into_iter().find_map(|root| {
            OriginalGameDataPaths::from_root(root)
                .ok()
                .and_then(|paths| {
                    OriginalGameData::load_with_writable_root(paths, temporary_root()).ok()
                })
        })
    }

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "commander-blood-presentation-catalog-test-{}",
            std::process::id()
        ))
    }
}
