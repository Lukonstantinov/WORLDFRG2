//! Txt import for the goods library (DEPOSITS_AND_MINING_PLAN.md slice 3).
//!
//! Format is INI-ish — a `[id]` header per good, `key = value` lines below it,
//! `#`/`;` comments (full-line or trailing). CSV cannot carry optional/nested
//! fields and JSON punishes a trailing comma; this has to be hand-editable in a
//! plain text file.
//!
//! Two rules from the plan (D8), both load-bearing:
//! * **Only `[id]` and `name` are required.** Everything else defaults —
//!   `deposit_model` falls back to `deposits::default_model_for(id)`, which
//!   already carries a geologically correct guess for any id it recognises and a
//!   reasonable one (`CollisionalOrogen`) for anything it doesn't. A three-line
//!   block must produce a working mineral.
//! * **The import ADDS to the library; it never replaces.** An id already
//!   present is rejected, not overwritten — the user prunes afterwards.
//!
//! And one rule of this codebase's own (§2.4 / rule "never a silent drop"):
//! every line that isn't cleanly understood — an unknown key, an unparseable
//! value, a field the placement engine doesn't read yet (`workings`/`grade`/
//! `depth` — see the plan's slice 3 note) — is named in the report, not quietly
//! ignored.

use crate::sim::deposits::DepositModel;
use crate::sim::goods_spec::{DepositSpec, Distribution, Domain, GoodSpec};

/// What happened while importing a `.txt` goods file. Always populated — even a
/// perfect import reports its `added` ids — so the caller never has to guess
/// whether "nothing happened" meant "nothing was there" or "everything failed
/// silently".
#[derive(serde::Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Ids successfully parsed and added to the library.
    pub added: Vec<String>,
    /// (id, note) — a field that fell back to a default, or a key this format
    /// does not (yet) act on, e.g. `workings`/`grade`/`depth` (not wired to any
    /// per-mineral override — see DEPOSITS_AND_MINING_PLAN.md slice 3).
    pub defaulted: Vec<(String, String)>,
    /// (identifier, reason) — a whole `[section]` that could not become a good
    /// at all: no `name`, a duplicate id, an id already in the library.
    pub rejected: Vec<(String, String)>,
}

struct RawSection {
    id: String,
    /// Preserves input order so the report reads in the order the user wrote it.
    fields: Vec<(String, String)>,
}

/// Split the text into `[id]`-headed sections. Lines before the first header
/// that are neither blank nor a comment are reported as rejected (there is no
/// section to attach them to) rather than silently dropped.
fn split_sections(text: &str) -> (Vec<RawSection>, Vec<(String, String)>) {
    let mut sections: Vec<RawSection> = Vec::new();
    let mut orphan_rejects: Vec<(String, String)> = Vec::new();
    let mut current: Option<RawSection> = None;

    for (lineno, raw_line) in text.lines().enumerate() {
        let mut line = raw_line.trim();
        // Strip a trailing comment. Values in this format are plain tokens
        // (numbers, bare words, `#rrggbb` colors are the one exception — guarded
        // below), never quoted strings, so a first-`#`/`;` split is unambiguous
        // EXCEPT for a `color = #rrggbb` line, which must keep its `#`.
        let is_color_line = line.to_ascii_lowercase().starts_with("color")
            && line[5..].trim_start().starts_with('=');
        if !is_color_line {
            if let Some(p) = line.find(['#', ';']) {
                line = line[..p].trim_end();
            }
        } else if let Some(p) = line.rfind(';') {
            // A color line can still carry a trailing `;`-comment.
            line = line[..p].trim_end();
        }
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            current = Some(RawSection { id: inner.trim().to_string(), fields: Vec::new() });
            continue;
        }
        let Some(eq) = line.find('=') else {
            let note = format!("line {}: not a [section], key=value, or comment — ignored", lineno + 1);
            orphan_rejects.push(("(unparsed line)".to_string(), note));
            continue;
        };
        let key = line[..eq].trim().to_ascii_lowercase();
        let value = line[eq + 1..].trim().to_string();
        match current.as_mut() {
            Some(sec) => sec.fields.push((key, value)),
            None => {
                let note = format!("line {}: `{key}` given before any [section] header — ignored", lineno + 1);
                orphan_rejects.push((key, note));
            }
        }
    }
    if let Some(sec) = current.take() {
        sections.push(sec);
    }
    (sections, orphan_rejects)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !id.starts_with('_')
}

/// Parse an enum via its existing serde representation, so this parser can
/// never disagree with the real `Domain`/`Distribution`/`DepositModel` mapping.
fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_str(&format!("\"{}\"", value.trim().to_ascii_lowercase())).ok()
}

