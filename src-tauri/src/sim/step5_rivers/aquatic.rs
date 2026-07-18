//! Freshwater ecology â€” river flow-regime + fish assemblage, and lake
//! limnological classification + descriptions. Everything here is derived from
//! data the world sim already computes (KÃ¶ppen climate, temperature, gradient,
//! discharge, salinity, elevation, volcanism, plate boundaries) and mapped to
//! REAL Earth taxa and real-world analogs, matching the "use real-world river
//! reference" design brief.
//!
//! Pure & deterministic: functions take plain scalars (aggregated from the
//! WorldBuffer by the query layer) so they can be unit-tested without a world.

use crate::sim::koppen;

/// Broad thermal band a freshwater body sits in, from mean annual air temp (Â°C).
/// Drives which fish families are plausible (cold salmonids â†” warm cichlids).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Band {
    Tropical,
    WarmTemperate,
    CoolTemperate,
    Boreal,
    Polar,
}

pub fn band_from_temp(t: f32) -> Band {
    if t >= 22.0 { Band::Tropical }
    else if t >= 14.0 { Band::WarmTemperate }
    else if t >= 6.0 { Band::CoolTemperate }
    else if t >= -2.0 { Band::Boreal }
    else { Band::Polar }
}

/// A river's hydrological *regime* â€” the shape of its year, driven by the
/// dominant KÃ¶ppen class of its basin. Returns a short human phrase.
pub fn river_regime(dominant_koppen: u8, band: Band) -> String {
    use koppen::*;
    match dominant_koppen {
        AF | AM => "perennial tropical â€” steady, high year-round flow".into(),
        AW => "tropical flood-pulse â€” a strong wet-season flood, a lean dry season".into(),
        BWH | BWK | BSH | BSK =>
            "arid â€” ephemeral flow between storms, or an 'exotic' river fed from wetter uplands".into(),
        CSA | CSB =>
            "Mediterranean â€” winter floods and a parched summer low".into(),
        CWA | CWB =>
            "subtropical monsoonal â€” summer-rain fed, high in the wet monsoon".into(),
        CFA | CFB | CFC =>
            "perennial temperate â€” rain-fed and reliable all year".into(),
        DFA | DFB | DWA | DWB =>
            "continental â€” a strong spring snowmelt freshet, ice cover in winter".into(),
        DFC | DFD | DWC | DWD =>
            "nival subarctic â€” snowmelt-dominated, frozen much of the year".into(),
        ET | EF =>
            "nival polar â€” a brief summer melt, ice-bound the rest of the year".into(),
        _ => match band {
            Band::Tropical => "perennial tropical flow".into(),
            Band::WarmTemperate => "warm-temperate, largely perennial".into(),
            Band::CoolTemperate => "cool-temperate, rain- and snow-fed".into(),
            Band::Boreal => "boreal, snowmelt-fed".into(),
            Band::Polar => "polar, melt-fed and brief".into(),
        },
    }
}

/// A river's fish assemblage as a prose sentence, built from thermal band Ã—
/// channel gradient (rheophilic headwater vs limnophilic lowland) Ã— size
/// (discharge â†’ megafish) Ã— mouth type (diadromous runs from an estuary).
/// `gradient_m_per_km` and `discharge_m3s` come straight from the dashboard math;
/// `estuary` is true for a tidal/drowned mouth (mouth_kind == 2) or a big delta.
pub fn river_fish(band: Band, gradient_m_per_km: f32, discharge_m3s: f32, estuary: bool) -> String {
    let steep = gradient_m_per_km >= 6.0;
    let great = discharge_m3s >= 3000.0;
    let large = discharge_m3s >= 400.0;

    // Core community by band + gradient.
    let core: &str = match band {
        Band::Tropical => if steep {
            "swift-water loaches and hillstream catfish"
        } else {
            "characins (tetras, pacu), pimelodid catfish and cichlids"
        },
        Band::WarmTemperate => if steep {
            "trout and stone loach in the cool riffles"
        } else {
            "barbel, nase, carp and wels catfish"
        },
        Band::CoolTemperate => if steep {
            "brown trout, grayling and bullhead sculpin"
        } else {
            "roach, bream, perch and pike, with pike-perch in the deeper pools"
        },
        Band::Boreal => if steep {
            "grayling and Arctic char"
        } else {
            "pike, perch, burbot and whitefish"
        },
        Band::Polar => "hardy whitefish, char and grayling (sparse, low-diversity)",
    };

    // Megafish for the great trunks.
    let mega: &str = if great {
        match band {
            Band::Tropical => "; giant catfish and arapaima haunt the main channel",
            Band::WarmTemperate | Band::CoolTemperate => "; sturgeon and giant catfish run the deep trunk",
            Band::Boreal => "; sturgeon work the deep main stem",
            Band::Polar => "",
        }
    } else { "" };

    // Diadromous run from the sea.
    let run: &str = if estuary && large {
        match band {
            Band::Tropical => "; mullet and snook push up from the brackish delta",
            Band::WarmTemperate => "; shad, eel and sea bass ascend from the estuary",
            Band::CoolTemperate | Band::Boreal => "; salmon, shad and eel run up from the estuary to spawn",
            Band::Polar => "; anadromous char and salmon run in from the coast",
        }
    } else { "" };

    format!("{core}{mega}{run}")
}

/// Water character â€” clarity / sediment / productivity, the way a limnologist
/// would tag the water itself. Tropical rivers split into the classic
/// whitewater / blackwater / clearwater triad (Amazon terminology).
pub fn river_water(band: Band, steep: bool, arid: bool) -> String {
    if arid {
        return "turbid, mineral-laden and warm, shrinking to pools in the dry".into();
    }
    match band {
        Band::Tropical => if steep {
            "clearwater â€” cool, transparent and low in nutrients off the shield rock".into()
        } else {
            "whitewater â€” silt-laden and cafÃ©-au-lait, fertile with Andean-style sediment".into()
        },
        Band::WarmTemperate => "warm and moderately productive, clouding after rain".into(),
        Band::CoolTemperate => "cool, clear and well-oxygenated".into(),
        Band::Boreal => "cold, tea-stained blackwater draining peat and bog".into(),
        Band::Polar => "frigid, clear glacial-melt, milky with rock flour near the source".into(),
    }
}

/// Riverine wildlife beyond fish â€” the charismatic fauna a naturalist would list.
pub fn river_wildlife(band: Band, great: bool) -> String {
    match band {
        Band::Tropical => if great {
            "crocodilians, river turtles, otters and wading storks; river dolphins in the great channel".into()
        } else {
            "caiman, kingfishers, turtles and otters along the banks".into()
        },
        Band::WarmTemperate => "otters, herons, terrapins and kingfishers; beavers on quiet reaches".into(),
        Band::CoolTemperate => "otters, beavers, herons, dippers and kingfishers".into(),
        Band::Boreal => "beavers, moose at the fords, mergansers and osprey".into(),
        Band::Polar => "migratory waterfowl in the brief summer; little else".into(),
    }
}

/// The riparian (bankside) character in a phrase, from thermal band + aridity.
pub fn river_riparian(band: Band, arid: bool) -> String {
    if arid {
        return "a thin ribbon of tamarisk and reeds threading otherwise dry country".into();
    }
    match band {
        Band::Tropical => "dense gallery forest and papyrus swamp crowding the banks".into(),
        Band::WarmTemperate => "willow, poplar and cane along a broad floodplain".into(),
        Band::CoolTemperate => "alder and willow galleries with reed-marsh backwaters".into(),
        Band::Boreal => "spruce and birch riverine taiga, boggy in the flats".into(),
        Band::Polar => "low tundra and dwarf willow, frozen for much of the year".into(),
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ National-Geographic river narrative â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Deterministic splitmix64 â€” a tiny PRNG so every river's write-up is UNIQUE yet
/// reproducible (seeded from the river's own id/position). Two hydrologically
/// identical rivers still read differently because their seeds differ.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    /// Pick one of `opts` by the next draw.
    fn pick<'a>(&mut self, opts: &[&'a str]) -> &'a str {
        opts[(self.next() % opts.len() as u64) as usize]
    }
    fn chance(&mut self, p: u64) -> bool { self.next() % 100 < p }
}

/// Everything the narrative weaves together â€” the river's own numbers plus the
/// ecology phrases already computed. Borrowed so no allocation is forced.
pub struct RiverStoryFacts<'a> {
    pub name: &'a str,
    pub counterpart: &'a str,
    pub length_km: f32,
    pub discharge_m3s: f32,
    pub source_kind: &'a str,
    pub mouth_kind: u8,
    pub navigable: bool,
    pub trib_total: usize,
    pub city_total: usize,
    pub band: Band,
    pub regime: &'a str,
    pub fish: &'a str,
    pub riparian: &'a str,
    pub water: &'a str,
    pub wildlife: &'a str,
    pub arid: bool,
    pub seed: u64,
}

fn scale_word(q: f32) -> &'static str {
    if q >= 8000.0 { "one of the great rivers of the world" }
    else if q >= 3000.0 { "a mighty river" }
    else if q >= 800.0 { "a large river" }
    else if q >= 150.0 { "a substantial river" }
    else if q >= 30.0 { "a modest river" }
    else { "a small river" }
}

