//! Bengali Unicode constants and character classification traits
//! Based on Unicode Bengali block U+0980 - U+09FF and OpenBangla keyboard conventions.

#![allow(dead_code)]

/* Signs */
pub const B_SIGN_ANJI: char = '\u{0980}';
pub const B_CHANDRA: char = '\u{0981}';
pub const B_ANUSHAR: char = '\u{0982}';
pub const B_BISHARGA: char = '\u{0983}';

/* Independent Vowels */
pub const B_A: char = '\u{0985}';
pub const B_AA: char = '\u{0986}';
pub const B_I: char = '\u{0987}';
pub const B_II: char = '\u{0988}';
pub const B_U: char = '\u{0989}';
pub const B_UU: char = '\u{098A}';
pub const B_RRI: char = '\u{098B}';
pub const B_VOCALIC_L: char = '\u{098C}';
pub const B_E: char = '\u{098F}';
pub const B_OI: char = '\u{0990}';
pub const B_O: char = '\u{0993}';
pub const B_OU: char = '\u{0994}';

/* Consonants */
pub const B_K: char = '\u{0995}';
pub const B_KH: char = '\u{0996}';
pub const B_G: char = '\u{0997}';
pub const B_GH: char = '\u{0998}';
pub const B_NGA: char = '\u{0999}';
pub const B_C: char = '\u{099A}';
pub const B_CH: char = '\u{099B}';
pub const B_J: char = '\u{099C}';
pub const B_JH: char = '\u{099D}';
pub const B_NYA: char = '\u{099E}';
pub const B_TT: char = '\u{099F}';
pub const B_TTH: char = '\u{09A0}';
pub const B_DD: char = '\u{09A1}';
pub const B_DDH: char = '\u{09A2}';
pub const B_NN: char = '\u{09A3}';
pub const B_T: char = '\u{09A4}';
pub const B_TH: char = '\u{09A5}';
pub const B_D: char = '\u{09A6}';
pub const B_DH: char = '\u{09A7}';
pub const B_N: char = '\u{09A8}';
pub const B_P: char = '\u{09AA}';
pub const B_PH: char = '\u{09AB}';
pub const B_B: char = '\u{09AC}';
pub const B_BH: char = '\u{09AD}';
pub const B_M: char = '\u{09AE}';
pub const B_Z: char = '\u{09AF}';
pub const B_R: char = '\u{09B0}';
pub const B_L: char = '\u{09B2}';
pub const B_SH: char = '\u{09B6}';
pub const B_SS: char = '\u{09B7}';
pub const B_S: char = '\u{09B8}';
pub const B_H: char = '\u{09B9}';

pub const B_SIGN_NUKTA: char = '\u{09BC}';
pub const B_SIGN_AVAGRAHA: char = '\u{09BD}';

/* Dependent Vowel Signs (Kars) */
pub const B_AA_KAR: char = '\u{09BE}';
pub const B_I_KAR: char = '\u{09BF}';
pub const B_II_KAR: char = '\u{09C0}';
pub const B_U_KAR: char = '\u{09C1}';
pub const B_UU_KAR: char = '\u{09C2}';
pub const B_RRI_KAR: char = '\u{09C3}';
pub const B_VOCALIC_RR: char = '\u{09C4}';
pub const B_E_KAR: char = '\u{09C7}';
pub const B_OI_KAR: char = '\u{09C8}';
pub const B_O_KAR: char = '\u{09CB}';
pub const B_OU_KAR: char = '\u{09CC}';

/* Hasanta (Virama) */
pub const B_HASANTA: char = '\u{09CD}';

/* Khanda Ta */
pub const B_KHANDATTA: char = '\u{09CE}';

/* Additional signs & letters */
pub const B_LENGTH_MARK: char = '\u{09D7}'; // Au length mark
pub const B_RR: char = '\u{09DC}'; // Rra
pub const B_RH: char = '\u{09DD}'; // Rha
pub const B_Y: char = '\u{09DF}'; // Yya

/* Punctuation */
pub const B_DARI: char = '\u{0964}';
pub const B_DDARI: char = '\u{0965}';

