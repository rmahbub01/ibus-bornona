//! Fixed-layout Bengali typing automation.

use super::chars::*;

#[derive(Debug, Clone)]
pub struct TypingConfig {
    pub auto_vowel: bool,
    pub auto_chandra: bool,
    pub old_reph: bool,
    pub old_kar_order: bool,
    pub traditional_kar: bool,
}

impl Default for TypingConfig {
    fn default() -> Self {
        Self {
            auto_vowel: true,
            auto_chandra: true,
            old_reph: true,
            old_kar_order: true,
            traditional_kar: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypingProcessor {
    buffer: String,
    pending_kar: Option<char>,
    config: TypingConfig,
    history: Vec<(String, Option<char>)>,
}

impl Default for TypingProcessor {
    fn default() -> Self {
        Self::new(TypingConfig::default())
    }
}

impl TypingProcessor {
    pub fn new(config: TypingConfig) -> Self {
        Self {
            buffer: String::with_capacity(64),
            pending_kar: None,
            config,
            history: Vec::new(),
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.pending_kar.is_none()
    }

    pub fn ongoing_session(&self) -> bool {
        !self.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.pending_kar = None;
        self.history.clear();
    }

    pub fn commit(&mut self) -> String {
        let mut text = self.buffer.clone();
        if let Some(kar) = self.pending_kar.take() {
            if self.config.auto_vowel {
                text.push(kar.kar_to_independent_vowel().unwrap_or(kar));
            } else {
                text.push(kar);
            }
        }
        self.clear();
        text
    }

    pub fn backspace(&mut self) -> bool {
        let Some((buffer, pending_kar)) = self.history.pop() else {
            return false;
        };
        self.buffer = buffer;
        self.pending_kar = pending_kar;
        true
    }

    /// Process one layout output as a single undoable physical-key action.
    pub fn process_value(&mut self, value: &str) {
        self.history
            .push((self.buffer.clone(), self.pending_kar));
        if value == "\u{09CD}\u{09AF}" {
            self.insert_zo_fola();
            return;
        }
        if value == "\u{09B0}\u{09CD}" && self.config.old_reph {
            self.insert_old_style_reph();
            return;
        }
        for character in value.chars() {
            self.process_character(character);
        }
    }

    fn process_character(&mut self, character: char) {
        let last = self.buffer.chars().last().unwrap_or_default();

        if character.is_kar() {
            if self.config.old_kar_order
                && character.is_left_standing_kar()
                && (self.buffer.is_empty() || last.is_vowel() || MARKS.contains(last))
                && last != B_HASANTA
            {
                self.pending_kar = Some(character);
                return;
            }

            if self.config.old_kar_order
                && last == B_E_KAR
                && matches!(character, B_AA_KAR | B_OU_KAR)
            {
                self.buffer.pop();
                self.buffer
                    .push(if character == B_AA_KAR { B_O_KAR } else { B_OU_KAR });
                return;
            }

            self.flush_pending_kar_before(character);

            if self.config.auto_vowel
                && (self.buffer.is_empty() || last.is_vowel() || MARKS.contains(last))
            {
                if let Some(vowel) = character.kar_to_independent_vowel() {
                    self.buffer.push(vowel);
                    return;
                }
            }

            if self.config.auto_chandra && last == B_CHANDRA {
                self.buffer.pop();
                self.buffer.push(character);
                self.buffer.push(B_CHANDRA);
                return;
            }

            if self.config.traditional_kar && last.is_pure_consonant() {
                if character.is_ligature_making_kar() {
                    self.buffer.push(ZWNJ);
                }
            }
            self.buffer.push(character);
            return;
        }

        if character == B_HASANTA && self.config.old_kar_order && last.is_left_standing_kar() {
            if let Some(kar) = self.buffer.pop() {
                self.buffer.push(B_HASANTA);
                self.buffer.push(kar);
                return;
            }
        }

        if character == B_LENGTH_MARK && last == B_HASANTA {
            self.buffer.pop();
            self.buffer.push(B_OU);
            return;
        }

        if self.config.old_kar_order && last == B_E_KAR && character == B_LENGTH_MARK {
            self.buffer.pop();
            self.buffer.push(B_OU_KAR);
            return;
        }

        if self.flush_pending_kar_before(character) {
            return;
        }
        self.buffer.push(character);
    }

    fn flush_pending_kar_before(&mut self, next: char) -> bool {
        let Some(kar) = self.pending_kar else {
            return false;
        };
        if next == B_HASANTA {
            self.buffer.push(next);
            return true;
        }
        if next.is_pure_consonant() {
            self.buffer.push(next);
            self.buffer.push(kar);
            self.pending_kar = None;
            return true;
        }
        if self.config.auto_vowel
            && (self.buffer.is_empty()
                || self
                    .buffer
                    .chars()
                    .last()
                    .is_some_and(|last| last.is_vowel() || MARKS.contains(last)))
        {
            self.buffer.push(kar.kar_to_independent_vowel().unwrap_or(kar));
            self.pending_kar = None;
            return false;
        }
        self.buffer.push(kar);
        self.pending_kar = None;
        false
    }

    fn insert_zo_fola(&mut self) {
        let last = self.buffer.chars().last().unwrap_or_default();
        let before_last = self.buffer.chars().rev().nth(1).unwrap_or_default();
        if last == B_R && before_last != B_HASANTA {
            self.buffer.push(ZWJ);
        }
        if self.config.old_kar_order && last.is_left_standing_kar() {
            self.buffer.pop();
            self.buffer.push(B_HASANTA);
            self.buffer.push(B_Z);
            self.buffer.push(last);
            return;
        }
        self.buffer.push(B_HASANTA);
        self.buffer.push(B_Z);
    }

    fn is_reph_moveable(&self) -> bool {
        let mut chars = self.buffer.chars().rev();
        let Some(mut rightmost) = chars.next() else {
            return false;
        };
        if rightmost == B_CHANDRA {
            rightmost = chars.next().unwrap_or_default();
        }
        let before_rightmost = chars.next().unwrap_or_default();
        rightmost.is_pure_consonant()
            || (rightmost.is_vowel() && before_rightmost.is_pure_consonant())
    }

    fn insert_old_style_reph(&mut self) {
        if !self.is_reph_moveable() {
            self.buffer.push(B_R);
            self.buffer.push(B_HASANTA);
            return;
        }

        let mut consonant = false;
        let mut vowel = false;
        let mut hasanta = false;
        let mut chandra = false;
        let mut count = 0;

        for (index, character) in self.buffer.chars().rev().enumerate() {
            if character.is_pure_consonant() {
                if consonant && !hasanta {
                    break;
                }
                consonant = true;
                hasanta = false;
                count += 1;
            } else if character == B_HASANTA {
                hasanta = true;
                count += 1;
            } else if character.is_vowel() {
                if vowel || (index != 0 && !chandra) {
                    break;
                }
                vowel = true;
                count += 1;
            } else if character == B_CHANDRA && index == 0 {
                chandra = true;
                count += 1;
            } else {
                break;
            }
        }

        if count == 0 {
            self.buffer.push(B_R);
            self.buffer.push(B_HASANTA);
            return;
        }

        let split = self
            .buffer
            .char_indices()
            .rev()
            .nth(count - 1)
            .map(|(index, _)| index)
            .unwrap_or(0);
        let trailing = self.buffer.split_off(split);
        self.buffer.push(B_R);
        self.buffer.push(B_HASANTA);
        self.buffer.push_str(&trailing);
    }
}

/// Compose Bornona key characters using the same fixed-layout engine as IBus.
pub fn compose(keys: &str) -> String {
    let layout = super::layout::BornonaLayout::new();
    let mut processor = TypingProcessor::default();
    for key in keys.chars() {
        if key == ' ' {
            processor.process_value(" ");
        } else if let Some(value) = layout.map_char(key) {
            processor.process_value(value);
        } else {
            processor.process_value(&key.to_string());
        }
    }
    processor.commit()
}
