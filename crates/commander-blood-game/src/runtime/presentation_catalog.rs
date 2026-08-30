//! Mutable flat presentation-line names assembled from executable templates and DESCRIPT.

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{
    BLOODPRG_PRESENTATION_LINE_COUNT, BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT,
    BloodprgPresentationCatalog,
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
    unclamped_line_ids: [u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT],
}

impl RuntimePresentationCatalog {
    /// Clone every executable template before DESCRIPT overwrites dynamic names.
    ///
    /// The shipped dynamic slots contain deliberately unavailable placeholder
    /// names. Retaining them reproduces the native soft resource-open failure
    /// when a script selects a slot before any DESCRIPT record has populated it.
    pub fn new(initial: &BloodprgPresentationCatalog) -> Self {
        Self {
            names: std::array::from_fn(|line| Some(initial.lines()[line].resource_name().clone())),
            backgrounds: [RuntimePresentationBackground::None; BLOODPRG_PRESENTATION_LINE_COUNT],
            flags: std::array::from_fn(|line| initial.lines()[line].flags()),
            variants: std::array::from_fn(|line| initial.lines()[line].variant()),
            unclamped_line_ids: *initial.unclamped_line_ids(),
        }
    }

    /// Apply filename slots written by one DESCRIPT record.
    ///
    /// `vm_c2_descript_lookup` resets its cursors and per-record counts, but it
    /// does not erase the fixed filename and talk-entry buffers. An absent
    /// opcode therefore leaves that presentation line bound to its prior
    /// resource.
    pub fn apply_descript_assets(&mut self, assets: &DescriptPresentationAssets) -> Result<()> {
        if let Some(video) = assets.location_scene_video() {
            self.names[LOCATION_PRESENTATION_LINE] = Some(prefixed_name(
                LOCATION_RESOURCE_DIRECTORY,
                video,
                "location presentation",
            )?);
        }
        if let Some(clip) = assets.idle_clip() {
            self.names[CHARACTER_IDLE_PRESENTATION_LINE] = Some(prefixed_name(
                CHARACTER_RESOURCE_DIRECTORY,
                clip.video().as_bytes(),
                "character idle presentation",
            )?);
            self.backgrounds[CHARACTER_IDLE_PRESENTATION_LINE] = clip.background().into();
        }

        if assets.talk_clips().len() > CHARACTER_TALK_PRESENTATION_LINE_COUNT {
            bail!(
                "DESCRIPT has {} talk clips; presentation table holds {CHARACTER_TALK_PRESENTATION_LINE_COUNT}",
                assets.talk_clips().len()
            );
        }
        // Opcodes 09/0A write these names before opcode-07 talk entries in every
        // shipped character record. Talk entries 31 and 32 intentionally alias
        // lines 39 and 40, so preserve that native last-write-wins ordering.
        if let Some(video) = assets.character_right_scene_video() {
            self.names[CHARACTER_RIGHT_PRESENTATION_LINE] = Some(prefixed_name(
                CHARACTER_RESOURCE_DIRECTORY,
                video,
                "right character presentation",
            )?);
        }
        if let Some(video) = assets.character_left_scene_video() {
            self.names[CHARACTER_LEFT_PRESENTATION_LINE] = Some(prefixed_name(
                CHARACTER_RESOURCE_DIRECTORY,
                video,
                "left character presentation",
            )?);
        }
        for (index, clip) in assets.talk_clips().iter().enumerate() {
            let line = FIRST_CHARACTER_TALK_PRESENTATION_LINE + index;
            self.names[line] = Some(prefixed_name(
                CHARACTER_RESOURCE_DIRECTORY,
                clip.video().as_bytes(),
                "character talk presentation",
            )?);
            self.backgrounds[line] = clip.background().into();
        }

        if let Some(video) = assets.object_scene_video() {
            self.names[OBJECT_PRESENTATION_LINE] = Some(prefixed_name(
                OBJECT_RESOURCE_DIRECTORY,
                video,
                "object presentation",
            )?);
        }
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

    /// Return the exact eight line IDs scanned by the recovered scene dispatcher.
    pub const fn unclamped_line_ids(&self) -> &[u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT] {
        &self.unclamped_line_ids
    }
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
    const EXECUTABLE_DYNAMIC_RESOURCE_PLACEHOLDER: &[u8] = b"xxxxxxxxxxxx";
    type MissingAuthoredResource = (Box<[u8]>, Box<[u8]>);

    #[test]
    fn fixed_and_placeholder_templates_retain_the_executable_names() {
        let Some(data) = original_data() else {
            return;
        };
        let catalog = RuntimePresentationCatalog::new(data.presentation_catalog());
        let opening = catalog.request(OPENING_PRESENTATION_LINE).unwrap();
        assert_eq!(opening.resource_name.as_bytes(), b"sq\\mind.HNM");
        assert_eq!(opening.descriptor_flags, 0);
        assert_eq!(opening.variant, 16);
        assert_eq!(
            catalog
                .request(FIRST_DYNAMIC_PRESENTATION_LINE)
                .unwrap()
                .resource_name
                .as_bytes(),
            b"sq\\xxxxxxxxxxxx"
        );
    }

    #[test]
    fn unrelated_descript_records_preserve_native_filename_slots() {
        let Some(data) = original_data() else {
            return;
        };
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

        backend
            .apply_description(b"Ondoya", true, &mut text)
            .unwrap()
            .unwrap();
        catalog.apply_descript_assets(backend.assets()).unwrap();
        let location = catalog.names[LOCATION_PRESENTATION_LINE].clone().unwrap();

        backend
            .apply_description(b"Scruter_Jo", true, &mut text)
            .unwrap()
            .unwrap();
        catalog.apply_descript_assets(backend.assets()).unwrap();
        let character = catalog.names[CHARACTER_IDLE_PRESENTATION_LINE]
            .clone()
            .unwrap();
        assert_eq!(
            catalog.names[LOCATION_PRESENTATION_LINE],
            Some(location.clone())
        );

        backend
            .apply_description(b"hat", true, &mut text)
            .unwrap()
            .unwrap();
        catalog.apply_descript_assets(backend.assets()).unwrap();
        let object = catalog.names[OBJECT_PRESENTATION_LINE].clone().unwrap();

        backend
            .apply_description(b"present", true, &mut text)
            .unwrap()
            .unwrap();
        catalog.apply_descript_assets(backend.assets()).unwrap();
        assert_eq!(catalog.names[LOCATION_PRESENTATION_LINE], Some(location));
        assert_eq!(
            catalog.names[CHARACTER_IDLE_PRESENTATION_LINE],
            Some(character)
        );
        assert_eq!(catalog.names[OBJECT_PRESENTATION_LINE], Some(object));
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
                if resource_name
                    .as_bytes()
                    .ends_with(EXECUTABLE_DYNAMIC_RESOURCE_PLACEHOLDER)
                {
                    continue;
                }
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
