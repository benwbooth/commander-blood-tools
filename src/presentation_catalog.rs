//! Symbolic names for the dialogue presentation table recovered from DESCRIPT.DES.
//!
//! DESCRIPT opcode 7 appends each character talk HNM to the native table in file
//! order. VM TEXT selector 0 addresses the first entry; BLOODPRG adds nine only
//! when forming its active-line ID. Keeping this catalog ordered therefore avoids
//! exposing either the byte selector or the biased native ID in BloodScript.

pub(crate) const TEXT_ONLY_PRESENTATION: &str = "text_only";
const TEXT_ONLY_ACTIVE_LINE_ID: i16 = crate::vm::DLG_LINE_ID_BIAS - 1;
const ULIKAN_AFTERNOON_LINE_ID: i16 = 22;
const ULIKAN_EVENING_LINE_ID: i16 = 23;
const ULIKAN_NIGHT_LINE_ID: i16 = 24;
const ULIKAN_MORNING_LINE_ID: i16 = 25;
const ULIKAN_EARLY_MORNING_LINE_ID: i16 = 26;

const ACTOR_PRESENTATIONS: &[(&str, &[&str])] = &[
    ("receiver", &["p_clefs.hnm"]),
    (
        "Rotator",
        &[
            "g_gara.hnm",
            "g_garag.hnm",
            "g_garb.hnm",
            "g_garc.hnm",
            "g_gard.hnm",
            "g_gare.hnm",
            "g_gareg.hnm",
            "g_garf.hnm",
            "g_garg.hnm",
            "g_gargg.hnm",
            "g_garh.hnm",
            "g_gari.hnm",
            "g_garj.hnm",
        ],
    ),
    (
        "Maziok",
        &[
            "ompb.hnm", "ompc.hnm", "ompd.hnm", "ompe.hnm", "ompf.hnm", "ompg.hnm", "omph.hnm",
            "ompi.hnm", "ompj.hnm", "ompk.hnm", "ompl.hnm",
        ],
    ),
    (
        "Outrageor",
        &[
            "r_pria.hnm",
            "r_prib.hnm",
            "r_pric.hnm",
            "r_prid.hnm",
            "r_prie.hnm",
            "r_prif.hnm",
            "r_prig.hnm",
            "r_prigg.hnm",
            "r_prih.hnm",
            "r_prii.hnm",
        ],
    ),
    (
        "Super_Tromp",
        &[
            "strag_tr.hnm",
            "stra_tr.hnm",
            "strb_tr.hnm",
            "strc_tr.hnm",
            "strd_tr.hnm",
            "stre_tr.hnm",
            "strf_tr.hnm",
            "strg_tr.hnm",
            "strh_tr.hnm",
        ],
    ),
    ("Betakam", &["pagotete.hnm", "pagotour.hnm"]),
    (
        "Bratakas",
        &[
            "trma_tr.hnm",
            "trmb_tr.hnm",
            "trmc_tr.hnm",
            "trme_tr.hnm",
            "trmf_tr.hnm",
            "trmg_tr.hnm",
            "trmh_tr.hnm",
            "trmi_tr.hnm",
        ],
    ),
    (
        "Anna_Haf",
        &[
            "cltr10_2.hnm",
            "cltr10_3.hnm",
            "cltr10_4.hnm",
            "cltr10_5.hnm",
            "cltr10_6.hnm",
            "cltr10_7.hnm",
            "cltr10_8.hnm",
            "cltr10_9.hnm",
            "cl10_10.hnm",
            "clt10_10.hnm",
            "cltr10_9.hnm@2",
        ],
    ),
    (
        "Hom",
        &[
            "homm_01.hnm",
            "homm_02.hnm",
            "homm_03.hnm",
            "homm_04.hnm",
            "homm_05.hnm",
            "homm_06.hnm",
            "homm_07.hnm",
            "homm_08.hnm",
            "homm_09.hnm",
            "homm_10.hnm",
            "homm_11.hnm",
            "homm_12.hnm",
            "homm_13.hnm",
            "homm_14.hnm",
            "homm_15.hnm",
            "homm_16.hnm",
            "homm_17.hnm",
        ],
    ),
    (
        "Kran_Dobu",
        &[
            "khran1_2.hnm",
            "khran1_3.hnm",
            "khran2_1.hnm",
            "khran2_2.hnm",
            "khran1_2.hnm@2",
            "khran2_3.hnm",
            "khran3_1.hnm",
            "khran3_2.hnm",
            "khran3_3.hnm",
            "khran4_1.hnm",
            "khran4_2.hnm",
        ],
    ),
    (
        "Yoko",
        &[
            "yok01.hnm",
            "yok02.hnm",
            "yok03.hnm",
            "yok04.hnm",
            "yok05.hnm",
            "yok06.hnm",
            "yok07.hnm",
            "yok08.hnm",
            "yok09.hnm",
            "yok10.hnm",
            "yok11.hnm",
            "yok12.hnm",
            "yok13.hnm",
            "yok14.hnm",
            "yok15.hnm",
            "yok16.hnm",
            "yok17.hnm",
            "yokn_tr.hnm",
            "yoko_tr.hnm",
            "yokp_tr.hnm",
            "yokq_tr.hnm",
        ],
    ),
    (
        "Eviscerator",
        &[
            "crl01.hnm",
            "crl02.hnm",
            "crl03.hnm",
            "crl04.hnm",
            "crl05.hnm",
            "crl06.hnm",
            "crl07.hnm",
            "crl08.hnm",
            "crl09.hnm",
            "crl10.hnm",
            "crl11.hnm",
            "crl12.hnm",
            "crl13.hnm",
        ],
    ),
    (
        "Emasculator",
        &[
            "v_crla.hnm",
            "v_croe.hnm",
            "v_crof.hnm",
            "v_crlag.hnm",
            "v_croj.hnm",
            "v_crol.hnm",
            "v_crod.hnm",
            "v_crob.hnm",
            "v_croi.hnm",
            "v_croc.hnm",
        ],
    ),
    (
        "Cyberquizz",
        &[
            "eye01.hnm",
            "eye02.hnm",
            "eye03.hnm",
            "eye04.hnm",
            "eye05.hnm",
            "eye06.hnm",
            "eye07.hnm",
            "eye08.hnm",
            "eye09.hnm",
            "eye10.hnm",
            "eyeer.hnm",
        ],
    ),
    (
        "Jerry_Khan",
        &[
            "jerry_1.hnm",
            "jerry_10.hnm",
            "jerry_11.hnm",
            "jerry_12.hnm",
            "jerry_13.hnm",
            "jerry_14.hnm",
            "jerry_15.hnm",
            "jerry_16.hnm",
        ],
    ),
    (
        "Morning_Oil",
        &[
            "morn01.hnm",
            "morn02.hnm",
            "morn03.hnm",
            "morn04.hnm",
            "morn05.hnm",
            "morn06.hnm",
            "morn07.hnm",
            "morn08.hnm",
        ],
    ),
    (
        "Super_Zen",
        &[
            "zen01.hnm",
            "zen02.hnm",
            "zen06.hnm",
            "zen07.hnm",
            "zen08.hnm",
            "zen03.hnm",
            "zen05.hnm",
            "zen09.hnm",
            "zen04.hnm",
            "zen12.hnm",
            "zenj_tr.hnm",
            "zenk_tr.hnm",
        ],
    ),
    (
        "Amigo",
        &[
            "amia_tr.hnm",
            "amib_tr.hnm",
            "amibg_tr.hnm",
            "amic_tr.hnm",
            "amicg_tr.hnm",
            "amid_tr.hnm",
            "amie_tr.hnm",
            "amif_tr.hnm",
            "amig_tr.hnm",
            "amih_tr.hnm",
            "amii_tr.hnm",
        ],
    ),
    (
        "Migrator",
        &[
            "miga_tr.hnm",
            "migb_tr.hnm",
            "migc_tr.hnm",
            "migd_tr.hnm",
            "mige_tr.hnm",
            "migf_tr.hnm",
            "migg_tr.hnm",
            "migh_tr.hnm",
            "migi_tr.hnm",
        ],
    ),
    (
        "Tina_Burner",
        &[
            "tina_tr.hnm",
            "tinb_tr.hnm",
            "tinc_tr.hnm",
            "tind_tr.hnm",
            "tine_tr.hnm",
            "tinf_tr.hnm",
            "ting_tr.hnm",
            "tinh_tr.hnm",
            "tini_tr.hnm",
            "tinj_tr.hnm",
        ],
    ),
    (
        "Tromp_la_Mort",
        &[
            "trma_tr.hnm",
            "trmb_tr.hnm",
            "trmc_tr.hnm",
            "trmd_tr.hnm",
            "trme_tr.hnm",
            "trmf_tr.hnm",
            "trmg_tr.hnm",
            "trmh_tr.hnm",
            "trmi_tr.hnm",
        ],
    ),
    (
        "Scruter_Mac",
        &[
            "scr01.hnm",
            "scr02.hnm",
            "scr03.hnm",
            "scr04.hnm",
            "scr05.hnm",
            "scr06.hnm",
            "scr07.hnm",
            "scr08.hnm",
            "scr09.hnm",
            "scr10.hnm",
            "scr11.hnm",
            "scr12.hnm",
            "scr13.hnm",
            "scr14.hnm",
            "scr15.hnm",
            "scr16.hnm",
            "scr17.hnm",
            "scr18.hnm",
            "scr19.hnm",
            "scr20.hnm",
            "scr21.hnm",
            "scr22.hnm",
        ],
    ),
    (
        "Scruter_Jo",
        &[
            "scr01.hnm",
            "scr02.hnm",
            "scr03.hnm",
            "scr04.hnm",
            "scr05.hnm",
            "scr06.hnm",
            "scr07.hnm",
            "scr08.hnm",
            "scr09.hnm",
            "scr10.hnm",
            "scr11.hnm",
            "scr12.hnm",
            "scr13.hnm",
            "scr14.hnm",
            "scr15.hnm",
            "scr16.hnm",
            "scr17.hnm",
            "scr18.hnm",
            "scr19.hnm",
            "scr20.hnm",
            "scr21.hnm",
            "scr22.hnm",
        ],
    ),
    (
        "Scruter_K",
        &[
            "mg_scr1.hnm",
            "mg_scr2.hnm",
            "mg_scr3.hnm",
            "scr01.hnm",
            "scr02.hnm",
            "scr03.hnm",
            "scr04.hnm",
            "scr05.hnm",
            "scr06.hnm",
            "scr07.hnm",
            "scr08.hnm",
            "scr09.hnm",
            "scr10.hnm",
            "scr11.hnm",
            "scr12.hnm",
            "scr13.hnm",
            "scr14.hnm",
            "scr15.hnm",
            "scr16.hnm",
            "scr17.hnm",
            "scr18.hnm",
            "scr19.hnm",
            "scr20.hnm",
            "scr21.hnm",
            "scr22.hnm",
        ],
    ),
    (
        "Daddy_Gluxx",
        &[
            "glu00.hnm",
            "glu01.hnm",
            "glu02.hnm",
            "glu03.hnm",
            "glu04.hnm",
            "glu05.hnm",
            "gluxfric.hnm",
            "gluxnon.hnm",
            "gluxpla.hnm",
            "gluxroug.hnm",
        ],
    ),
    (
        "Otto_Von_Smile",
        &[
            "doc01.hnm",
            "doc02.hnm",
            "doc03.hnm",
            "doc04.hnm",
            "doc05.hnm",
            "doc06.hnm",
            "doc07.hnm",
            "doc08.hnm",
        ],
    ),
    (
        "Maxxon",
        &[
            "maxa_tr.hnm",
            "maxb_tr.hnm",
            "maxc_tr.hnm",
            "maxd_tr.hnm",
            "maxe_tr.hnm",
            "maxf_tr.hnm",
            "maxg_tr.hnm",
            "maxh_tr.hnm",
            "maxi_tr.hnm",
            "maxj_tr.hnm",
            "maxk_tr.hnm",
            "maxl_tr.hnm",
            "maxm_tr.hnm",
            "maxn_tr.hnm",
            "maxo_tr.hnm",
            "maxp_tr.hnm",
            "maxq_tr.hnm",
            "maxr_tr.hnm",
            "maxs_tr.hnm",
            "maxt_tr.hnm",
        ],
    ),
    (
        "Bronko",
        &[
            "rgbtr1.hnm",
            "rgbtr2.hnm",
            "rgbtr3.hnm",
            "rgbtr4.hnm",
            "rgbtr5.hnm",
            "rgbtr6.hnm",
            "rgbtr7.hnm",
            "rgbtr8.hnm",
            "rgbtr9.hnm",
        ],
    ),
    (
        "Izwalito",
        &[
            "iswa1.hnm",
            "iswa.hnm",
            "iswb.hnm",
            "iswc.hnm",
            "iswd.hnm",
            "iswe.hnm",
            "iswf.hnm",
            "iswg.hnm",
            "iswh.hnm",
            "iswi.hnm",
            "iswj.hnm",
            "iswj1.hnm",
            "iswk.hnm",
            "iswk1.hnm",
            "iswx.hnm",
        ],
    ),
    (
        "Fifi",
        &[
            "ompa.hnm", "ompb.hnm", "ompc.hnm", "ompd.hnm", "ompe.hnm", "ompf.hnm", "ompg.hnm",
            "omph.hnm", "ompi.hnm", "ompj.hnm", "ompk.hnm", "ompl.hnm",
        ],
    ),
    (
        "Beauregard",
        &[
            "bor_a_tr.hnm",
            "bor_b_tr.hnm",
            "bor_c_tr.hnm",
            "bor_d_tr.hnm",
            "bor_e_tr.hnm",
            "borb_atr.hnm",
            "borb_btr.hnm",
            "borb_ctr.hnm",
            "borb_dtr.hnm",
            "borb_etr.hnm",
            "hboa.hnm",
            "hbob.hnm",
            "hboc.hnm",
            "hbod.hnm",
            "hboe.hnm",
            "hbof.hnm",
            "hbog.hnm",
            "hboh.hnm",
            "hboi.hnm",
            "hboi1.hnm",
            "hboj.hnm",
            "hbok.hnm",
            "hbol.hnm",
            "zhboa.hnm",
            "zhbob.hnm",
            "zhbod.hnm",
            "zhbof.hnm",
            "zhbog.hnm",
            "zhboh.hnm",
            "zhbok.hnm",
            "zhbol.hnm",
            "zhbolmor.hnm",
        ],
    ),
    (
        "Bob_Morlock",
        &[
            "boba.hnm",
            "bobb.hnm",
            "bobc.hnm",
            "bobd.hnm",
            "bobe.hnm",
            "bobf.hnm",
            "bobg.hnm",
            "bobh.hnm",
            "bobi.hnm",
            "bobj.hnm",
            "bobk.hnm",
            "bobr.hnm",
            "bobx.hnm",
            "borb_atr.hnm",
            "borb_btr.hnm",
            "borb_ctr.hnm",
            "borb_dtr.hnm",
            "borb_etr.hnm",
            "boba2.hnm",
        ],
    ),
    (
        "Bug_Deluxe",
        &[
            "sytr_1.hnm",
            "sytr_2.hnm",
            "sytr_3.hnm",
            "sytr_5.hnm",
            "sytr_6.hnm",
            "sytr_7.hnm",
            "sytr_8.hnm",
            "sytr_10.hnm",
            "sytr_11.hnm",
        ],
    ),
    (
        "Sinox",
        &[
            "sytr_1.hnm",
            "sytr_2.hnm",
            "sytr_3.hnm",
            "sytr_5.hnm",
            "sytr_6.hnm",
            "sytr_7.hnm",
            "sytr_8.hnm",
            "sytr_10.hnm",
            "sytr_11.hnm",
        ],
    ),
    (
        "ondoyant",
        &[
            "onda01.hnm",
            "onda03.hnm",
            "onda04.hnm",
            "onda05.hnm",
            "onda06.hnm",
            "onda07.hnm",
            "onda08.hnm",
            "onda09.hnm",
            "onda10.hnm",
            "onda11.hnm",
            "onda12.hnm",
            "onda13.hnm",
            "onda14.hnm",
            "onda15.hnm",
            "onda16.hnm",
            "onda17.hnm",
            "onda18.hnm",
            "onda19.hnm",
            "onda20.hnm",
            "onda21.hnm",
            "onda22.hnm",
        ],
    ),
];

