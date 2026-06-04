//! Antique place-name generator — Roman, Greek, Phoenician/Punic and Persian
//! styles. Names are a deterministic function of cell position + world size only
//! (no external seed), so a settlement and the trade hub that sits on the same
//! cell always resolve to the SAME name across the settlement, political and
//! economy layers.
//!
//! Culture is assigned by coarse spatial "province" so neighbouring settlements
//! share a regional naming style (a Greek coast here, a Persian interior there),
//! rather than each town reading from a random culture.

/// The four antique naming cultures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Culture {
    Roman,
    Greek,
    Phoenician,
    Persian,
}

impl Culture {
    pub fn label(self) -> &'static str {
        match self {
            Culture::Roman => "Roman",
            Culture::Greek => "Greek",
            Culture::Phoenician => "Phoenician",
            Culture::Persian => "Persian",
        }
    }
}

// ── Syllable banks ───────────────────────────────────────────────────────────
// Each culture has onsets (name starts), optional mid joins, and endings.

const ROMAN_ON: [&str; 16] = [
    "Aqu", "Por", "Nov", "Ver", "Cas", "Lav", "Tarr", "Bened", "Aug", "Salt",
    "Ostia", "Vol", "Arr", "Pomp", "Luc", "Mar",
];
const ROMAN_MID: [&str; 6] = ["", "i", "e", "en", "el", "ar"];
const ROMAN_END: [&str; 12] = [
    "ium", "a", "entia", "onium", "aria", "anum", "ona", "ina", "etia", "olum",
    "estum", "icum",
];

const GREEK_ON: [&str; 16] = [
    "Meg", "Thess", "Korin", "Pyr", "Hali", "Eph", "Mil", "Delph", "Olymp", "Argos",
    "Naxa", "Therm", "Knoss", "Pell", "Syrak", "Mykon",
];
const GREEK_MID: [&str; 6] = ["", "a", "i", "o", "ar", "is"];
const GREEK_END: [&str; 12] = [
    "os", "on", "aia", "polis", "andros", "ia", "ene", "yssa", "kos", "thea",
    "nthos", "kleia",
];

const PUNIC_ON: [&str; 16] = [
    "Qart", "Gad", "Tyr", "Sid", "Byr", "Mel", "Utix", "Lep", "Mot", "Pan",
    "Sab", "Mag", "Tharr", "Hadr", "Bos", "Kart",
];
const PUNIC_MID: [&str; 6] = ["", "a", "i", "o", "ad", "as"];
const PUNIC_END: [&str; 12] = [
    "ada", "on", "ath", "qart", "im", "tis", "uba", "esh", "ar", "anto",
    "umet", "shama",
];

const PERSIAN_ON: [&str; 16] = [
    "Pasar", "Ekba", "Susa", "Persa", "Zara", "Bakh", "Ragha", "Hyrka", "Nisa", "Kuru",
    "Asha", "Dara", "Anshan", "Gaba", "Tushpa", "Mithra",
];
const PERSIAN_MID: [&str; 6] = ["", "a", "i", "an", "ar", "o"];
const PERSIAN_END: [&str; 12] = [
    "gard", "kana", "dana", "shahr", "abad", "stan", "kert", "vana", "thra", "spa",
    "drava", "mada",
];

/// splitmix64-style finalizer → u64 (deterministic, well-mixed).
fn hash64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    x
}

/// Culture of the province containing `(x, y)` in a `w×h` world. Provinces are a
/// coarse grid (~5 across, ~4 down) so neighbouring places share a style.
pub fn culture_at(x: u32, y: u32, w: u32, h: u32) -> Culture {
    let pw = (w / 5).max(1);
    let ph = (h / 4).max(1);
    let px = (x / pw) as u64;
    let py = (y / ph) as u64;
    let v = hash64(0x501E_C0DE_u64 ^ px.wrapping_mul(0x9E3779B1) ^ py.wrapping_mul(0x85EBCA77));
    match v % 4 {
        0 => Culture::Roman,
        1 => Culture::Greek,
        2 => Culture::Phoenician,
        _ => Culture::Persian,
    }
}

fn pick<'a>(bank: &[&'a str], h: u64) -> &'a str {
    bank[(h % bank.len() as u64) as usize]
}

/// Generate an antique place name for the settlement / hub at `(x, y)` in a
/// `w×h` world. Deterministic; the same cell always yields the same name.
pub fn gen_name(x: u32, y: u32, w: u32, h: u32) -> String {
    let culture = culture_at(x, y, w, h);
    let base = hash64((x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663));
    let (on, mid, end) = match culture {
        Culture::Roman => (&ROMAN_ON[..], &ROMAN_MID[..], &ROMAN_END[..]),
        Culture::Greek => (&GREEK_ON[..], &GREEK_MID[..], &GREEK_END[..]),
        Culture::Phoenician => (&PUNIC_ON[..], &PUNIC_MID[..], &PUNIC_END[..]),
        Culture::Persian => (&PERSIAN_ON[..], &PERSIAN_MID[..], &PERSIAN_END[..]),
    };
    let o = pick(on, base);
    let m = pick(mid, base >> 8);
    let e = pick(end, base >> 16);
    let raw = format!("{}{}{}", o, m, e);
    // Collapse any stray double spaces / trim (a couple of endings carry a space).
    let name: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    name
}

/// Generate a name plus an epithet for a major place (a capital / top hub), e.g.
/// "Aquentia Magna". `tier` 0 = none, higher = grander.
pub fn gen_name_epithet(x: u32, y: u32, w: u32, h: u32, tier: u8) -> String {
    let name = gen_name(x, y, w, h);
    if tier == 0 {
        return name;
    }
    let culture = culture_at(x, y, w, h);
    let epithets: &[&str] = match culture {
        Culture::Roman => &["Magna", "Augusta", "Maior"],
        Culture::Greek => &["Megale", "Hypsele", "Akra"],
        Culture::Phoenician => &["Rabba", "the Great Harbour", "Adir"],
        Culture::Persian => &["the Great", "Shahanshah", "Buzurg"],
    };
    let h2 = hash64((x as u64) ^ (y as u64).wrapping_mul(0xABCD));
    format!("{} {}", name, epithets[(h2 as usize) % epithets.len()])
}
