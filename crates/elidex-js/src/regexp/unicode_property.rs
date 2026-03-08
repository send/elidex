//! ES2024 §22.2.2.9 — Unicode property name/value validation for `\p{...}` / `\P{...}`.
//!
//! Tables derived from Unicode 16.0 specification files:
//! - PropertyAliases.txt  <https://www.unicode.org/Public/16.0.0/ucd/PropertyAliases.txt>
//! - PropertyValueAliases.txt  <https://www.unicode.org/Public/16.0.0/ucd/PropertyValueAliases.txt>
//!
//! # Maintenance
//!
//! When a new Unicode version is released (typically annually):
//! 1. Download the new PropertyAliases.txt and PropertyValueAliases.txt.
//! 2. Diff against the previous version to identify new Script names,
//!    `General_Category` values, or binary properties.
//! 3. Append new entries to the appropriate const table below.
//! 4. Update the "Unicode X.Y" version comment at the top of this file.
//! 5. Run `cargo test -p elidex-js` to verify no regressions.
//!
//! Entries are never removed (Unicode stability policy guarantees backwards compatibility).

/// Non-binary property names and their aliases (ES2024 Table 68).
/// Format: `(canonical_name, alias)`.
const NON_BINARY_PROPERTIES: &[(&str, &str)] = &[
    ("General_Category", "gc"),
    ("Script", "sc"),
    ("Script_Extensions", "scx"),
];

/// Binary property names (ES2024 Table 69).
/// Includes both canonical names and aliases recognized by the spec.
/// Sorted alphabetically for readability; lookup uses linear scan (table is small).
const BINARY_PROPERTIES: &[&str] = &[
    // Canonical names
    "ASCII",
    "ASCII_Hex_Digit",
    "AHex",
    "Alphabetic",
    "Alpha",
    "Any",
    "Assigned",
    "Bidi_Control",
    "Bidi_C",
    "Bidi_Mirrored",
    "Bidi_M",
    "Case_Ignorable",
    "CI",
    "Cased",
    "Changes_When_Casefolded",
    "CWCF",
    "Changes_When_Casemapped",
    "CWCM",
    "Changes_When_Lowercased",
    "CWL",
    "Changes_When_NFKC_Casefolded",
    "CWKCF",
    "Changes_When_Titlecased",
    "CWT",
    "Changes_When_Uppercased",
    "CWU",
    "Dash",
    "Default_Ignorable_Code_Point",
    "DI",
    "Deprecated",
    "Dep",
    "Diacritic",
    "Dia",
    "Emoji",
    "Emoji_Component",
    "EComp",
    "Emoji_Modifier",
    "EMod",
    "Emoji_Modifier_Base",
    "EBase",
    "Emoji_Presentation",
    "EPres",
    "Extended_Pictographic",
    "ExtPict",
    "Extender",
    "Ext",
    "Grapheme_Base",
    "Gr_Base",
    "Grapheme_Extend",
    "Gr_Ext",
    "Hex_Digit",
    "Hex",
    "IDS_Binary_Operator",
    "IDSB",
    "IDS_Trinary_Operator",
    "IDST",
    "IDS_Unary_Operator",
    "IDSU",
    "ID_Continue",
    "IDC",
    "ID_Start",
    "IDS",
    "Ideographic",
    "Ideo",
    "Join_Control",
    "Join_C",
    "Logical_Order_Exception",
    "LOE",
    "Lowercase",
    "Lower",
    "Math",
    "Noncharacter_Code_Point",
    "NChar",
    "Pattern_Syntax",
    "Pat_Syn",
    "Pattern_White_Space",
    "Pat_WS",
    "Quotation_Mark",
    "QMark",
    "Radical",
    "Regional_Indicator",
    "RI",
    "Sentence_Terminal",
    "STerm",
    "Soft_Dotted",
    "SD",
    "Terminal_Punctuation",
    "Term",
    "Unified_Ideograph",
    "UIdeo",
    "Uppercase",
    "Upper",
    "Variation_Selector",
    "VS",
    "White_Space",
    "space",
    "XID_Continue",
    "XIDC",
    "XID_Start",
    "XIDS",
];

/// v-flag (unicodeSets) sequence properties (ES2024 §22.2.2.4).
/// Only valid with the `v` flag, not the `u` flag.
const SEQUENCE_PROPERTIES: &[&str] = &[
    "Basic_Emoji",
    "Emoji_Keycap_Sequence",
    "RGI_Emoji",
    "RGI_Emoji_Flag_Sequence",
    "RGI_Emoji_Modifier_Sequence",
    "RGI_Emoji_Tag_Sequence",
    "RGI_Emoji_ZWJ_Sequence",
];

