#[cfg(test)]
mod tests {
    use crate::engine::chars::ZWNJ;
    use crate::engine::layout::BornonaLayout;
    use crate::engine::processor::{compose, TypingConfig, TypingProcessor};

    fn type_keys(processor: &mut TypingProcessor, layout: &BornonaLayout, input: &str) {
        for key in input.chars() {
            if key == ' ' {
                processor.process_value(" ");
            } else if let Some(value) = layout.map_char(key) {
                processor.process_value(value);
            } else {
                processor.process_value(&key.to_string());
            }
        }
    }

    #[test]
    fn bornona_keymap_is_available_to_the_engine() {
        let layout = BornonaLayout::new();
        assert_eq!(layout.map_char('a'), Some("া"));
        assert_eq!(layout.map_char('k'), Some("ক"));
        assert_eq!(layout.map_char('K'), Some("খ"));
        assert_eq!(layout.map_char('A'), Some("অ"));
        assert_eq!(layout.map_char('F'), Some("র্"));
        assert_eq!(layout.map_char('P'), Some("্র"));
        assert_eq!(layout.map_char('V'), Some("্য"));
        assert_eq!(layout.map_char('B'), Some("\u{09AF}\u{09BC}"));
        assert_eq!(layout.map_numpad(0xffb1), Some("১"));
    }

    #[test]
    fn automatic_vowels_and_consonant_kars() {
        assert_eq!(compose("a"), "আ");
        assert_eq!(compose("i"), "ই");
        assert_eq!(compose("u"), "উ");
        assert_eq!(compose("o"), "ও");
        assert_eq!(compose("A"), "অ");
        assert_eq!(compose("ek"), "এক");
        assert_eq!(compose("eK"), "এখ");
        assert_eq!(compose("ka"), "কা");
        assert_eq!(compose("ki"), "কি");
        assert_eq!(compose("ke"), "কে");
        assert_eq!(compose("Ke"), "খে");
        assert_eq!(compose("kE"), "কৈ");
        assert_eq!(compose("leauT"), "লেআউট");
        assert_eq!(compose("beaini"), "বেআইনি");
        assert_eq!(compose("pai"), "পাই");
        assert_eq!(compose("paik"), "পাইক");
    }

    #[test]
    fn hasanta_is_immediate_and_kars_become_independent_vowels() {
        assert_eq!(compose("h"), "্");
        assert_eq!(compose("ha"), "আ");
        assert_eq!(compose("hi"), "ই");
        assert_eq!(compose("hI"), "ঈ");
        assert_eq!(compose("ho"), "ও");
        assert_eq!(compose("ha hi hI"), "আ ই ঈ");
        assert_eq!(compose("kha"), "কআ");
        assert_eq!(compose("khi"), "কই");
        assert_eq!(compose("khI"), "কঈ");
        assert_eq!(compose("kho"), "কও");
        assert_eq!(compose("Qh"), "ঙ্");
        assert_eq!(compose("Qhi"), "ঙই");
        assert_eq!(compose("Hh"), "হ্");
        assert_eq!(compose("HB"), "হ\u{09AF}\u{09BC}");
        assert_eq!(compose("HhoBar"), "হ\u{0993}\u{09AF}\u{09BC}\u{09BE}\u{09B0}");
        assert_eq!(compose(&format!("h`")), format!("্{}", ZWNJ));
    }

    #[test]
    fn fixed_layout_conjuncts_are_composed_in_the_preedit_buffer() {
        assert_eq!(compose("khx"), "ক্ষ");
        assert_eq!(compose("jhMan"), "জ্ঞান");
        assert_eq!(compose("Qhk"), "ঙ্ক");
        assert_eq!(compose("nhd"), "ন্দ");
        assert_eq!(compose("xhT"), "ষ্ট");
    }

    #[test]
    fn automatic_chandra_position_and_vowel_order() {
        assert_eq!(compose("k@a"), "কাঁ");
        assert_eq!(compose("ig"), "ইগ");
        assert_eq!(compose("gi"), "গি");
        assert_eq!(compose("ik"), "ইক");
        assert_eq!(compose("ki"), "কি");
        assert_eq!(compose("Ek"), "ঐক");
        assert_eq!(compose("kE"), "কৈ");

        let old_config = TypingConfig {
            old_kar_order: true,
            ..TypingConfig::default()
        };
        assert_eq!(
            crate::engine::processor::compose_with_config("ik", old_config.clone()),
            "কি"
        );
        assert_eq!(
            crate::engine::processor::compose_with_config("Ek", old_config),
            "কৈ"
        );
    }