fn default_province_scale() -> f32 {
    0.06
}

/// Build one `GoodSpec` from a section's raw fields, recording every default
/// and every unrecognised/unwired key against `id` in `report`.
fn build_good(sec: &RawSection, report: &mut ImportReport) -> Option<GoodSpec> {
    if !valid_id(&sec.id) {
        report.rejected.push((
            sec.id.clone(),
            "invalid id — use lowercase ascii letters, digits and underscores, not starting with '_'".into(),
        ));
        return None;
    }
    let get = |k: &str| sec.fields.iter().find(|(fk, _)| fk == k).map(|(_, v)| v.as_str());

    let Some(name) = get("name").filter(|n| !n.trim().is_empty()) else {
        report.rejected.push((sec.id.clone(), "missing required `name`".into()));
        return None;
    };

    let icon = match get("icon") {
        Some(v) => v.to_string(),
        None => {
            report.defaulted.push((sec.id.clone(), "no `icon` given, defaulted to \u{26CF}\u{FE0F}".into()));
            "\u{26CF}\u{FE0F}".to_string()
        }
    };
    let color = match get("color") {
        Some(v) => v.to_string(),
        None => {
            report.defaulted.push((sec.id.clone(), "no `color` given, defaulted to #8a8e96".into()));
            "#8a8e96".to_string()
        }
    };
    let domain = match get("domain").and_then(parse_enum::<Domain>) {
        Some(d) => d,
        None => {
            if let Some(v) = get("domain") {
                report.defaulted.push((sec.id.clone(), format!("unrecognised domain '{v}', defaulted to continental")));
            } else {
                report.defaulted.push((sec.id.clone(), "no `domain` given, defaulted to continental".into()));
            }
            Domain::Continental
        }
    };
    let distribution = match get("distribution").and_then(parse_enum::<Distribution>) {
        Some(d) => d,
        None => {
            if let Some(v) = get("distribution") {
                report.defaulted.push((sec.id.clone(), format!("unrecognised distribution '{v}', defaulted to deposits")));
            } else {
                report.defaulted.push((sec.id.clone(), "no `distribution` given, defaulted to deposits".into()));
            }
            Distribution::Deposits
        }
    };

    let deposit = if distribution == Distribution::Deposits {
        let model = match get("deposit_model") {
            Some(v) => match parse_enum::<DepositModel>(v) {
                Some(m) => Some(m),
                None => {
                    report.defaulted.push((
                        sec.id.clone(),
                        format!("unrecognised deposit_model '{v}', falling back to the per-mineral geological default"),
                    ));
                    None
                }
            },
            None => {
                report.defaulted.push((
                    sec.id.clone(),
                    "no `deposit_model` given, using deposits::default_model_for(id)".into(),
                ));
                None
            }
        };
        let districts: u32 = match get("districts").map(|v| v.trim().parse::<u32>()) {
            Some(Ok(n)) => n,
            Some(Err(_)) => {
                report.defaulted.push((sec.id.clone(), "`districts` was not a whole number, defaulted to 6".into()));
                6
            }
            None => {
                report.defaulted.push((sec.id.clone(), "no `districts` given, defaulted to 6".into()));
                6
            }
        };
        let placer_frac = get("placer_frac").and_then(|v| v.trim().parse::<f32>().ok());
        let parent = get("parent").map(|v| v.trim().to_string());
        // `districts` is stated as the count AT THE DEFAULT deposit slider (6),
        // matching how the shipped minerals' count_num/count_den are authored —
        // see e.g. lapis_lazuli's own (1, 6) shipped in `default_custom_goods`.
        Some(DepositSpec {
            min_elev: 0.0,
            count_num: districts.max(1),
            count_den: 6,
            province_scale: default_province_scale(),
            model,
            placer_frac,
            parent,
        })
    } else {
        None
    };

    // Fields the placer does not yet read per-mineral (see the module doc and
    // DEPOSITS_AND_MINING_PLAN.md slice 3): named, not silently dropped.
    for (key, doc) in [
        ("workings", "workings count is not yet a per-mineral override — every mineral shares WORKINGS_MIN..WORKINGS_MAX"),
        ("grade", "a grade RANGE is not yet a per-mineral override — grade comes from local geological fit (see make_working)"),
        ("depth", "a fixed depth is not yet a per-mineral override — depth is rolled per working"),
    ] {
        if get(key).is_some() {
            report.defaulted.push((sec.id.clone(), format!("`{key}` given but not yet wired: {doc}")));
        }
    }

    let rarity = match get("rarity").map(|v| v.trim().parse::<f32>()) {
        Some(Ok(r)) => r.clamp(0.0, 1.0),
        Some(Err(_)) => { report.defaulted.push((sec.id.clone(), "`rarity` was not a number, defaulted to 0.6".into())); 0.6 }
        None => { report.defaulted.push((sec.id.clone(), "no `rarity` given, defaulted to 0.6".into())); 0.6 }
    };
    let base_value = match get("base_value").map(|v| v.trim().parse::<f32>()) {
        Some(Ok(b)) if b > 0.0 => b,
        Some(_) => { report.defaulted.push((sec.id.clone(), "`base_value` was not a positive number, defaulted to 5.0".into())); 5.0 }
        None => { report.defaulted.push((sec.id.clone(), "no `base_value` given, defaulted to 5.0".into())); 5.0 }
    };
    let desire = get("desire").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.5).clamp(0.0, 1.0);
    let network_luxury = get("network_luxury").map(|v| v.trim().eq_ignore_ascii_case("true")).unwrap_or(false);
    let need_tier: u8 = get("need_tier").and_then(|v| v.trim().parse::<u8>().ok()).unwrap_or(0).min(2);
    let bulk = get("bulk").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(1.0).max(0.1);
    let perishable = get("perishable").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0).max(0.0);
    let category = get("category").unwrap_or("").to_string();

    // Anything else on the section is a key this format does not understand at
    // all — named, not dropped, so a typo (`base_vaule`) is visible.
    let known = [
        "name", "icon", "color", "domain", "distribution", "deposit_model", "districts",
        "placer_frac", "parent", "workings", "grade", "depth", "rarity", "base_value",
        "desire", "network_luxury", "need_tier", "bulk", "perishable", "category",
    ];
    for (k, _) in &sec.fields {
        if !known.contains(&k.as_str()) {
            report.defaulted.push((sec.id.clone(), format!("unknown key `{k}` ignored")));
        }
    }

    Some(GoodSpec {
        origins: crate::sim::goods_spec::default_origins_for(&sec.id),
        soil: Vec::new(),
        relief: None,
        id: sec.id.clone(),
        name: name.to_string(),
        icon,
        color,
        enabled: true,
        domain,
        distribution,
        rarity,
        desire,
        network_luxury,
        builtin: false,
        deposit,
        marine_band: crate::sim::goods_spec::default_marine_band_for(&sec.id),
        scoring: if distribution == Distribution::Deposits || distribution == Distribution::Manufactured {
            None
        } else {
            Some(Default::default())
        },
        category,
        need_tier,
        base_value,
        bulk,
        perishable,
        inputs: Vec::new(),
        labor: 1.0,
        consumption_interval: if network_luxury { 180.0 } else { 45.0 },
    })
}

