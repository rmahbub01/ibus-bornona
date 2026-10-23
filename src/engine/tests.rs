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
    }

    #[test]
    fn hasanta_is_immediate_and_left_standing_kars_become_independent_vowels() {
        assert_eq!(compose("h"), "্");
        assert_eq!(compose("ha"), "্া");
        assert_eq!(compose("hi"), "ই");
        assert_eq!(compose("hI"), "্ী");
        assert_eq!(compose("ha hi hI"), "্া ই ্ী");
        assert_eq!(compose("kha"), "ক্া");
        assert_eq!(compose("khi"), "কই");
        assert_eq!(compose("khI"), "ক্ী");
        assert_eq!(compose("Qh"), "ঙ্");
        assert_eq!(compose("Qhi"), "ঙই");
        assert_eq!(compose("Hh"), "হ্");
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
        assert_eq!(compose("ik"), "কি");
        assert_eq!(compose("Ek"), "কৈ");
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
        let mut processor = TypingProcessor::default();
        type_keys(&mut processor, &layout, "pai");

        assert_eq!(processor.buffer(), "পা");
        assert_eq!(processor.preedit_text(), "পাই");
        assert_eq!(processor.commit(), "পাই");
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
}