const SPECIAL_PRESENTATIONS: &[(&str, &str, i16)] = &[
    ("Ulikan", "afternoon_signoff", ULIKAN_AFTERNOON_LINE_ID),
    ("Ulikan", "evening_signoff", ULIKAN_EVENING_LINE_ID),
    ("Ulikan", "night_signoff", ULIKAN_NIGHT_LINE_ID),
    ("Ulikan", "morning_signoff", ULIKAN_MORNING_LINE_ID),
    (
        "Ulikan",
        "early_morning_signoff",
        ULIKAN_EARLY_MORNING_LINE_ID,
    ),
];

pub(crate) fn symbolic_name(actor: &str, active_line_id: i16) -> Option<&'static str> {
    if active_line_id == TEXT_ONLY_ACTIVE_LINE_ID {
        return Some(TEXT_ONLY_PRESENTATION);
    }
    if let Some((_, name, _)) = SPECIAL_PRESENTATIONS
        .iter()
        .find(|(candidate, _, line)| *candidate == actor && *line == active_line_id)
    {
        return Some(name);
    }
    let index = usize::try_from(active_line_id.checked_sub(crate::vm::DLG_LINE_ID_BIAS)?).ok()?;
    actor_presentations(actor)?.get(index).copied()
}

pub(crate) fn active_line_id(actor: &str, name: &str) -> Option<i16> {
    if name == TEXT_ONLY_PRESENTATION {
        return Some(TEXT_ONLY_ACTIVE_LINE_ID);
    }
    if let Some((_, _, line)) = SPECIAL_PRESENTATIONS
        .iter()
        .find(|(candidate, candidate_name, _)| *candidate == actor && *candidate_name == name)
    {
        return Some(*line);
    }
    let index = actor_presentations(actor)?
        .iter()
        .position(|candidate| *candidate == name)?;
    i16::try_from(index)
        .ok()?
        .checked_add(crate::vm::DLG_LINE_ID_BIAS)
}