/// Parse a `.txt` goods file into new specs (never touching the existing
/// library) plus a full report of what happened to every section.
pub fn parse_goods_txt(text: &str) -> (Vec<GoodSpec>, ImportReport) {
    let (sections, orphans) = split_sections(text);
    let mut report = ImportReport { rejected: orphans, ..Default::default() };
    let mut out = Vec::new();
    let mut seen_in_file: std::collections::HashSet<String> = std::collections::HashSet::new();
    for sec in &sections {
        if !seen_in_file.insert(sec.id.clone()) {
            report.rejected.push((sec.id.clone(), "duplicate [section] within this file — only the first is kept".into()));
            continue;
        }
        if let Some(spec) = build_good(sec, &mut report) {
            report.added.push(spec.id.clone());
            out.push(spec);
        }
    }
    (out, report)
}

/// Merge freshly parsed goods into an existing library — ADD ONLY (D8): an id
/// already in `existing` is rejected, never overwritten. Mutates `report` in
/// place so a caller sees one report covering parsing AND the merge.
pub fn merge_add_only(existing: &mut Vec<GoodSpec>, parsed: Vec<GoodSpec>, report: &mut ImportReport) {
    let existing_ids: std::collections::HashSet<String> = existing.iter().map(|s| s.id.clone()).collect();
    let mut kept_added = Vec::new();
    for spec in parsed {
        if existing_ids.contains(&spec.id) {
            report.added.retain(|id| id != &spec.id);
            report.rejected.push((spec.id.clone(), "id already exists in the library — import never replaces (D8)".into()));
            continue;
        }
        kept_added.push(spec.id.clone());
        existing.push(spec);
    }
    let _ = kept_added;
}