/// `General_Category` values — canonical names and aliases (`PropertyValueAliases.txt` gc section).
const GENERAL_CATEGORY_VALUES: &[&str] = &[
    // Major categories
    "Cased_Letter",
    "LC",
    "Close_Punctuation",
    "Pe",
    "Connector_Punctuation",
    "Pc",
    "Control",
    "Cc",
    "cntrl",
    "Currency_Symbol",
    "Sc",
    "Dash_Punctuation",
    "Pd",
    "Decimal_Number",
    "Nd",
    "digit",
    "Enclosing_Mark",
    "Me",
    "Final_Punctuation",
    "Pf",
    "Format",
    "Cf",
    "Initial_Punctuation",
    "Pi",
    "Letter",
    "L",
    "Letter_Number",
    "Nl",
    "Line_Separator",
    "Zl",
    "Lowercase_Letter",
    "Ll",
    "Mark",
    "M",
    "Combining_Mark",
    "Math_Symbol",
    "Sm",
    "Modifier_Letter",
    "Lm",
    "Modifier_Symbol",
    "Sk",
    "Nonspacing_Mark",
    "Mn",
    "Number",
    "N",
    "Open_Punctuation",
    "Ps",
    "Other",
    "C",
    "Other_Letter",
    "Lo",
    "Other_Number",
    "No",
    "Other_Punctuation",
    "Po",
    "Other_Symbol",
    "So",
    "Paragraph_Separator",
    "Zp",
    "Private_Use",
    "Co",
    "Punctuation",
    "P",
    "punct",
    "Separator",
    "Z",
    "Space_Separator",
    "Zs",
    "Spacing_Mark",
    "Mc",
    "Surrogate",
    "Cs",
    "Symbol",
    "S",
    "Titlecase_Letter",
    "Lt",
    "Unassigned",
    "Cn",
    "Uppercase_Letter",
    "Lu",
];