    #[test]
    fn reph_and_phala_automation() {
        assert_eq!(compose("bNFna"), "বর্ণনা");
        assert_eq!(compose("djFi"), "দর্জি");
        assert_eq!(compose("pPaNe"), "প্রাণে");
        assert_eq!(compose("rV"), format!("র{}\u{09CD}\u{09AF}", '\u{200D}'));
        assert_eq!(compose("kV"), "ক্য");
    }

    #[test]
    fn backspace_restores_the_exact_preedit_state() {
        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::default();
        type_keys(&mut processor, &layout, "ka");
        assert_eq!(processor.buffer(), "কা");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "ক");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "");
        assert!(!processor.backspace());
    }

    #[test]
    fn backspace_undoes_hasanta_and_pending_left_kar() {
        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::default();
        type_keys(&mut processor, &layout, "kh");
        assert_eq!(processor.buffer(), "ক্");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "ক");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "");

        type_keys(&mut processor, &layout, "i");
        assert!(processor.ongoing_session());
        assert!(processor.backspace());
        assert!(processor.is_empty());
    }

    #[test]
    fn commit_flushes_pending_left_standing_vowel_and_clears_state() {
        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::new(TypingConfig::default());
        type_keys(&mut processor, &layout, "i");
        assert_eq!(processor.preedit_text(), "ই");
        assert_eq!(processor.commit(), "ই");
        assert!(processor.is_empty());
    }

    #[test]
    fn pending_vowel_is_visible_in_preedit_before_commit() {
        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::new(TypingConfig {
            old_kar_order: true,
            ..TypingConfig::default()
        });
        type_keys(&mut processor, &layout, " i");

        assert_eq!(processor.buffer(), " ");
        assert_eq!(processor.preedit_text(), " ই");
        assert_eq!(processor.commit(), " ই");
    }

    #[test]
    fn clear_discards_composition_and_history() {
        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::default();
        type_keys(&mut processor, &layout, "ka");
        processor.clear();
        assert!(processor.is_empty());
        assert!(!processor.backspace());
    }

    #[test]
    fn suggestion_setting_never_changes_composed_text() {
        for keys in ["bFNna", "ik", "k@a", "pPaNe", "rV"] {
            let suggestions_off = TypingConfig {
                word_suggestion: false,
                ..TypingConfig::default()
            };
            let suggestions_on = TypingConfig {
                word_suggestion: true,
                ..TypingConfig::default()
            };
            assert_eq!(
                crate::engine::processor::compose_with_config(keys, suggestions_off),
                crate::engine::processor::compose_with_config(keys, suggestions_on),
                "suggestion setting changed the composition for {keys}"
            );
        }
    }

    #[test]
    fn hb_composes_hoya_with_canonical_nukta() {
        use crate::engine::SuggestionEngine;

        let composed = compose("HB");
        assert_eq!(composed, "হ\u{09AF}\u{09BC}");
        assert!(!composed.contains('\u{09DF}'));

        let suggestions = SuggestionEngine::suggest(&composed, 5);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0], "হ\u{09AF}\u{09BC}");

        let layout = BornonaLayout::new();
        let mut processor = TypingProcessor::default();
        type_keys(&mut processor, &layout, "HB");
        assert_eq!(processor.buffer(), "হ\u{09AF}\u{09BC}");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "হ");
        assert!(processor.backspace());
        assert_eq!(processor.buffer(), "");

        let old_config = TypingConfig {
            old_kar_order: true,
            ..TypingConfig::default()
        };
        assert_eq!(
            crate::engine::processor::compose_with_config("iB", old_config.clone()),
            "\u{09AF}\u{09BC}\u{09BF}"
        );
        assert_eq!(
            crate::engine::processor::compose_with_config("EB", old_config),
            "\u{09AF}\u{09BC}\u{09C8}"
        );
    }
}