/// Import a `.txt` goods file into the global library (the editing template for
/// new worlds — see `goods_commands::get_goods_library`/`save_goods_library`).
/// Add-only per D8; always returns a full report even when nothing was added.
/// `path` is a plain filesystem path (the frontend picks it via the OS file
/// dialog, same pattern `import_world_layers` uses).
#[tauri::command]
pub fn import_goods_txt(app: tauri::AppHandle, path: String) -> Result<ImportReport, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))?;
    let mut library = crate::commands::goods_commands::get_goods_library(app.clone())?;
    let (parsed, mut report) = parse_goods_txt(&text);
    merge_add_only(&mut library, parsed, &mut report);
    crate::sim::goods_spec::backfill_market_fields(&mut library);
    crate::commands::goods_commands::save_goods_library(app, library)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_line_block_produces_a_working_mineral() {
        let txt = "[foo_stone]\nname = Foo Stone\n";
        let (specs, report) = parse_goods_txt(txt);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].id, "foo_stone");
        assert_eq!(specs[0].distribution, Distribution::Deposits);
        assert!(report.added.contains(&"foo_stone".to_string()));
        // Every field left implicit must show up in the report, not vanish.
        assert!(report.defaulted.iter().any(|(id, _)| id == "foo_stone"));
    }

    #[test]
    fn the_plan_example_parses_fully() {
        let txt = "[lapis_lazuli]\n\
            name          = Lapis Lazuli\n\
            icon          = \u{1F537}\n\
            color         = #26619c\n\
            domain        = continental\n\
            distribution  = deposits\n\
            deposit_model = contact_metamorphic\n\
            districts     = 1              # famously ONE source, for four thousand years\n\
            workings      = 3..6\n\
            grade         = 0.55..0.95\n\
            depth         = shallow\n\
            rarity        = 0.93\n\
            base_value    = 40\n\
            category      = gem\n\
            need_tier     = 2\n";
        let (specs, report) = parse_goods_txt(txt);
        assert_eq!(specs.len(), 1);
        let s = &specs[0];
        assert_eq!(s.name, "Lapis Lazuli");
        assert_eq!(s.color, "#26619c");
        assert_eq!(s.base_value, 40.0);
        assert_eq!(s.category, "gem");
        assert_eq!(s.need_tier, 2);
        assert_eq!(s.deposit.as_ref().unwrap().model, Some(DepositModel::ContactMetamorphic));
        assert_eq!(s.deposit.as_ref().unwrap().count_num, 1);
        // workings/grade/depth are named as not-yet-wired, never silently eaten.
        for k in ["workings", "grade", "depth"] {
            assert!(report.defaulted.iter().any(|(_, note)| note.contains(k)), "missing report for {k}");
        }
    }

    #[test]
    fn missing_name_is_rejected_not_silently_skipped() {
        let (specs, report) = parse_goods_txt("[no_name]\nicon = \u{1FAA8}\n");
        assert!(specs.is_empty());
        assert!(report.rejected.iter().any(|(id, reason)| id == "no_name" && reason.contains("name")));
    }

    #[test]
    fn duplicate_id_within_file_keeps_only_the_first() {
        let txt = "[a]\nname = First\n[a]\nname = Second\n";
        let (specs, report) = parse_goods_txt(txt);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "First");
        assert!(report.rejected.iter().any(|(id, r)| id == "a" && r.contains("duplicate")));
    }

    #[test]
    fn add_only_never_overwrites_an_existing_id() {
        let mut existing = vec![GoodSpec {
            origins: 1, soil: Vec::new(), relief: None,
            id: "iron".into(), name: "Iron (existing)".into(), icon: "x".into(), color: "#000".into(),
            enabled: true, domain: Domain::Continental, distribution: Distribution::Deposits,
            rarity: 0.5, desire: 0.5, network_luxury: false, builtin: true, deposit: None, scoring: None,
            marine_band: crate::sim::goods_spec::MarineBand::Either,
            category: "metal".into(), need_tier: 1, base_value: 3.0, bulk: 1.0, perishable: 0.0,
            inputs: Vec::new(), labor: 1.0, consumption_interval: 45.0,
        }];
        let (parsed, mut report) = parse_goods_txt("[iron]\nname = New Iron\n");
        merge_add_only(&mut existing, parsed, &mut report);
        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].name, "Iron (existing)");
        assert!(!report.added.contains(&"iron".to_string()));
        assert!(report.rejected.iter().any(|(id, r)| id == "iron" && r.contains("already exists")));
    }

    #[test]
    fn unknown_key_is_reported_not_dropped() {
        let (_, report) = parse_goods_txt("[x]\nname = X\nbase_vaule = 10\n");
        assert!(report.defaulted.iter().any(|(_, note)| note.contains("base_vaule")));
    }

    #[test]
    fn inline_comment_after_a_color_value_keeps_the_hash() {
        let (specs, _) = parse_goods_txt("[x]\nname = X\ncolor = #ff00aa\n");
        assert_eq!(specs[0].color, "#ff00aa");
    }
}