fn actor_presentations(actor: &str) -> Option<&'static [&'static str]> {
    ACTOR_PRESENTATIONS
        .iter()
        .find_map(|(candidate, presentations)| (*candidate == actor).then_some(*presentations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_names_round_trip_without_alias_collisions() {
        for (actor, presentations) in ACTOR_PRESENTATIONS {
            let mut names = std::collections::HashSet::new();
            for (index, name) in presentations.iter().enumerate() {
                assert!(
                    names.insert(*name),
                    "duplicate symbolic name {actor}.{name}"
                );
                let line = i16::try_from(index).unwrap() + crate::vm::DLG_LINE_ID_BIAS;
                assert_eq!(symbolic_name(actor, line), Some(*name));
                assert_eq!(active_line_id(actor, name), Some(line));
            }
        }
        for (actor, name, line) in SPECIAL_PRESENTATIONS {
            assert_eq!(symbolic_name(actor, *line), Some(*name));
            assert_eq!(active_line_id(actor, name), Some(*line));
        }
        assert_eq!(symbolic_name("Honk", 8), Some(TEXT_ONLY_PRESENTATION));
        assert_eq!(active_line_id("Honk", TEXT_ONLY_PRESENTATION), Some(8));
    }

    #[test]
    fn catalog_matches_descript_talk_hnms_in_native_table_order() {
        let path = std::path::Path::new("accuracy/cblood_install/cblood/DESCRIPT.DES");
        if !path.is_file() {
            return;
        }
        let database = crate::descript::DescriptDb::parse_file(path).unwrap();
        let mut compared = 0usize;
        for record in database
            .records
            .iter()
            .filter(|record| !record.talk_hnms.is_empty())
        {
            let expected = record
                .talk_hnms
                .iter()
                .scan(
                    std::collections::HashMap::<&str, usize>::new(),
                    |seen, media| {
                        let occurrence = seen.entry(&media.name).or_default();
                        *occurrence += 1;
                        Some(if *occurrence == 1 {
                            media.name.clone()
                        } else {
                            format!("{}@{}", media.name, occurrence)
                        })
                    },
                )
                .collect::<Vec<_>>();
            assert_eq!(
                actor_presentations(&record.name),
                Some(
                    expected
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .as_slice()
                ),
                "{}",
                record.name
            );
            compared += expected.len();
        }
        assert_eq!(compared, 448);
    }
}
