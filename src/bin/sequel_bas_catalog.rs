//! Inventory separate BBB BAS resources without treating COD fallback as BAS.

use anyhow::{Context, Result, ensure};
use commander_blood_formats::bas::{ScriptBasInstruction, decode_script_bas};
use commander_blood_formats::instruction::ScriptTextWord;
use commander_blood_formats::script::{ScriptWordId, decode_script_dictionary};
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let root = std::path::PathBuf::from(
        args.next()
            .context("usage: sequel_bas_catalog <resource-root>")?,
    );
    ensure!(args.next().is_none(), "unexpected argument");
    let mut profiles = Vec::new();
    for profile in 1..=17 {
        let name = format!("SCRIPT{profile}");
        let bas = match std::fs::read(root.join(format!("{name}.BAS"))) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let dic = std::fs::read(root.join(format!("{name}.DIC")))?;
        let dictionary = decode_script_dictionary(&dic)?;
        let program = decode_script_bas(&bas, &dictionary)
            .with_context(|| format!("decoding {name}.BAS with its shipped dictionary"))?;
        ensure!(program.encode() == bas, "BAS round-trip mismatch");
        let word = |id: ScriptWordId| -> Result<serde_json::Value> {
            let bytes = dictionary.word(id).context("missing BAS word")?;
            Ok(json!({
                "dictionary_byte": dictionary.source_offset(id).context("missing BAS word offset")?,
                "source_bytes": bytes,
                "source": commander_blood_tools::font::cp437_string(bytes),
            }))
        };
        let mut sites = Vec::new();
        for token in program.tokens() {
            let data = match token.instruction() {
                ScriptBasInstruction::Menu(words) => json!({
                    "kind": "menu", "words": words.iter().copied().map(&word).collect::<Result<Vec<_>>>()?,
                }),
                ScriptBasInstruction::Text(text) => {
                    let mut words = Vec::new();
                    for item in &text.words {
                        words.push(match item {
                            ScriptTextWord::Dictionary(id) => word(*id)?,
                            ScriptTextWord::SectionSeparator => json!({"section_separator":true}),
                            ScriptTextWord::InventoryChoices => json!({"inventory_choices":true}),
                            ScriptTextWord::StateNumber(number) => {
                                json!({"state_byte":number.source_offset()})
                            }
                        });
                    }
                    json!({"kind":"text", "control":text.control.bits(), "words":words})
                }
                ScriptBasInstruction::TopicOffer(topic) => json!({
                    "kind":"topic", "word":topic.topic.map(&word).transpose()?,
                }),
                _ => continue,
            };
            sites.push(json!({"source_byte":token.source_offset().index(),"data":data}));
        }
        profiles.push(json!({
            "profile":name, "bas_sha256":format!("{:x}",Sha256::digest(&bas)),
            "dic_sha256":format!("{:x}",Sha256::digest(&dic)), "sites":sites,
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format":"bbb-bas-text-inventory-v1",
            "scope":"Separate BAS files decoded by the runtime parser; source bytes are authoritative, not the CP437 preview. No reachability claim.",
            "profiles":profiles,
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use commander_blood_formats::bas::ScriptBasError;
    use commander_blood_formats::code::ScriptCodeOffset;

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn shipped_separate_bas_does_not_resolve_against_its_dictionary() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("output/big-bug-bang/imported-assets/resources");
        let bas = std::fs::read(root.join("SCRIPT2.BAS")).unwrap();
        let dic = std::fs::read(root.join("SCRIPT2.DIC")).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bas)),
            "3e2b4a6d7c26aca6be2f88b3b539972b655ab1423907bbf75fb0931717bb5314"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(&dic)),
            "1666ae7bb0dead682f9c3fd64b5ea6b71999475beb77c85ad75e79a0ee71a5b0"
        );
        let dictionary = decode_script_dictionary(&dic).unwrap();
        assert_eq!(&dic[0x1efa..0x1f02], b"Private\0");
        assert_eq!(dictionary.resolve_source_offset(0x1f00), None);
        assert_eq!(
            decode_script_bas(&bas, &dictionary).unwrap_err(),
            ScriptBasError::InvalidMenuWord {
                source_offset: ScriptCodeOffset::new(6),
                dictionary_offset: 0x1f00,
            }
        );
    }
}