fn source_phrase(kind: &str) -> &'static str {
    match kind {
        "alpine" => "high among the snows and glaciers",
        "highland" => "in cold highland springs",
        "hills" => "in rain-fed hill country",
        "bog" => "in a broad lowland bog",
        _ => "in the low country",
    }
}

fn mouth_phrase(kind: u8) -> &'static str {
    match kind {
        1 => "spilling into the sea across a broad, marshy delta",
        2 => "widening into a drowned tidal estuary at its mouth",
        _ => "reaching its end on a quiet shore",
    }
}

/// Compose a UNIQUE, multi-sentence NatGeo-style account of a river SYSTEM,
/// stitched from seeded phrase banks + the river's real figures + its ecology.
/// Each river gets its own voice; the same seed always yields the same text.
pub fn river_story(f: &RiverStoryFacts) -> String {
    let mut r = Rng(f.seed ^ 0x5149_5645_5200_0001);
    let mut s = String::new();

    // â”€â”€ 1 Â· Opening: name, scale, source, course, mouth â”€â”€
    let opener = r.pick(&[
        "The {N} is {S}, rising {SRC} and running some {L} km before {M}.",
        "Rising {SRC}, the {N} gathers itself into {S}, threading {L} km down to the coast before {M}.",
        "{S_cap}, the {N} runs roughly {L} km from its birth {SRC} to where it ends {M}.",
        "Born {SRC}, the {N} â€” {S} â€” winds {L} km across the land, at last {M}.",
    ]);
    let mut para = opener
        .replace("{N}", f.name)
        .replace("{S_cap}", cap_first(scale_word(f.discharge_m3s)).as_str())
        .replace("{S}", scale_word(f.discharge_m3s))
        .replace("{SRC}", source_phrase(f.source_kind))
        .replace("{L}", &format!("{:.0}", f.length_km.max(1.0)))
        .replace("{M}", mouth_phrase(f.mouth_kind));
    s.push_str(&para);

    // â”€â”€ 2 Â· Regime + water character â”€â”€
    para = r.pick(&[
        " Its waters are {REGIME}; {WATER}.",
        " Fed by a flow that is {REGIME}, it runs {WATER}.",
        " The river keeps a rhythm that is {REGIME}, its channel {WATER}.",
    ]).replace("{REGIME}", f.regime).replace("{WATER}", f.water);
    s.push_str(&para);

    // â”€â”€ 3 Â· Banks + wildlife â”€â”€
    para = r.pick(&[
        " Along its banks grow {RIP}, alive with {WILD}.",
        " {RIP_cap} crowd the shore, where one finds {WILD}.",
        " The floodplain is {RIP}; here live {WILD}.",
    ]).replace("{RIP_cap}", cap_first(f.riparian).as_str())
      .replace("{RIP}", f.riparian).replace("{WILD}", f.wildlife);
    s.push_str(&para);

    // â”€â”€ 4 Â· Fish (the ichthyology) â”€â”€
    para = r.pick(&[
        " In its depths swim {FISH}.",
        " The river's fish are {FISH}.",
        " Beneath the surface move {FISH}.",
    ]).replace("{FISH}", f.fish);
    s.push_str(&para);

    // â”€â”€ 5 Â· System scale (tributaries) â€” only if it's a real network â”€â”€
    if f.trib_total >= 2 {
        para = r.pick(&[
            " Gathering {T} named tributaries, it drains a wide basin.",
            " A branching system of some {T} tributaries feeds it.",
            " Its watershed knits together {T} tributary streams.",
        ]).replace("{T}", &f.trib_total.to_string());
        s.push_str(&para);
    }

    // â”€â”€ 6 Â· Human use + real-world comparison â”€â”€
    let human = if f.navigable {
        r.pick(&[
            " Barges work its navigable length, and {C} cities stand on its banks.",
            " It is navigable far inland â€” a highway of trade linking {C} riverside cities.",
        ])
    } else {
        r.pick(&[
            " Too swift and shallow for boats, it still waters {C} settlements along its course.",
            " Its {C} riverside towns draw on it for water, mills and fishing.",
        ])
    };
    if f.city_total > 0 {
        s.push_str(&human.replace("{C}", &f.city_total.to_string()));
    }
    if !f.counterpart.is_empty() && r.chance(85) {
        s.push_str(&r.pick(&[
            " In scale and character it recalls {CP}.",
            " Travellers might liken it to {CP}.",
            " In its way it is a cousin to {CP}.",
        ]).replace("{CP}", f.counterpart));
    }

    s
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ Segmented river journey (upper / middle / delta) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Plain-language BIOME for a KÃ¶ppen code â€” for the river's climate journey and
/// per-zone stories (readable, with the raw code available via `koppen_code`).
pub fn koppen_to_biome(k: u8) -> &'static str {
    use koppen::*;
    match k {
        AF => "tropical rainforest",
        AM => "tropical monsoon forest",
        AW => "tropical savanna",
        BWH => "hot desert",
        BWK => "cold desert",
        BSH => "hot steppe",
        BSK => "cold steppe",
        CSA | CSB => "Mediterranean scrub",
        CWA | CWB => "subtropical monsoon woodland",
        CFA => "humid subtropical forest",
        CFB | CFC => "temperate broadleaf forest",
        DFA | DFB | DWA | DWB => "continental mixed forest",
        DFC | DFD | DWC | DWD => "boreal taiga",
        ET => "cold tundra",
        EF => "polar ice",
        _ => "open country",
    }
}

/// The raw KÃ¶ppen code string (shown in a tooltip alongside the biome name).
pub fn koppen_code(k: u8) -> &'static str {
    use koppen::*;
    match k {
        AF => "Af", AM => "Am", AW => "Aw", BWH => "BWh", BWK => "BWk", BSH => "BSh", BSK => "BSk",
        CSA => "Csa", CSB => "Csb", CWA => "Cwa", CWB => "Cwb", CFA => "Cfa", CFB => "Cfb", CFC => "Cfc",
        DFA => "Dfa", DFB => "Dfb", DFC => "Dfc", DFD => "Dfd", DWA => "Dwa", DWB => "Dwb", DWC => "Dwc", DWD => "Dwd",
        ET => "ET", EF => "EF", _ => "â€”",
    }
}

/// Facts for one reach (upper / middle / lower-delta) of a river's course.
pub struct ZoneFacts<'a> {
    pub kind: u8,                 // 0 upper Â· 1 middle Â· 2 lower/delta
    pub name: &'a str,
    pub biome: &'a str,          // dominant biome of this reach
    pub fish: &'a str,           // reach-appropriate fish prose
    pub tribs: &'a [(String, f32)], // (name, km-from-source) joining in this reach
    pub mouth_kind: u8,          // only meaningful for the delta reach
    pub band: Band,
    pub seed: u64,
}

/// A UNIQUE, seeded 2-3 sentence account of ONE reach of a river â€” its character,
/// the biome it crosses, its fish, and any tributaries that join along it.
pub fn zone_story(f: &ZoneFacts) -> String {
    let mut r = Rng(f.seed ^ 0x20E5_0000_0000_0001);
    let mut s = String::new();
    let open = match f.kind {
        0 => r.pick(&[
            "In its upper course the {N} is a cold, clear torrent, cutting a steep valley through {B}.",
            "The {N} begins as a brisk headwater stream in {B}, tumbling swiftly down a narrow valley.",
            "High in {B} the young {N} runs fast and shallow over rock and gravel.",
        ]),
        1 => r.pick(&[
            "Through its middle course the {N} broadens and slows across {B}, gathering volume and beginning to wander.",
            "In the middle reaches the {N} is a fuller, calmer river winding through {B}.",
            "Across {B} the {N} settles into a steady, meandering flow.",
        ]),
        _ => match f.mouth_kind {
            1 => r.pick(&[
                "Near the sea the {N} fans into a broad delta amid {B} â€” a teeming maze of reed, channel and tidal flat.",
                "At its mouth the {N} splits into a marshy delta over {B}, alive with fish, fowl and rustling reed-beds.",
            ]),
            2 => r.pick(&[
                "The {N} ends in a drowned tidal estuary through {B}, broad and brackish where river meets sea.",
                "At its mouth the {N} widens into a tidal estuary among {B}, its water turning salt with the tide.",
            ]),
            _ => r.pick(&[
                "In its lower course the {N} crosses {B} at an easy pace to a quiet shore.",
                "The {N} runs out its lower course over {B} to its end.",
            ]),
        },
    };
    s.push_str(&open.replace("{N}", f.name).replace("{B}", f.biome));
    s.push(' ');
    s.push_str(&r.pick(&["Its waters here hold {F}.", "Here swim {F}.", "The fishing yields {F}."]).replace("{F}", f.fish));
    if !f.tribs.is_empty() {
        let list: Vec<String> = f.tribs.iter().take(3)
            .map(|(n, km)| format!("the {} at {:.0} km", n.trim_start_matches("the ").trim(), km))
            .collect();
        s.push(' ');
        s.push_str(&format!("It gathers {} along the way.", join_and(&list)));
    }
    let _ = f.band;
    s
}

/// Facts for the river's one-paragraph SUMMARY lede (a synthesis of the 3 reaches).
pub struct RiverSummaryFacts<'a> {
    pub name: &'a str,
    pub discharge_m3s: f32,
    pub source_kind: &'a str,
    pub mouth_kind: u8,
    pub trib_total: usize,
    pub journey: &'a str,        // "temperate forest, then cold steppe, ending in reed delta"
    pub counterpart: &'a str,
    pub length_km: f32,
    pub navigable: bool,
    pub seed: u64,
}

