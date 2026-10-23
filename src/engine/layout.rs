//! Bornona Layout key mapper.
//! Implements mapping from ASCII key characters / Keypad to Bengali Unicode strings
//! identical to OpenBangla's Bornona.json.

use std::collections::HashMap;

pub const SINGLE_KEY: &[(char, &str)] = &[
    // 1. Independent Vowel A
    ('A', "অ"),
    // 2. Vowel modifiers / kars
    ('a', "া"),
    ('i', "ি"),
    ('I', "ী"),
    ('u', "ু"),
    ('U', "ূ"),
    ('R', "ৃ"),
    ('e', "ে"),
    ('E', "ৈ"),
    ('o', "ো"),
    ('O', "ৌ"),
    // 3. Consonants
    ('k', "ক"),
    ('K', "খ"),
    ('g', "গ"),
    ('G', "ঘ"),
    ('Q', "ঙ"),
    ('c', "চ"),
    ('C', "ছ"),
    ('j', "জ"),
    ('J', "ঝ"),
    ('M', "ঞ"),
    ('T', "ট"),
    ('W', "ঠ"),
    ('D', "ড"),
    ('w', "ঢ"),
    ('N', "ণ"),
    ('t', "ত"),
    ('Y', "থ"),
    ('d', "দ"),
    ('y', "ধ"),
    ('n', "ন"),
    ('p', "প"),
    ('f', "ফ"),
    ('b', "ব"),
    ('v', "ভ"),
    ('m', "ম"),
    ('z', "য"),
    ('r', "র"),
    ('l', "ল"),
    ('S', "শ"),
    ('x', "ষ"),
    ('s', "স"),
    ('H', "হ"),
    ('Z', "ড়"),
    ('X', "ঢ়"),
    ('B', "য়"),
    ('\\', "ৱ"),
    ('|', "ৰ"),
    // 4. Special signs & phalas
    ('h', "্"), // Hasanta (virama)
    ('q', "ং"), // Anushar
    ('^', "ঃ"), // Bisharga
    ('@', "ঁ"), // Chandrabindu
    ('&', "ৎ"), // Khanda Ta
    ('$', "৳"), // Bangladeshi Taka
    ('V', "্য"), // Zo-fola
    ('P', "্র"), // Ro-fola
    ('F', "র্"), // Reph
    ('L', "।"), // Bengali Dari
    ('`', "\u{200C}"), // ZWNJ
    // 5. Bengali Digits
    ('0', "০"),
    ('1', "১"),
    ('2', "২"),
    ('3', "৩"),
    ('4', "৪"),
    ('5', "৫"),
    ('6', "৬"),
    ('7', "৭"),
    ('8', "৮"),
    ('9', "৯"),
];

/// Return the single glyph string for a key.
pub fn single(key: char) -> Option<&'static str> {
    SINGLE_KEY.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Check if key is a Bengali consonant.
pub fn is_consonant(key: char) -> bool {
    matches!(
        key,
        'k' | 'K' | 'g' | 'G' | 'Q' | 'c' | 'C' | 'j' | 'J' | 'M' | 'T' | 'W'
            | 'D' | 'w' | 'N' | 't' | 'Y' | 'd' | 'y' | 'n' | 'p' | 'f' | 'b'
            | 'v' | 'm' | 'z' | 'r' | 'l' | 'S' | 'x' | 's' | 'H' | 'Z' | 'X'
            | 'B' | '\\' | '|'
    )
}

/// Map a vowel key to its independent vowel form.
pub fn independent_vowel(key: char) -> Option<&'static str> {
    match key {
        'a' => Some("আ"),
        'i' => Some("ই"),
        'I' => Some("ঈ"),
        'u' => Some("উ"),
        'U' => Some("ঊ"),
        'R' => Some("ঋ"),
        'e' => Some("এ"),
        'E' => Some("ঐ"),
        'o' => Some("ও"),
        'O' => Some("ঔ"),
        _ => None,
    }
}

/// Glyph for key, defaulting to the character itself as string.
pub fn glyph(key: char) -> String {
    single(key)
        .map(str::to_string)
        .unwrap_or_else(|| key.to_string())
}

#[derive(Debug, Clone)]
pub struct BornonaLayout {
    char_map: HashMap<char, &'static str>,
    numpad_map: HashMap<u32, &'static str>,
}

impl Default for BornonaLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl BornonaLayout {
    pub fn new() -> Self {
        let mut char_map = HashMap::with_capacity(128);
        for &(k, v) in SINGLE_KEY {
            char_map.insert(k, v);
        }

        let mut numpad_map = HashMap::with_capacity(16);
        numpad_map.insert(0xffb0, "০");
        numpad_map.insert(0xffb1, "১");
        numpad_map.insert(0xffb2, "২");
        numpad_map.insert(0xffb3, "৩");
        numpad_map.insert(0xffb4, "৪");
        numpad_map.insert(0xffb5, "৫");
        numpad_map.insert(0xffb6, "৬");
        numpad_map.insert(0xffb7, "৭");
        numpad_map.insert(0xffb8, "৮");
        numpad_map.insert(0xffb9, "৯");
        numpad_map.insert(0xffab, "+");
        numpad_map.insert(0xffad, "-");
        numpad_map.insert(0xffaa, "*");
        numpad_map.insert(0xffaf, "/");
        numpad_map.insert(0xffae, ".");

        Self {
            char_map,
            numpad_map,
        }
    }

    pub fn map_char(&self, ch: char) -> Option<&'static str> {
        self.char_map.get(&ch).copied()
    }

    pub fn map_numpad(&self, keyval: u32) -> Option<&'static str> {
        self.numpad_map.get(&keyval).copied()
    }
}