/// `Script` / `Script_Extensions` values (`PropertyValueAliases.txt` sc section).
/// Unicode 16.0 — 168 script values.
const SCRIPT_VALUES: &[&str] = &[
    "Adlam", "Adlm",
    "Ahom",
    "Anatolian_Hieroglyphs", "Hluw",
    "Arabic", "Arab",
    "Armenian", "Armn",
    "Avestan", "Avst",
    "Balinese", "Bali",
    "Bamum", "Bamu",
    "Bassa_Vah", "Bass",
    "Batak", "Batk",
    "Bengali", "Beng",
    "Bhaiksuki", "Bhks",
    "Bopomofo", "Bopo",
    "Brahmi", "Brah",
    "Braille", "Brai",
    "Buginese", "Bugi",
    "Buhid", "Buhd",
    "Canadian_Aboriginal", "Cans",
    "Carian", "Cari",
    "Caucasian_Albanian", "Aghb",
    "Chakma", "Cakm",
    "Cham",
    "Cherokee", "Cher",
    "Chorasmian", "Chrs",
    "Common", "Zyyy",
    "Coptic", "Copt", "Qaac",
    "Cuneiform", "Xsux",
    "Cypriot", "Cprt",
    "Cypro_Minoan", "Cpmn",
    "Cyrillic", "Cyrl",
    "Deseret", "Dsrt",
    "Devanagari", "Deva",
    "Dives_Akuru", "Diak",
    "Dogra", "Dogr",
    "Duployan", "Dupl",
    "Egyptian_Hieroglyphs", "Egyp",
    "Elbasan", "Elba",
    "Elymaic", "Elym",
    "Ethiopic", "Ethi",
    "Garay", "Gara",
    "Georgian", "Geor",
    "Glagolitic", "Glag",
    "Gothic", "Goth",
    "Grantha", "Gran",
    "Greek", "Grek",
    "Gujarati", "Gujr",
    "Gunjala_Gondi", "Gong",
    "Gurmukhi", "Guru",
    "Gurung_Khema", "Gukh",
    "Han", "Hani",
    "Hangul", "Hang",
    "Hanifi_Rohingya", "Rohg",
    "Hanunoo", "Hano",
    "Hatran", "Hatr",
    "Hebrew", "Hebr",
    "Hiragana", "Hira",
    "Imperial_Aramaic", "Armi",
    "Inherited", "Zinh", "Qaai",
    "Inscriptional_Pahlavi", "Phli",
    "Inscriptional_Parthian", "Prti",
    "Javanese", "Java",
    "Kaithi", "Kthi",
    "Kannada", "Knda",
    "Katakana", "Kana",
    "Kayah_Li", "Kali",
    "Kharoshthi", "Khar",
    "Khitan_Small_Script", "Kits",
    "Khmer", "Khmr",
    "Khojki", "Khoj",
    "Kirat_Rai", "Krai",
    "Lao", "Laoo",
    "Latin", "Latn",
    "Lepcha", "Lepc",
    "Limbu", "Limb",
    "Linear_A", "Lina",
    "Linear_B", "Linb",
    "Lisu",
    "Lycian", "Lyci",
    "Lydian", "Lydi",
    "Mahajani", "Mahj",
    "Makasar", "Maka",
    "Malayalam", "Mlym",
    "Mandaic", "Mand",
    "Manichaean", "Mani",
    "Marchen", "Marc",
    "Masaram_Gondi", "Gonm",
    "Medefaidrin", "Medf",
    "Meetei_Mayek", "Mtei",
    "Mende_Kikakui", "Mend",
    "Meroitic_Cursive", "Merc",
    "Meroitic_Hieroglyphs", "Mero",
    "Miao", "Plrd",
    "Modi",
    "Mongolian", "Mong",
    "Mro", "Mroo",
    "Multani", "Mult",
    "Myanmar", "Mymr",
    "Nabataean", "Nbat",
    "Nandinagari", "Nand",
    "New_Tai_Lue", "Talu",
    "Newa",
    "Nko", "Nkoo",
    "Nushu", "Nshu",
    "Nyiakeng_Puachue_Hmong", "Hmnp",
    "Ogham", "Ogam",
    "Ol_Chiki", "Olck",
    "Ol_Onal", "Onao",
    "Old_Hungarian", "Hung",
    "Old_Italic", "Ital",
    "Old_North_Arabian", "Narb",
    "Old_Permic", "Perm",
    "Old_Persian", "Xpeo",
    "Old_Sogdian", "Sogo",
    "Old_South_Arabian", "Sarb",
    "Old_Turkic", "Orkh",
    "Old_Uyghur", "Ougr",
    "Oriya", "Orya",
    "Osage", "Osge",
    "Osmanya", "Osma",
    "Pahawh_Hmong", "Hmng",
    "Palmyrene", "Palm",
    "Pau_Cin_Hau", "Pauc",
    "Phags_Pa", "Phag",
    "Phoenician", "Phnx",
    "Psalter_Pahlavi", "Phlp",
    "Rejang", "Rjng",
    "Runic", "Runr",
    "Samaritan", "Samr",
    "Saurashtra", "Saur",
    "Sharada", "Shrd",
    "Shavian", "Shaw",
    "Siddham", "Sidd",
    "SignWriting", "Sgnw",
    "Sinhala", "Sinh",
    "Sogdian", "Sogd",
    "Sora_Sompeng", "Sora",
    "Soyombo", "Soyo",
    "Sundanese", "Sund",
    "Sunuwar", "Sunu",
    "Syloti_Nagri", "Sylo",
    "Syriac", "Syrc",
    "Tagalog", "Tglg",
    "Tagbanwa", "Tagb",
    "Tai_Le", "Tale",
    "Tai_Tham", "Lana",
    "Tai_Viet", "Tavt",
    "Takri", "Takr",
    "Tamil", "Taml",
    "Tangsa", "Tnsa",
    "Tangut", "Tang",
    "Telugu", "Telu",
    "Thaana", "Thaa",
    "Thai",
    "Tibetan", "Tibt",
    "Tifinagh", "Tfng",
    "Tirhuta", "Tirh",
    "Todhri", "Todr",
    "Toto",
    "Tulu_Tigalari", "Tutg",
    "Ugaritic", "Ugar",
    "Unknown", "Zzzz",
    "Vai", "Vaii",
    "Vithkuqi", "Vith",
    "Wancho", "Wcho",
    "Warang_Citi", "Wara",
    "Yezidi", "Yezi",
    "Yi", "Yiii",
    "Zanabazar_Square", "Zanb",
];

/// Resolve a property name to its canonical form.
/// Returns `None` if the name is not a recognized non-binary property.
fn resolve_non_binary(name: &str) -> Option<&'static str> {
    for &(canonical, alias) in NON_BINARY_PROPERTIES {
        if name == canonical || name == alias {
            return Some(canonical);
        }
    }
    None
}

/// Check if a name is a recognized binary property.
fn is_binary_property(name: &str) -> bool {
    BINARY_PROPERTIES.contains(&name)
}

/// Check if a name is a recognized `General_Category` value.
fn is_gc_value(name: &str) -> bool {
    GENERAL_CATEGORY_VALUES.contains(&name)
}