/// The SUMMARY lede â€” a short synthesis of the whole course (scale, birth, the
/// climate journey, mouth), the replacement for the old single story blob.
pub fn river_summary(f: &RiverSummaryFacts) -> String {
    let mut r = Rng(f.seed ^ 0x5010_0000_0000_0001);
    let mut s = String::new();
    let opener = r.pick(&[
        "The {N} is {S}, rising {SRC} and running some {L} km {M}.",
        "Rising {SRC}, the {N} â€” {S} â€” threads {L} km {M}.",
        "{S_cap}, the {N} runs about {L} km from its birth {SRC} to where it ends {M}.",
    ]);
    s.push_str(&opener
        .replace("{N}", f.name)
        .replace("{S_cap}", cap_first(scale_word(f.discharge_m3s)).as_str())
        .replace("{S}", scale_word(f.discharge_m3s))
        .replace("{SRC}", source_phrase(f.source_kind))
        .replace("{L}", &format!("{:.0}", f.length_km.max(1.0)))
        .replace("{M}", mouth_phrase(f.mouth_kind)));
    if !f.journey.is_empty() {
        s.push(' ');
        s.push_str(&r.pick(&[
            "Along the way it crosses {J}.",
            "Its course carries it through {J}.",
        ]).replace("{J}", f.journey));
    }
    if f.trib_total >= 2 {
        s.push(' ');
        s.push_str(&r.pick(&[
            "It gathers some {T} named tributaries into a wide basin.",
            "A branching web of {T} tributaries feeds it.",
        ]).replace("{T}", &f.trib_total.to_string()));
    }
    if f.navigable {
        s.push(' ');
        s.push_str("Barges work its lower reaches, a highway of inland trade.");
    }
    if !f.counterpart.is_empty() && r.chance(85) {
        s.push(' ');
        s.push_str(&r.pick(&[
            "In scale and character it recalls {CP}.",
            "Travellers might liken it to {CP}.",
        ]).replace("{CP}", f.counterpart));
    }
    s
}

/// Join phrases with commas and a trailing "and" ("a, b and c").
fn join_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, head) = items.split_last().unwrap();
            format!("{} and {}", head.join(", "), last)
        }
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ Lakes â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Limnological lake type. Ordered by how distinctive the signal is; the
/// classifier tests them roughly in this priority.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LakeKind {
    Crater,   // volcanic caldera lake (Crater Lake, Toba)
    Rift,     // deep tectonic/graben lake (Baikal, Tanganyika)
    SaltEndorheic, // terminal, no outlet, arid (Caspian, Aral, Dead Sea, Chad)
    Glacial,  // ice-carved, high-latitude (Great Lakes, Finnish lakes)
    TropicalGreat, // large shallow warm lake (Victoria)
    Lowland,  // shallow temperate floodplain lake (Balaton, Neusiedl)
    Tarn,     // small high-elevation upland lake (default)
}

impl LakeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LakeKind::Crater => "crater",
            LakeKind::Rift => "rift",
            LakeKind::SaltEndorheic => "salt",
            LakeKind::Glacial => "glacial",
            LakeKind::TropicalGreat => "tropical",
            LakeKind::Lowland => "lowland",
            LakeKind::Tarn => "tarn",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LakeKind::Crater => "Crater lake",
            LakeKind::Rift => "Rift lake",
            LakeKind::SaltEndorheic => "Salt lake",
            LakeKind::Glacial => "Glacial lake",
            LakeKind::TropicalGreat => "Great tropical lake",
            LakeKind::Lowland => "Lowland lake",
            LakeKind::Tarn => "Upland tarn",
        }
    }
}

/// Inputs the classifier needs, aggregated over a lake's cells by the query layer.
pub struct LakeFacts {
    pub area_km2: f32,
    pub max_depth_m: f32,
    pub elev_m: f32,
    pub abs_lat: f32,
    pub mean_temp_c: f32,
    pub volcanic: bool,       // volcanic cell in/around the basin
    pub on_boundary: bool,    // sits on a plate boundary (rift/graben)
    pub endorheic: bool,      // no outflow river
    pub arid: bool,           // basin lies in a BW/BS KÃ¶ppen zone
    pub elongated: bool,      // long/narrow footprint (graben-like)
}

/// Classify a lake into a real limnological type from its facts (priority order:
/// caldera â†’ rift â†’ terminal salt â†’ glacial â†’ great-tropical â†’ lowland â†’ tarn).
pub fn classify_lake(f: &LakeFacts) -> LakeKind {
    if f.volcanic && f.max_depth_m >= 40.0 && f.area_km2 <= 400.0 {
        return LakeKind::Crater;
    }
    if f.on_boundary && f.max_depth_m >= 150.0 && f.elongated {
        return LakeKind::Rift;
    }
    if f.endorheic && f.arid {
        return LakeKind::SaltEndorheic;
    }
    if f.abs_lat >= 48.0 && f.mean_temp_c < 8.0 {
        return LakeKind::Glacial;
    }
    if f.area_km2 >= 1500.0 && f.max_depth_m < 80.0 && f.mean_temp_c >= 20.0 {
        return LakeKind::TropicalGreat;
    }
    if f.max_depth_m < 30.0 && f.elev_m < 600.0 {
        return LakeKind::Lowland;
    }
    LakeKind::Tarn
}

/// Thermal mixing regime phrase from depth + thermal band.
pub fn lake_thermal(kind: LakeKind, band: Band, max_depth_m: f32) -> String {
    match kind {
        LakeKind::SaltEndorheic => "warm, well-mixed and evaporation-driven".into(),
        LakeKind::TropicalGreat => "warm, wind-mixed, only briefly stratifying".into(),
        _ => match band {
            Band::Tropical => "warm-monomictic â€” never cools enough to fully turn over".into(),
            Band::WarmTemperate | Band::CoolTemperate =>
                if max_depth_m >= 40.0 { "dimictic â€” turns over each spring and autumn".into() }
                else { "warm-polymictic â€” mixed by the wind through the season".into() },
            Band::Boreal | Band::Polar =>
                if max_depth_m >= 60.0 { "cold-monomictic â€” mixes once a year under a summer sun".into() }
                else { "ice-covered much of the year, mixing briefly after thaw".into() },
        },
    }
}

/// Trophic / water-clarity phrase.
pub fn lake_water(kind: LakeKind) -> String {
    match kind {
        LakeKind::Crater => "deep, isolated and ultra-clear (oligotrophic)".into(),
        LakeKind::Rift => "deep, clear and oligotrophic, with an oxygen-poor abyss".into(),
        LakeKind::SaltEndorheic => "saline to hypersaline, its shoreline rising and falling with the years".into(),
        LakeKind::Glacial => "cold, clear and nutrient-poor (oligotrophic)".into(),
        LakeKind::TropicalGreat => "warm, turbid and richly productive (eutrophic)".into(),
        LakeKind::Lowland => "shallow, green and weed-rich (eutrophic), warm in summer".into(),
        LakeKind::Tarn => "cold, clear and sparse (oligotrophic)".into(),
    }
}

/// Lake fish assemblage as prose, keyed to type + thermal band. Notes endemism
/// for the ancient rift lakes (species flocks found nowhere else).
pub fn lake_fish(kind: LakeKind, band: Band) -> String {
    match kind {
        LakeKind::SaltEndorheic => match band {
            Band::Tropical | Band::WarmTemperate =>
                "brine shrimp and salt-tolerant tilapia and cyprinodonts â€” or lifeless where it turns hypersaline".into(),
            _ => "a few salt-tolerant whitefish and sticklebacks, or barren brine".into(),
        },
        LakeKind::Rift => match band {
            Band::Tropical =>
                "an explosive cichlid species-flock found nowhere else, with catfish and lungfish".into(),
            _ =>
                "relict deep-water sculpins and endemic whitefish, and in the coldest a lone freshwater seal".into(),
        },
        LakeKind::Crater =>
            "sparse and isolated â€” a handful of hardy, often endemic fish, if any at all".into(),
        LakeKind::TropicalGreat =>
            "a cichlid flock, tilapia and lungfish crowd the productive shallows".into(),
        LakeKind::Glacial => "lake trout, whitefish, char, pike and perch".into(),
        LakeKind::Lowland => "carp, bream, tench, pike and pike-perch in the weedy shallows".into(),
        LakeKind::Tarn => "cold-water trout and char only".into(),
    }
}

/// The closest real-world lake analog for a type + band (distinct from a river's
/// discharge counterpart).
pub fn lake_analog(kind: LakeKind, band: Band) -> &'static str {
    match kind {
        LakeKind::Crater => "Crater Lake",
        LakeKind::Rift => if band == Band::Tropical { "Lake Tanganyika" } else { "Lake Baikal" },
        LakeKind::SaltEndorheic => if band == Band::Tropical { "Lake Chad" } else { "the Caspian Sea" },
        LakeKind::Glacial => "the Great Lakes",
        LakeKind::TropicalGreat => "Lake Victoria",
        LakeKind::Lowland => "Lake Balaton",
        LakeKind::Tarn => "an alpine tarn",
    }
}

