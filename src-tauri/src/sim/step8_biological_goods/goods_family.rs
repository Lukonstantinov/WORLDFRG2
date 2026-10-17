//! GOODS FAMILIES (owner decisions 2026-10-17, `docs/PROVINCES_REALMS_LOGISTICS_REDESIGN.md` §7).
//!
//! A family is several kinds of the same material: fleece and llama wool, every gem,
//! gold and silver, the spices. Within a family:
//! - a recipe may use ANY member (a jeweller uses whatever gem reaches the city), and
//!   the product is named after what it was made from ("Carnelian-gold jewelry");
//! - a people's taste for the family resolves to its own favourite member.
//! Members keep their own quality, deposits and price — a family only shares use.

use super::goods_spec::GoodSpec;

/// The family a shipped good belongs to, if any.
pub fn family_of(id: &str) -> Option<&'static str> {
    Some(match id {
        "wool_fleece" | "wool_llama" => "wool",
        "cotton" | "flax" | "hemp" => "plant_fibre",
        "dyes" | "indigo" | "tyrian_purple" => "dye",
        "gemstones" | "ruby" | "sapphire" | "emerald" | "diamond" | "amethyst" | "topaz" | "garnet"
        | "carnelian" | "lapis_lazuli" | "turquoise" | "jade" => "gem",
        "gold" | "silver" => "bullion",
        "spices" | "pepper" | "cloves" | "cinnamon" | "nutmeg" | "mace" | "saffron" => "spice",
        "incense" | "frankincense" | "benzoin" | "camphor" => "incense",
        "salt" | "bay_salt" => "salt",
        "iron" | "bog_iron" => "iron",
        "timber" | "hardwoods" => "timber",
        "stockfish" | "herring" => "fish",
        _ => return None,
    })
}

/// A spec's family: the shipped table, else (for a custom good) its category when
/// that category was already a fungible input group (`preservative`, `gem`), so
/// custom goods keep the substitution they had before families existed.
pub fn family_key(spec: &GoodSpec) -> Option<String> {
    family_of(&spec.id).map(str::to_string).or_else(|| {
        crate::sim::manufacture::is_fungible_input_category(&spec.category).then(|| spec.category.clone())
    })
}

/// The word a material lends a product's name: "llama-wool", "carnelian", "gold".
pub fn material_word(id: &str) -> String {
    match id {
        "wool_fleece" => "fleece-wool".into(),
        "wool_llama" => "llama-wool".into(),
        "dyes" => "murex".into(),
        "tyrian_purple" => "purple".into(),
        "lapis_lazuli" => "lapis".into(),
        "bay_salt" => "sea-salt".into(),
        "salt" => "rock-salt".into(),
        "bog_iron" => "bog-iron".into(),
        "hardwoods" => "hardwood".into(),
        "stockfish" => "cod".into(),
        other => other.replace('_', " "),
    }
}

/// The name prefix for a made good from the materials actually used, most important
/// first: "Carnelian-gold", "Llama-wool". Empty when nothing to say.
pub fn material_label(ids: &[&str]) -> String {
    let words: Vec<String> = ids.iter().map(|i| material_word(i)).collect();
    let s = words.join("-");
    let mut c = s.chars();
    match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() }
}

#[cfg(test)]
mod goods_family_tests {
    use super::*;

    #[test]
    fn goods_families_group_kinds_of_one_material_and_name_products() {
        assert_eq!(family_of("wool_llama"), family_of("wool_fleece"));
        assert_eq!(family_of("carnelian"), Some("gem"));
        assert_eq!(family_of("gold"), family_of("silver"));
        assert_eq!(family_of("wheat"), None, "grain substitutes in the market, not as a recipe family");
        assert_eq!(material_label(&["carnelian", "gold"]), "Carnelian-gold");
        assert_eq!(material_label(&["wool_llama"]), "Llama-wool");
        assert_eq!(material_label(&[]), "");
    }
}