/// Check if a name is a recognized `Script` / `Script_Extensions` value.
fn is_script_value(name: &str) -> bool {
    SCRIPT_VALUES.contains(&name)
}

/// Check if a name is a recognized sequence property (v-flag only).
fn is_sequence_property(name: &str) -> bool {
    SEQUENCE_PROPERTIES.contains(&name)
}

/// Public check for sequence property names (used by negation validation).
pub(super) fn is_sequence_property_name(name: &str) -> bool {
    is_sequence_property(name)
}

/// Validate a `\p{...}` or `\P{...}` property name/value pair.
///
/// Returns `Ok(())` if valid, `Err(message)` if invalid.
pub(super) fn validate(
    name: &str,
    value: Option<&str>,
    is_unicode_sets: bool,
) -> Result<(), &'static str> {
    match value {
        None => {
            // \p{X} — X is a binary property, or a lone General_Category value
            if is_binary_property(name) {
                return Ok(());
            }
            if is_gc_value(name) {
                return Ok(());
            }
            if is_unicode_sets && is_sequence_property(name) {
                return Ok(());
            }
            Err("Invalid Unicode property name or value")
        }
        Some(val) => {
            // \p{Name=Value}
            let Some(canonical) = resolve_non_binary(name) else {
                return Err("Invalid Unicode property name");
            };
            match canonical {
                "General_Category" => {
                    if is_gc_value(val) {
                        Ok(())
                    } else {
                        Err("Invalid General_Category value")
                    }
                }
                "Script" | "Script_Extensions" => {
                    if is_script_value(val) {
                        Ok(())
                    } else {
                        Err("Invalid Script value")
                    }
                }
                _ => Err("Invalid Unicode property name"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Binary properties ──

    #[test]
    fn binary_canonical() {
        assert!(validate("ASCII", None, false).is_ok());
        assert!(validate("Alphabetic", None, false).is_ok());
        assert!(validate("Emoji", None, false).is_ok());
        assert!(validate("White_Space", None, false).is_ok());
    }

    #[test]
    fn binary_alias() {
        assert!(validate("Alpha", None, false).is_ok());
        assert!(validate("space", None, false).is_ok());
        assert!(validate("Upper", None, false).is_ok());
        assert!(validate("Hex", None, false).is_ok());
    }

    #[test]
    fn binary_invalid() {
        assert!(validate("Nonexistent", None, false).is_err());
        assert!(validate("ascii", None, false).is_err()); // case-sensitive
        assert!(validate("ALPHABETIC", None, false).is_err());
    }

    // ── Lone General_Category values ──

    #[test]
    fn gc_lone_value() {
        assert!(validate("Letter", None, false).is_ok());
        assert!(validate("L", None, false).is_ok());
        assert!(validate("Nd", None, false).is_ok());
        assert!(validate("Ll", None, false).is_ok());
    }

    // ── Name=Value form ──

    #[test]
    fn gc_name_value() {
        assert!(validate("General_Category", Some("Letter"), false).is_ok());
        assert!(validate("gc", Some("L"), false).is_ok());
        assert!(validate("gc", Some("Nd"), false).is_ok());
    }

    #[test]
    fn gc_invalid_value() {
        assert!(validate("gc", Some("Nonexistent"), false).is_err());
    }

    #[test]
    fn script_name_value() {
        assert!(validate("Script", Some("Latin"), false).is_ok());
        assert!(validate("sc", Some("Latn"), false).is_ok());
        assert!(validate("Script_Extensions", Some("Han"), false).is_ok());
        assert!(validate("scx", Some("Hani"), false).is_ok());
    }

    #[test]
    fn script_invalid_value() {
        assert!(validate("sc", Some("Nonexistent"), false).is_err());
    }

    #[test]
    fn invalid_name_with_value() {
        assert!(validate("Nonexistent", Some("Value"), false).is_err());
        // Binary properties don't take values
        assert!(validate("ASCII", Some("true"), false).is_err());
    }

    // ── Sequence properties (v-flag) ──

    #[test]
    fn sequence_property_v_flag() {
        assert!(validate("Basic_Emoji", None, true).is_ok());
        assert!(validate("RGI_Emoji", None, true).is_ok());
        assert!(validate("RGI_Emoji_ZWJ_Sequence", None, true).is_ok());
    }

    #[test]
    fn sequence_property_without_v_flag() {
        // Sequence properties require the v flag
        assert!(validate("Basic_Emoji", None, false).is_err());
        assert!(validate("RGI_Emoji", None, false).is_err());
    }
}