/// Lakeshore wildlife beyond fish.
pub fn lake_wildlife(kind: LakeKind, band: Band) -> String {
    match kind {
        LakeKind::SaltEndorheic =>
            "vast flocks of flamingoes and wading birds feeding on brine shrimp; little else".into(),
        LakeKind::TropicalGreat =>
            "hippos and crocodiles in the shallows, pelicans, cormorants and fish-eagles overhead".into(),
        LakeKind::Rift => if band == Band::Tropical {
            "crocodiles and otters below, cormorants and kingfishers above".into()
        } else {
            "seals hauled out on the ice, with gulls and diving ducks in summer".into()
        },
        LakeKind::Glacial => "loons, mergansers and osprey; moose browsing the shallows".into(),
        LakeKind::Lowland => "grebes, herons, coots and vast reed-bed birdlife".into(),
        LakeKind::Crater => "a few resident waterbirds on an otherwise still and silent water".into(),
        LakeKind::Tarn => "dippers and the odd wading bird; a quiet, high place".into(),
    }
}

/// Endemism note â€” how much of the fauna is unique to this water.
pub fn lake_endemism(kind: LakeKind, band: Band) -> String {
    match kind {
        LakeKind::Rift => if band == Band::Tropical {
            "extreme â€” hundreds of cichlid species evolved here and nowhere else".into()
        } else {
            "high â€” ancient isolation has bred relict species found only in these depths".into()
        },
        LakeKind::TropicalGreat => "high â€” a young but explosive cichlid radiation".into(),
        LakeKind::Crater => "moderate â€” isolation breeds the occasional unique form".into(),
        LakeKind::SaltEndorheic => "low â€” only a few hardy specialists tolerate the brine".into(),
        _ => "low â€” a familiar, widespread fauna".into(),
    }
}

/// Rough salinity (parts per thousand) â€” near-fresh for most, high for terminal
/// salt lakes (shallower & more arid â†’ more concentrated). Seawater â‰ˆ 35 ppt.
pub fn lake_salinity_ppt(kind: LakeKind, max_depth_m: f32) -> f32 {
    match kind {
        LakeKind::SaltEndorheic => {
            // Shallow terminal basins concentrate hardest; deep ones stay brackish.
            if max_depth_m < 15.0 { 120.0 } else if max_depth_m < 60.0 { 35.0 } else { 12.0 }
        }
        _ => 0.2,
    }
}

/// A one-line flavour description stitched from type + analog.
pub fn lake_blurb(kind: LakeKind, band: Band) -> String {
    let analog = lake_analog(kind, band);
    match kind {
        LakeKind::Crater =>
            format!("A drowned volcanic caldera in the mould of {analog} â€” round, deep and startlingly clear."),
        LakeKind::Rift =>
            format!("An ancient rift lake like {analog} â€” long, immensely deep, and a cradle of life found nowhere else."),
        LakeKind::SaltEndorheic =>
            format!("A terminal salt lake in the character of {analog} â€” rivers run in and none run out; only the sun lets it go."),
        LakeKind::Glacial =>
            format!("A cold, clear lake carved by ice, in the family of {analog}."),
        LakeKind::TropicalGreat =>
            format!("A vast, shallow tropical lake like {analog} â€” warm, teeming and wind-stirred."),
        LakeKind::Lowland =>
            format!("A broad, shallow lowland lake in the character of {analog} â€” green and warm in summer, reed-fringed all round."),
        LakeKind::Tarn =>
            format!("A small, cold upland tarn â€” clear, quiet and sparsely peopled by fish."),
    }
}

/// Facts the lake narrative weaves together.
pub struct LakeStoryFacts<'a> {
    pub name: &'a str,
    pub kind: LakeKind,
    pub band: Band,
    pub analog: &'a str,
    pub area_km2: f32,
    pub max_depth_m: f32,
    pub endorheic: bool,
    pub inflow_count: usize,
    pub outflow: &'a str,
    pub thermal: &'a str,
    pub water: &'a str,
    pub fish: &'a str,
    pub wildlife: &'a str,
    pub endemism: &'a str,
    pub seed: u64,
}