/* Digits */
pub const B_0: char = '\u{09E6}';
pub const B_1: char = '\u{09E7}';
pub const B_2: char = '\u{09E8}';
pub const B_3: char = '\u{09E9}';
pub const B_4: char = '\u{09EA}';
pub const B_5: char = '\u{09EB}';
pub const B_6: char = '\u{09EC}';
pub const B_7: char = '\u{09ED}';
pub const B_8: char = '\u{09EE}';
pub const B_9: char = '\u{09EF}';

/* Currency */
pub const B_CRTAKA: char = '\u{09F3}';

/* Assamese / East Indic */
pub const B_RM: char = '\u{09F0}'; // Assamese Ra (ৰ)
pub const B_WA: char = '\u{09F1}'; // Assamese Wa (ৱ)

/* Zero Width Joiner / Non-Joiner */
pub const ZWJ: char = '\u{200D}';
pub const ZWNJ: char = '\u{200C}';

pub const MARKS: &str = "`~!@#$%^+*-_=+\\|\"/;:,./?><()[]{} \t\n\r";

pub trait BengaliCharExt {
    fn is_kar(&self) -> bool;
    fn is_vowel(&self) -> bool;
    fn is_pure_consonant(&self) -> bool;
    fn is_left_standing_kar(&self) -> bool;
    fn is_ligature_making_kar(&self) -> bool;
    fn kar_to_independent_vowel(&self) -> Option<char>;
}

impl BengaliCharExt for char {
    fn is_kar(&self) -> bool {
        matches!(
            *self,
            B_AA_KAR
                | B_I_KAR
                | B_II_KAR
                | B_U_KAR
                | B_UU_KAR
                | B_RRI_KAR
                | B_VOCALIC_RR
                | B_E_KAR
                | B_OI_KAR
                | B_O_KAR
                | B_OU_KAR
        )
    }

    fn is_vowel(&self) -> bool {
        matches!(
            *self,
            B_A | B_AA
                | B_I
                | B_II
                | B_U
                | B_UU
                | B_RRI
                | B_VOCALIC_L
                | B_E
                | B_OI
                | B_O
                | B_OU
                | B_AA_KAR
                | B_I_KAR
                | B_II_KAR
                | B_U_KAR
                | B_UU_KAR
                | B_RRI_KAR
                | B_E_KAR
                | B_OI_KAR
                | B_O_KAR
                | B_OU_KAR
        )
    }

    fn is_pure_consonant(&self) -> bool {
        matches!(
            *self,
            B_K | B_KH
                | B_G
                | B_GH
                | B_NGA
                | B_C
                | B_CH
                | B_J
                | B_JH
                | B_NYA
                | B_TT
                | B_TTH
                | B_DD
                | B_DDH
                | B_NN
                | B_T
                | B_TH
                | B_D
                | B_DH
                | B_N
                | B_P
                | B_PH
                | B_B
                | B_BH
                | B_M
                | B_Z
                | B_R
                | B_L
                | B_SH
                | B_SS
                | B_S
                | B_H
                | B_KHANDATTA
                | B_RR
                | B_RH
                | B_Y
                | B_RM
                | B_WA
        )
    }

    fn is_left_standing_kar(&self) -> bool {
        matches!(*self, B_I_KAR | B_E_KAR | B_OI_KAR)
    }

    fn is_ligature_making_kar(&self) -> bool {
        matches!(*self, B_U_KAR | B_UU_KAR | B_RRI_KAR)
    }

    fn kar_to_independent_vowel(&self) -> Option<char> {
        match *self {
            B_AA_KAR => Some(B_AA),
            B_I_KAR => Some(B_I),
            B_II_KAR => Some(B_II),
            B_U_KAR => Some(B_U),
            B_UU_KAR => Some(B_UU),
            B_RRI_KAR => Some(B_RRI),
            B_E_KAR => Some(B_E),
            B_OI_KAR => Some(B_OI),
            B_O_KAR => Some(B_O),
            B_OU_KAR => Some(B_OU),
            _ => None,
        }
    }
}