/// A UNIQUE, seeded National-Geographic-style account of a lake â€” no two of the
/// same type read alike.
pub fn lake_story(f: &LakeStoryFacts) -> String {
    let mut r = Rng(f.seed ^ 0x1A4E_0000_0000_0001u64);
    let size = if f.area_km2 >= 20000.0 { "a vast inland sea" }
        else if f.area_km2 >= 2000.0 { "a great lake" }
        else if f.area_km2 >= 200.0 { "a broad lake" }
        else if f.area_km2 >= 20.0 { "a sizeable lake" }
        else { "a small lake" };
    let depthw = if f.max_depth_m >= 400.0 { "plunging to extraordinary depth" }
        else if f.max_depth_m >= 100.0 { "deep and dark" }
        else if f.max_depth_m >= 30.0 { "moderately deep" }
        else { "shallow throughout" };

    let mut s = String::new();
    s.push_str(&r.pick(&[
        "The {N} is {SZ}, {DP}.",
        "{SZ_cap}, {DP}, the {N} lies cupped in the land.",
        "The {N} â€” {SZ} â€” is {DP}.",
    ]).replace("{N}", f.name)
      .replace("{SZ_cap}", cap_first(size).as_str())
      .replace("{SZ}", size).replace("{DP}", depthw));

    s.push_str(&r.pick(&[
        " Its water is {WATER}, {THERM}.",
        " {THERM_cap}, the lake is {WATER}.",
        " The lake runs {WATER} and is {THERM}.",
    ]).replace("{THERM_cap}", cap_first(f.thermal).as_str())
      .replace("{THERM}", f.thermal).replace("{WATER}", f.water));

    s.push_str(&r.pick(&[
        " Its fish are {FISH}, and its shores hold {WILD}.",
        " Beneath the surface live {FISH}; along the margins, {WILD}.",
    ]).replace("{FISH}", f.fish).replace("{WILD}", f.wildlife));

    // Endemism only when it's remarkable.
    if matches!(f.kind, LakeKind::Rift | LakeKind::TropicalGreat | LakeKind::Crater) {
        s.push_str(&r.pick(&[
            " Its endemism is {END}.",
            " In biological terms its uniqueness is {END}.",
        ]).replace("{END}", f.endemism));
    }

    // Hydrology: inflow/outflow.
    if f.endorheic {
        s.push_str(&r.pick(&[
            " Rivers run in but none run out â€” the sun alone empties it, and salt gathers year on year.",
            " It is a terminal basin: fed from {IN} directions, drained by none, slowly turning to brine.",
        ]).replace("{IN}", &f.inflow_count.max(1).to_string()));
    } else if !f.outflow.is_empty() {
        s.push_str(&r.pick(&[
            " It gathers {IN} feeder streams and spills out through the {OUT}.",
            " Fed by {IN} rivers, it drains seaward by way of the {OUT}.",
        ]).replace("{IN}", &f.inflow_count.max(1).to_string())
          .replace("{OUT}", f.outflow));
    }

    if r.chance(85) {
        s.push_str(&r.pick(&[
            " In kind it recalls {A}.",
            " In its way it is a cousin to {A}.",
            " Naturalists would name {A} as its closest likeness.",
        ]).replace("{A}", f.analog));
    }
    s
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ Signature fish species (river zonation) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// A curated catalogue of named freshwater fish, one roster per river ZONE
// (0 = upper/headwater, 1 = middle river, 2 = lower/delta), each suited to a set
// of thermal bands. A river reach is assigned one signature species per zone it
// spans (see `assign_river_fish`), giving a biologically-correct longitudinal
// succession (trout up top â†’ barbel in the middle â†’ sturgeon & shad at the mouth).
// `slug` is the stable key the frontend uses to look up an illustration
// (`public/fish/<slug>.png`), falling back to a drawn silhouette if absent.
#[derive(Clone, Copy)]
pub struct FishSpec {
    pub slug: &'static str,
    pub name: &'static str,
    pub binomial: &'static str,
    pub zone: u8,
    pub real: &'static str,
    pub blurb: &'static str,
    pub bands: &'static [Band],
}

use Band::*;
pub const SIGNATURE_FISH: &[FishSpec] = &[
    // â”€â”€ Zone 0 Â· upper / headwater â”€â”€
    FishSpec{slug:"silverfin-trout",name:"Silverfin Trout",binomial:"Salmo argentiventris",zone:0,real:"brown trout",blurb:"the classic cold-water game fish of clean gravelly streams.",bands:&[CoolTemperate,Boreal]},
    FishSpec{slug:"frostscale-grayling",name:"Frostscale Grayling",binomial:"Thymallus gelidus",zone:0,real:"grayling",blurb:"the sail-finned 'lady of the stream', an insect-hunter of clean water.",bands:&[CoolTemperate,Boreal]},
    FishSpec{slug:"stonecling-bullhead",name:"Stonecling Bullhead",binomial:"Cottus rupicola",zone:0,real:"bullhead sculpin",blurb:"a pebble-coloured bottom-lurker of stony riffles.",bands:&[CoolTemperate,Boreal,WarmTemperate]},
    FishSpec{slug:"torrent-loach",name:"Torrent Loach",binomial:"Nemacheilus torrentis",zone:0,real:"stone loach",blurb:"a barbelled bottom-forager of clean gravel.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"ribbon-dace",name:"Ribbon Dace",binomial:"Leuciscus vittatus",zone:0,real:"dace",blurb:"a quick silver shoaling fish of running water.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"redflank-char",name:"Redflank Char",binomial:"Salvelinus rubrimarga",zone:0,real:"Arctic char",blurb:"a jewel-bellied cold-water salmonid of the highest reaches.",bands:&[Boreal,CoolTemperate]},
    // â”€â”€ Zone 1 Â· middle river â”€â”€
    FishSpec{slug:"goldvein-barbel",name:"Goldvein Barbel",binomial:"Barbus auritenuis",zone:1,real:"barbel",blurb:"a barbelled bottom-feeder of warm gravel runs.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"bronze-chub",name:"Bronze Chub",binomial:"Squalius aeneus",zone:1,real:"chub",blurb:"a bold, broad-headed opportunist of the middle river.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"sailback-asp",name:"Sailback Asp",binomial:"Aspius rapax",zone:1,real:"asp",blurb:"a hard-hunting open-water predator of the mid-river.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"whiskered-wels",name:"Whiskered Wels",binomial:"Silurus paludis",zone:1,real:"wels catfish",blurb:"a giant nocturnal catfish of deep slow pools.",bands:&[WarmTemperate]},
    FishSpec{slug:"marbled-perch",name:"Marbled Perch",binomial:"Perca marmorata",zone:1,real:"perch",blurb:"a barred, spiny-finned ambush predator.",bands:&[WarmTemperate,CoolTemperate,Boreal]},
    FishSpec{slug:"gravel-nase",name:"Gravel Nase",binomial:"Chondrostoma lithos",zone:1,real:"nase",blurb:"an algae-scraping shoaler with a black underslung mouth.",bands:&[WarmTemperate,CoolTemperate]},
    // â”€â”€ Zone 2 Â· lower / delta / estuary â”€â”€
    FishSpec{slug:"broadscale-bream",name:"Broadscale Bream",binomial:"Abramis latus",zone:2,real:"bream",blurb:"a deep, slab-sided bottom-forager of slow water.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"marsh-carp",name:"Marsh Carp",binomial:"Cyprinus paludis",zone:2,real:"carp",blurb:"a hardy, barbelled omnivore of warm still reaches.",bands:&[WarmTemperate]},
    FishSpec{slug:"reedwater-zander",name:"Reedwater Zander",binomial:"Sander stagni",zone:2,real:"zander",blurb:"a glassy-eyed, fanged predator of turbid lower rivers.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"silt-sturgeon",name:"Silt Sturgeon",binomial:"Acipenser magnus",zone:2,real:"sturgeon",blurb:"an armoured, ancient bottom-feeder that runs up from the sea.",bands:&[WarmTemperate,CoolTemperate,Boreal]},
    FishSpec{slug:"silverback-eel",name:"Silverback Eel",binomial:"Anguilla vitrea",zone:2,real:"eel",blurb:"a snake-bodied migrant that breeds far out in the ocean.",bands:&[WarmTemperate,CoolTemperate]},
    FishSpec{slug:"tidewater-shad",name:"Tidewater Shad",binomial:"Alosa aestuaria",zone:2,real:"shad",blurb:"a silver herring that runs up from the sea to spawn.",bands:&[WarmTemperate,CoolTemperate,Boreal]},
    // â”€â”€ Tropical roster (Af/Am/Aw) â”€â”€
    FishSpec{slug:"emberscale-tetra",name:"Emberscale Tetra",binomial:"Hyphessobrycon cataractae",zone:0,real:"tetra",blurb:"a jewel-bright shoaling characin of clear tropical headwaters.",bands:&[Tropical]},
    FishSpec{slug:"golden-pacu",name:"Golden Pacu",binomial:"Piaractus auratus",zone:1,real:"pacu",blurb:"a deep-bodied, seed-crushing herbivore of the tropical main river.",bands:&[Tropical]},
    FishSpec{slug:"sabertooth-payara",name:"Sabertooth Payara",binomial:"Hydrolycus dentex",zone:1,real:"payara",blurb:"a fanged silver predator of fast tropical rivers.",bands:&[Tropical]},
    FishSpec{slug:"king-arapaima",name:"King Arapaima",binomial:"Arapaima regalis",zone:2,real:"arapaima",blurb:"a giant air-breathing behemoth of tropical floodplains.",bands:&[Tropical]},
    FishSpec{slug:"emperor-cichlid",name:"Emperor Cichlid",binomial:"Cichla imperator",zone:2,real:"peacock bass",blurb:"a barred, eye-spotted cichlid of warm lowland rivers.",bands:&[Tropical,WarmTemperate]},
    FishSpec{slug:"whiskered-redtail",name:"Whiskered Redtail",binomial:"Phractocephalus igneus",zone:2,real:"redtail catfish",blurb:"a huge red-tailed catfish of the tropical lower river.",bands:&[Tropical]},
    // â”€â”€ Boreal / subarctic roster (Dfc/Dwc/ET) â”€â”€
    FishSpec{slug:"silverpike-taimen",name:"Silverpike Taimen",binomial:"Hucho borealis",zone:0,real:"taimen",blurb:"a giant cold-river salmonid, the 'river tiger' of the north.",bands:&[Boreal,CoolTemperate]},
    FishSpec{slug:"broad-whitefish",name:"Broad Whitefish",binomial:"Coregonus latus",zone:0,real:"whitefish",blurb:"a silvery cold-water shoaler of northern rivers and lakes.",bands:&[Boreal,Polar,CoolTemperate]},
    FishSpec{slug:"arctic-grayling",name:"Arctic Grayling",binomial:"Thymallus arcticus",zone:0,real:"Arctic grayling",blurb:"a sail-finned grayling of clear subarctic streams.",bands:&[Boreal,Polar]},
    FishSpec{slug:"boreal-burbot",name:"Boreal Burbot",binomial:"Lota borealis",zone:1,real:"burbot",blurb:"an eel-shaped freshwater cod of cold, deep water.",bands:&[Boreal,CoolTemperate]},
    FishSpec{slug:"northern-pike",name:"Northern Pike",binomial:"Esox borealis",zone:1,real:"pike",blurb:"a long-jawed ambush predator of northern rivers.",bands:&[Boreal,CoolTemperate]},
    FishSpec{slug:"blackfin-char",name:"Blackfin Char",binomial:"Salvelinus ater",zone:2,real:"char",blurb:"a dark, pale-spotted char of the coldest reaches.",bands:&[Boreal,Polar]},
    // â”€â”€ Continental / lowland variety (Dfa/Dfb â†’ cool-temperate & boreal) â”€â”€
    // A deep bench so neighbouring continental rivers roll DIFFERENT rosters.
    FishSpec{slug:"marble-trout",name:"Dappled Marble Trout",binomial:"Salmo marmoratus",zone:0,real:"marble trout",blurb:"a marble-patterned trout of cold, clear headwaters.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"rustscale-roach",name:"Rustscale Roach",binomial:"Rutilus ferrugineus",zone:1,real:"roach",blurb:"a red-finned silver shoaler, the commonest fish of the middle river.",bands:&[CoolTemperate,WarmTemperate,Boreal]},
    FishSpec{slug:"goldeye-ide",name:"Goldeye Ide",binomial:"Leuciscus auratus",zone:1,real:"ide",blurb:"a golden-eyed, thick-bodied cyprinid of steady lowland flow.",bands:&[CoolTemperate,Boreal]},
    FishSpec{slug:"blueback-vimba",name:"Blueback Vimba",binomial:"Vimba caerulea",zone:1,real:"vimba",blurb:"a bottom-grubbing bream-cousin that runs upriver to spawn.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"silverstreak-bleak",name:"Silverstreak Bleak",binomial:"Alburnus fulgens",zone:1,real:"bleak",blurb:"a tiny, flashing surface shoaler that dimples the calm water.",bands:&[CoolTemperate,WarmTemperate,Boreal]},
    FishSpec{slug:"barbeled-gudgeon",name:"Barbeled Gudgeon",binomial:"Gobio limosus",zone:1,real:"gudgeon",blurb:"a small, spotted bottom-forager of clean sandy runs.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"curveblade-sabrefish",name:"Curveblade Sabrefish",binomial:"Pelecus falcatus",zone:1,real:"sabrefish",blurb:"a sabre-bellied silver predator that hunts the open mid-river.",bands:&[CoolTemperate,WarmTemperate]},
    FishSpec{slug:"redwing-rudd",name:"Redwing Rudd",binomial:"Scardinius rubellus",zone:2,real:"rudd",blurb:"a golden, red-finned fish of warm weedy backwaters.",bands:&[CoolTemperate,WarmTemperate,Boreal]},
    FishSpec{slug:"thornback-ruffe",name:"Thornback Ruffe",binomial:"Gymnocephalus spinosus",zone:2,real:"ruffe",blurb:"a small, spiny bottom-percid of turbid lower rivers.",bands:&[CoolTemperate,Boreal]},
    FishSpec{slug:"mussel-bitterling",name:"Mussel Bitterling",binomial:"Rhodeus amarus",zone:2,real:"bitterling",blurb:"a thumb-sized fish that lays its eggs inside living freshwater mussels.",bands:&[WarmTemperate,CoolTemperate]},
    // â”€â”€ Migratory / megafish (RESERVED â€” only added to great trunks / estuaries) â”€â”€
    FishSpec{slug:"greatmaw-catfish",name:"Greatmaw Catfish",binomial:"Silurus giganteus",zone:2,real:"giant catfish",blurb:"a vast, whiskered predator that patrols the deep main channel.",bands:&[WarmTemperate,CoolTemperate,Boreal]},
    FishSpec{slug:"silverleap-salmon",name:"Silverleap Salmon",binomial:"Salmo saliens",zone:2,real:"salmon",blurb:"a silver anadromous salmonid that leaps upriver from the sea to spawn.",bands:&[CoolTemperate,Boreal,Polar]},
    FishSpec{slug:"delta-mullet",name:"Delta Mullet",binomial:"Mugil deltae",zone:2,real:"grey mullet",blurb:"a silver, salt-tolerant grazer that pushes up from the brackish delta.",bands:&[Tropical,WarmTemperate]},
    FishSpec{slug:"brackish-snook",name:"Brackish Snook",binomial:"Centropomus aestuarii",zone:2,real:"snook",blurb:"a line-flanked estuary ambusher running between salt and fresh.",bands:&[Tropical,WarmTemperate]},
    // â”€â”€ Tropical variety (Af/Am/Aw middle & delta) â”€â”€
    FishSpec{slug:"cataract-loach",name:"Cataract Hillstream Loach",binomial:"Sewellia cataractae",zone:0,real:"hillstream loach",blurb:"a sucker-bellied loach clinging to rocks in swift tropical headwaters.",bands:&[Tropical]},
    FishSpec{slug:"silver-bocachico",name:"Silver Bocachico",binomial:"Prochilodus argenteus",zone:1,real:"bocachico",blurb:"a detritus-grazing shoaler that migrates in vast silver runs.",bands:&[Tropical]},
    FishSpec{slug:"redhook-myleus",name:"Redhook Silver Dollar",binomial:"Myloplus rubripinnis",zone:1,real:"silver dollar",blurb:"a disc-bodied, red-finned relative of the pacu of tropical rivers.",bands:&[Tropical]},
    FishSpec{slug:"painted-eartheater",name:"Painted Eartheater",binomial:"Geophagus pictus",zone:2,real:"eartheater cichlid",blurb:"a sand-sifting cichlid of warm lowland channels and delta pools.",bands:&[Tropical,WarmTemperate]},
];

/// Arid-river / oasis roster (BW/BS KÃ¶ppen). Kept separate so it is only drawn
/// for reaches flagged arid â€” a desert river shows barbs and pupfish, not trout.
/// Bands span hot to cold deserts.
pub const ARID_FISH: &[FishSpec] = &[
    FishSpec{slug:"wadi-killifish",name:"Wadi Killifish",binomial:"Nothobranchius wadi",zone:0,real:"annual killifish",blurb:"a jewel-coloured killifish that races the drying season.",bands:&[Tropical,WarmTemperate]},
    FishSpec{slug:"desert-barb",name:"Desert Barb",binomial:"Barbus deserticola",zone:1,real:"barb",blurb:"a hardy brassy barb of warm, turbid desert rivers.",bands:&[Tropical,WarmTemperate,CoolTemperate]},
    FishSpec{slug:"oasis-tilapia",name:"Oasis Tilapia",binomial:"Oreochromis aridus",zone:1,real:"tilapia",blurb:"a tough, tolerant cichlid of oases and warm pools.",bands:&[Tropical,WarmTemperate]},
    FishSpec{slug:"sand-catfish",name:"Sand Catfish",binomial:"Bagrus harenae",zone:2,real:"catfish",blurb:"a pale, whiskered catfish of muddy desert-river beds.",bands:&[Tropical,WarmTemperate,CoolTemperate]},
    FishSpec{slug:"saltcreek-pupfish",name:"Saltcreek Pupfish",binomial:"Cyprinodon deserti",zone:2,real:"pupfish",blurb:"a tiny, heat- and salt-hardy fish of shrinking desert pools.",bands:&[Tropical,WarmTemperate,CoolTemperate]},
    FishSpec{slug:"wadi-mullet",name:"Wadi Mullet",binomial:"Liza deserti",zone:2,real:"grey mullet",blurb:"a silver, salt-tolerant mullet of brackish desert mouths.",bands:&[Tropical,WarmTemperate,CoolTemperate]},
];

/// Assign one signature species per river ZONE the reach spans, filtered to the
/// reach's thermal band. Headwater sources start in zone 0, sizeable rivers gain a
/// middle (zone 1) reach, and delta/estuary or big mouths reach zone 2. Arid
/// reaches draw from the desert roster (barbs, pupfish) instead of the main one.
/// Returns `&'static FishSpec` references (deterministic per `seed`); empty only
/// when no catalogued species fits the band (e.g. an ice-cap polar river).
pub fn assign_river_fish(band: Band, source_kind: &str, mouth_kind: u8, discharge: f32, arid: bool, seed: u64) -> Vec<&'static FishSpec> {
    let cat: &'static [FishSpec] = if arid { ARID_FISH } else { SIGNATURE_FISH };
    let mut zones: Vec<u8> = Vec::new();
    let src_zone = match source_kind {
        "alpine" | "highland" => 0u8,
        "hills" => 1,
        _ => if discharge < 150.0 { 1 } else { 2 },
    };
    zones.push(src_zone);
    if discharge >= 150.0 && !zones.contains(&1) { zones.push(1); }
    let mouth_zone = if mouth_kind != 0 || discharge >= 800.0 { 2u8 } else { 1 };
    if !zones.contains(&mouth_zone) { zones.push(mouth_zone); }
    zones.sort_unstable();

    let mut r = Rng(seed ^ 0x00F1_5100_0000_0001);
    let mut out: Vec<&'static FishSpec> = Vec::new();
    for &z in &zones {
        let cands: Vec<&'static FishSpec> = cat.iter()
            .filter(|f| f.zone == z && f.bands.contains(&band))
            .collect();
        if !cands.is_empty() {
            let pick = cands[(r.next() % cands.len() as u64) as usize];
            if !out.iter().any(|f| f.slug == pick.slug) { out.push(pick); }
        }
    }
    out
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ Full per-REACH assemblage (one roster per upper/middle/delta) â”€
// Where `assign_river_fish` returns one signature species per zone the whole
// river spans, this returns the FULL assemblage a naturalist would list for a
// SINGLE reach (upper / middle / delta), so the panel can show every fish that
// lives there. Ecologically correct + seed-varied so neighbouring continental
// rivers roll different rosters (the user's "add variety for continental").
//
//  * base pool  â€” every catalogued species of this zoneÃ—band, MINUS the reserved
//    migrants below; a seed-shuffled subset is taken so rivers differ.
//  * megafish   â€” sturgeon / giant catfish (arapaima & redtail in the tropics)
//    ONLY on a `great` trunk's middle & delta.
//  * diadromous â€” salmon / shad / eel (mullet & snook in the tropics) ONLY at an
//    `estuary`/delta reach.

/// Reserved migrant slugs â€” kept OUT of the base pool so a small river's delta
/// doesn't sprout sturgeon or salmon; re-added only for great/estuary reaches.
fn is_reserved_migrant(slug: &str) -> bool {
    matches!(slug,
        "silt-sturgeon" | "greatmaw-catfish" | "king-arapaima" | "whiskered-redtail" |
        "silverleap-salmon" | "tidewater-shad" | "silverback-eel" | "delta-mullet" | "brackish-snook")
}

/// Megafish that haunt the deep trunk of a GREAT river (band-appropriate).
fn megafish_slugs(band: Band) -> &'static [&'static str] {
    match band {
        Tropical => &["king-arapaima", "whiskered-redtail"],
        WarmTemperate | CoolTemperate | Boreal => &["silt-sturgeon", "greatmaw-catfish"],
        Polar => &[],
    }
}

/// Diadromous fish that run in from the sea at an estuary/delta (band-appropriate).
fn run_slugs(band: Band) -> &'static [&'static str] {
    match band {
        Tropical => &["delta-mullet", "brackish-snook"],
        WarmTemperate => &["tidewater-shad", "silverback-eel", "brackish-snook"],
        CoolTemperate => &["silverleap-salmon", "tidewater-shad", "silverback-eel"],
        Boreal => &["silverleap-salmon", "tidewater-shad"],
        Polar => &["silverleap-salmon"],
    }
}

/// Seed-pick up to `n` species out of `cands` (removing them), deterministic.
fn shuffle_take<'a>(cands: &mut Vec<&'a FishSpec>, r: &mut Rng, n: usize) -> Vec<&'a FishSpec> {
    let mut out = Vec::new();
    let take = n.min(cands.len());
    for _ in 0..take {
        let i = (r.next() % cands.len() as u64) as usize;
        out.push(cands.remove(i));
    }
    out
}

/// The full fish assemblage of ONE reach. `zone` 0 upper Â· 1 middle Â· 2 delta;
/// `great` = a big trunk (megafish), `estuary` = tidal/delta mouth (diadromous runs).
pub fn assign_reach_fish(band: Band, zone: u8, great: bool, estuary: bool, arid: bool, seed: u64) -> Vec<&'static FishSpec> {
    if arid {
        let mut c: Vec<&'static FishSpec> = ARID_FISH.iter()
            .filter(|f| f.zone == zone && f.bands.contains(&band)).collect();
        if c.is_empty() {
            c = ARID_FISH.iter().filter(|f| f.bands.contains(&band)).collect();
        }
        let mut r = Rng(seed ^ 0x00A1_D000_0000_0001);
        return shuffle_take(&mut c, &mut r, 3);
    }
    let mut cands: Vec<&'static FishSpec> = SIGNATURE_FISH.iter()
        .filter(|f| f.zone == zone && f.bands.contains(&band) && !is_reserved_migrant(f.slug))
        .collect();
    let mut r = Rng(seed ^ 0x00F1_5EAC_0000_0001);
    let base_n = if zone == 0 { 3 } else { 4 };
    let mut out = shuffle_take(&mut cands, &mut r, base_n);
    if great && zone >= 1 {
        for slug in megafish_slugs(band) {
            if let Some(f) = fish_by_slug(slug) {
                if f.bands.contains(&band) && !out.iter().any(|e| e.slug == f.slug) { out.push(f); }
            }
        }
    }
    if estuary && zone == 2 {
        for slug in run_slugs(band) {
            if let Some(f) = fish_by_slug(slug) {
                if f.bands.contains(&band) && !out.iter().any(|e| e.slug == f.slug) { out.push(f); }
            }
        }
    }
    out
}

/// Prose fish-list for a reach, built from its assigned species' real-world names
/// (so the reach's sentence and its plates always name the SAME fish).
pub fn reach_fish_prose(species: &[&FishSpec]) -> String {
    if species.is_empty() { return "only a few hardy fish".into(); }
    let mut uniq: Vec<String> = Vec::new();
    for f in species {
        let n = f.real.to_string();
        if !uniq.contains(&n) { uniq.push(n); }
    }
    join_and(&uniq)
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ Signature fish species (lake rosters) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Lake-only species (cichlid flocks, the freshwater seal, brine shrimp, lake
// troutâ€¦), reusing the same FishSpec shape so the frontend plate machinery is
// shared. `zone` is repurposed as a rough depth niche (0 shallows Â· 1 open water Â·
// 2 deep) and is not shown for lakes. River-roster slugs are reused directly where
// a species also lives in lakes (pike, perch, whitefish, char, bream, tenchâ€¦), so
// only genuinely lake-signature fish are added here.
pub const LAKE_FISH: &[FishSpec] = &[
    // Rift / great-lake cichlid flock (tropical)
    FishSpec{slug:"mosaic-cichlid",name:"Mosaic Cichlid",binomial:"Haplochromis tessellatus",zone:0,real:"haplochromine cichlid",blurb:"one of an explosive flock of jewel-coloured cichlids found in this lake alone.",bands:&[Tropical]},
    FishSpec{slug:"azure-mbuna",name:"Azure Mbuna",binomial:"Maylandia azurea",zone:0,real:"mbuna cichlid",blurb:"a rock-grazing cichlid in electric blue, endemic to the rocky shore.",bands:&[Tropical]},
    FishSpec{slug:"marsh-lungfish",name:"Marsh Lungfish",binomial:"Protopterus paludis",zone:2,real:"lungfish",blurb:"an air-gulping relic that survives the dry season buried in mud.",bands:&[Tropical,WarmTemperate]},
    // Deep rift (cold) â€” relict fauna + the freshwater seal
    FishSpec{slug:"abyss-sculpin",name:"Abyss Sculpin",binomial:"Cottus abyssalis",zone:2,real:"deep-water sculpin",blurb:"a relict sculpin of the cold, lightless depths.",bands:&[Boreal,CoolTemperate]},
    FishSpec{slug:"silver-nerpa",name:"Silver Nerpa",binomial:"Pusa argentea",zone:1,real:"freshwater seal",blurb:"the world's only wholly freshwater seal, hauling out on the ice.",bands:&[Boreal,CoolTemperate]},
    FishSpec{slug:"pallid-omul",name:"Pallid Omul",binomial:"Coregonus pallidus",zone:1,real:"omul whitefish",blurb:"an endemic whitefish shoaling in the cold open water.",bands:&[Boreal,CoolTemperate,Polar]},
    // Glacial great lakes
    FishSpec{slug:"graven-laketrout",name:"Graven Lake Trout",binomial:"Salvelinus namaycush",zone:2,real:"lake trout",blurb:"a big, deep-dwelling char of cold, clear lakes.",bands:&[CoolTemperate,Boreal]},
    FishSpec{slug:"glass-cisco",name:"Glass Cisco",binomial:"Coregonus vitreus",zone:1,real:"cisco",blurb:"a silvery plankton-feeder of the cold pelagic zone.",bands:&[CoolTemperate,Boreal]},
    // Tropical great (Victoria-like) + lowland reedy lakes
    FishSpec{slug:"goldshoal-tilapia",name:"Goldshoal Tilapia",binomial:"Oreochromis auratus",zone:0,real:"tilapia",blurb:"a hardy, prolific cichlid crowding the warm, productive shallows.",bands:&[Tropical,WarmTemperate]},
    FishSpec{slug:"reed-tench",name:"Reed Tench",binomial:"Tinca palustris",zone:0,real:"tench",blurb:"an olive, slime-scaled bottom-fish of warm, weedy shallows.",bands:&[WarmTemperate,CoolTemperate]},
    // Salt / hypersaline
    FishSpec{slug:"brine-shrimp",name:"Brine Shrimp",binomial:"Artemia salina",zone:1,real:"brine shrimp",blurb:"not a fish at all â€” a tiny crustacean that reddens the brine and feeds the flamingoes.",bands:&[Tropical,WarmTemperate,CoolTemperate,Boreal,Polar]},
    FishSpec{slug:"lakeshore-pupfish",name:"Lakeshore Pupfish",binomial:"Cyprinodon lacustris",zone:0,real:"pupfish",blurb:"a thumb-sized survivor of hot, salty desert water.",bands:&[Tropical,WarmTemperate]},
];

/// Find a fish spec by slug across every roster (river / arid / lake), so lake
/// assemblages can reuse river species that also live in lakes.
fn fish_by_slug(slug: &str) -> Option<&'static FishSpec> {
    SIGNATURE_FISH.iter().chain(ARID_FISH).chain(LAKE_FISH).find(|f| f.slug == slug)
}

/// Assign a lake's signature fish â€” a VARIABLE-length roster keyed to lake type Ã—
/// thermal band (a rift lake shows a flock of several, a tarn one, a hypersaline
/// lake only brine shrimp). Deterministic; band-filtered so no fish appears out of
/// its climate. Returns `&'static FishSpec`, empty only for a truly barren water.
pub fn assign_lake_fish(kind: LakeKind, band: Band, salinity_ppt: f32) -> Vec<&'static FishSpec> {
    // Candidate slugs per lake type (ordered best-first); band filtering + the
    // per-type cap below produce the variable count.
    let (want, cap): (&[&str], usize) = match kind {
        LakeKind::Rift => if band == Band::Tropical {
            (&["mosaic-cichlid", "azure-mbuna", "marsh-lungfish", "whiskered-redtail", "goldshoal-tilapia"], 5)
        } else {
            (&["silver-nerpa", "abyss-sculpin", "pallid-omul", "broad-whitefish", "boreal-burbot"], 5)
        },
        LakeKind::TropicalGreat => (&["goldshoal-tilapia", "mosaic-cichlid", "marsh-lungfish", "azure-mbuna"], 4),
        LakeKind::Glacial => (&["graven-laketrout", "glass-cisco", "broad-whitefish", "northern-pike", "marbled-perch"], 5),
        LakeKind::Lowland => (&["reed-tench", "marbled-perch", "broadscale-bream", "northern-pike", "reedwater-zander"], 5),
        LakeKind::Crater => if matches!(band, Band::Boreal | Band::Polar | Band::CoolTemperate) {
            (&["redflank-char"], 1)
        } else {
            (&["goldshoal-tilapia"], 2)
        },
        LakeKind::SaltEndorheic => if salinity_ppt >= 120.0 {
            (&["brine-shrimp"], 1) // hypersaline: brine shrimp & flamingoes, no fish
        } else {
            (&["brine-shrimp", "lakeshore-pupfish", "oasis-tilapia"], 3)
        },
        LakeKind::Tarn => if band == Band::Tropical {
            (&["goldshoal-tilapia"], 1)
        } else {
            (&["redflank-char"], 1)
        },
    };
    let mut out: Vec<&'static FishSpec> = Vec::new();
    for &slug in want {
        if out.len() >= cap { break; }
        if let Some(f) = fish_by_slug(slug) {
            // Brine shrimp ignore the band gate (a crustacean, present at any temp).
            if f.slug == "brine-shrimp" || f.bands.contains(&band) {
                if !out.iter().any(|e| e.slug == f.slug) { out.push(f); }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperate_river_gets_zoned_species() {
        // An alpine-sourced, delta-mouthed, large cool-temperate river should get a
        // headwater + middle + lower species, all band-appropriate.
        let sp = assign_river_fish(Band::CoolTemperate, "alpine", 1, 5000.0, false, 42);
        assert!(sp.len() >= 2, "a great river spans multiple zones: {}", sp.len());
        let zones: Vec<u8> = sp.iter().map(|f| f.zone).collect();
        assert!(zones.contains(&0) && zones.contains(&2), "spans headwater and mouth: {:?}", zones);
        for f in &sp { assert!(f.bands.contains(&Band::CoolTemperate)); }
    }

    #[test]
    fn tropical_and_arid_rivers_get_their_own_rosters() {
        // Tropical rivers now have a roster (arapaima etc.).
        let trop = assign_river_fish(Band::Tropical, "hills", 1, 5000.0, false, 1);
        assert!(!trop.is_empty(), "tropical river has signature species");
        for f in &trop { assert!(f.bands.contains(&Band::Tropical)); }
        // An arid reach draws desert fish, never a trout.
        let arid = assign_river_fish(Band::WarmTemperate, "hills", 0, 300.0, true, 2);
        assert!(!arid.is_empty(), "arid river has desert species");
        assert!(arid.iter().all(|f| ARID_FISH.iter().any(|a| a.slug == f.slug)), "arid roster used");
    }

    #[test]
    fn reach_assemblage_is_multi_and_correct() {
        // A great cool-temperate delta lists a whole assemblage incl. migrants.
        let delta = assign_reach_fish(Band::CoolTemperate, 2, true, true, false, 7);
        assert!(delta.len() >= 4, "a great delta lists many fish: {}", delta.len());
        assert!(delta.iter().any(|f| f.real == "salmon"), "estuary run includes salmon");
        assert!(delta.iter().any(|f| f.real == "giant catfish" || f.real == "sturgeon"),
            "a great trunk has megafish");
        for f in &delta { assert!(f.bands.contains(&Band::CoolTemperate)); }
        // A small plain headwater is a short list â€” no sturgeon, no salmon.
        let up = assign_reach_fish(Band::CoolTemperate, 0, false, false, false, 3);
        assert!(!up.is_empty() && up.iter().all(|f| f.zone == 0));
        assert!(!up.iter().any(|f| is_reserved_migrant(f.slug)), "no migrants in a plain headwater");
        // The reach's prose names the SAME species as its plates.
        let prose = reach_fish_prose(&up);
        assert!(prose.contains(up[0].real), "prose lists the assigned species: {prose}");
    }

    /// Dump the whole fish catalogue to JSON for the standalone HTML gallery
    /// generator (`scripts/gen_fish_gallery.mjs`). Ignored by default â€” run with
    /// `cargo test --lib emit_fish_catalogue -- --ignored --nocapture` to refresh.
    #[test]
    #[ignore]
    fn emit_fish_catalogue() {
        fn band_str(b: Band) -> &'static str {
            match b {
                Tropical => "Tropical", WarmTemperate => "WarmTemperate",
                CoolTemperate => "CoolTemperate", Boreal => "Boreal", Polar => "Polar",
            }
        }
        fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }
        fn one(group: &str, f: &FishSpec) -> String {
            let bands: Vec<String> = f.bands.iter().map(|b| format!("\"{}\"", band_str(*b))).collect();
            format!("{{\"group\":\"{}\",\"slug\":\"{}\",\"name\":\"{}\",\"binomial\":\"{}\",\"zone\":{},\"real\":\"{}\",\"blurb\":\"{}\",\"bands\":[{}]}}",
                group, esc(f.slug), esc(f.name), esc(f.binomial), f.zone, esc(f.real), esc(f.blurb), bands.join(","))
        }
        let mut items: Vec<String> = Vec::new();
        for f in SIGNATURE_FISH { items.push(one("river", f)); }
        for f in ARID_FISH { items.push(one("arid", f)); }
        for f in LAKE_FISH { items.push(one("lake", f)); }
        let json = format!("[\n  {}\n]\n", items.join(",\n  "));
        let path = std::env::var("FISH_CATALOGUE_OUT")
            .unwrap_or_else(|_| "../scripts/fish_catalogue.json".to_string());
        std::fs::write(&path, json).expect("write fish_catalogue.json");
        eprintln!("wrote {} fish to {}", items.len(), path);
    }

    #[test]
    fn bands_split_by_temp() {
        assert_eq!(band_from_temp(27.0), Band::Tropical);
        assert_eq!(band_from_temp(10.0), Band::CoolTemperate);
        assert_eq!(band_from_temp(-10.0), Band::Polar);
    }

    #[test]
    fn great_tropical_river_has_megafish_and_run() {
        let s = river_fish(Band::Tropical, 1.0, 8000.0, true);
        assert!(s.contains("arapaima"), "great tropical river names megafish: {s}");
        assert!(s.contains("delta") || s.contains("mullet"), "estuary run present: {s}");
    }

    #[test]
    fn steep_cold_headwater_is_trout_water() {
        let s = river_fish(Band::CoolTemperate, 20.0, 5.0, false);
        assert!(s.contains("trout"), "steep cold creek is trout water: {s}");
    }

    #[test]
    fn baikal_like_basin_classifies_as_rift() {
        let f = LakeFacts {
            area_km2: 400.0, max_depth_m: 900.0, elev_m: 455.0, abs_lat: 53.0,
            mean_temp_c: 2.0, volcanic: false, on_boundary: true, endorheic: false,
            arid: false, elongated: true,
        };
        assert_eq!(classify_lake(&f), LakeKind::Rift);
        assert!(lake_fish(LakeKind::Rift, Band::Boreal).contains("seal"));
    }

    #[test]
    fn arid_terminal_basin_is_salt_lake() {
        let f = LakeFacts {
            area_km2: 900.0, max_depth_m: 20.0, elev_m: -20.0, abs_lat: 45.0,
            mean_temp_c: 12.0, volcanic: false, on_boundary: false, endorheic: true,
            arid: true, elongated: false,
        };
        assert_eq!(classify_lake(&f), LakeKind::SaltEndorheic);
    }

    #[test]
    fn zone_story_names_biome_and_tributaries() {
        let tribs = vec![("the Neris".to_string(), 210.0), ("the Venta".to_string(), 540.0)];
        let s = zone_story(&ZoneFacts {
            kind: 1, name: "the Nemunas", biome: "temperate broadleaf forest",
            fish: "barbel and nase", tribs: &tribs, mouth_kind: 0, band: Band::CoolTemperate, seed: 7,
        });
        assert!(s.contains("temperate broadleaf forest"), "zone names its biome: {s}");
        assert!(s.contains("Neris") && s.contains("210 km"), "confluence named at its km: {s}");
        // The delta reach reads as an abundant mouth.
        let d = zone_story(&ZoneFacts {
            kind: 2, name: "the Po", biome: "humid subtropical forest", fish: "sturgeon and shad",
            tribs: &[], mouth_kind: 1, band: Band::WarmTemperate, seed: 3,
        });
        assert!(d.to_lowercase().contains("delta"), "delta reach mentions the delta: {d}");
    }

    #[test]
    fn river_summary_weaves_journey_and_counterpart() {
        let s = river_summary(&RiverSummaryFacts {
            name: "the Daugava", discharge_m3s: 680.0, source_kind: "hills", mouth_kind: 2,
            trib_total: 6, journey: "boreal taiga, then temperate broadleaf forest",
            counterpart: "the Nemunas", length_km: 1020.0, navigable: true, seed: 11,
        });
        assert!(s.contains("the Daugava"), "names the river: {s}");
        assert!(s.contains("boreal taiga"), "weaves the climate journey: {s}");
        assert!(s.contains("1020") || s.contains("1,020") || s.contains("1020 km"), "states length: {s}");
    }

    #[test]
    fn biome_and_code_agree() {
        use crate::sim::koppen;
        assert_eq!(koppen_to_biome(koppen::DFC), "boreal taiga");
        assert_eq!(koppen_code(koppen::DFC), "Dfc");
        assert_eq!(koppen_to_biome(koppen::CSA), "Mediterranean scrub");
    }

    #[test]
    fn lake_fish_vary_by_type_and_band() {
        // A tropical rift lake shows a multi-species cichlid flockâ€¦
        let rift = assign_lake_fish(LakeKind::Rift, Band::Tropical, 0.2);
        assert!(rift.len() >= 3, "rift lake shows a flock: {}", rift.len());
        assert!(rift.iter().any(|f| f.real.contains("cichlid")), "rift flock has cichlids");
        // â€¦a tarn shows a single cold-water fishâ€¦
        let tarn = assign_lake_fish(LakeKind::Tarn, Band::Boreal, 0.2);
        assert_eq!(tarn.len(), 1, "a tarn is a one-fish water");
        // â€¦and a hypersaline lake shows only brine shrimp (no fish).
        let brine = assign_lake_fish(LakeKind::SaltEndorheic, Band::WarmTemperate, 150.0);
        assert_eq!(brine.len(), 1, "hypersaline: brine shrimp only");
        assert_eq!(brine[0].slug, "brine-shrimp");
        // Every assigned fish suits the lake's band (brine shrimp excepted).
        for f in &rift { assert!(f.bands.contains(&Band::Tropical)); }
        // A milder salt lake supports a few halotolerant specialists.
        let salty = assign_lake_fish(LakeKind::SaltEndorheic, Band::WarmTemperate, 45.0);
        assert!(salty.len() >= 2, "a brackish salt lake has a few specialists");
    }

    #[test]
    fn shallow_warm_wide_lake_is_tropical_great() {
        let f = LakeFacts {
            area_km2: 60000.0, max_depth_m: 40.0, elev_m: 1130.0, abs_lat: 1.0,
            mean_temp_c: 24.0, volcanic: false, on_boundary: false, endorheic: false,
            arid: false, elongated: false,
        };
        assert_eq!(classify_lake(&f), LakeKind::TropicalGreat);
    }
}



