#[derive(Debug)]
pub struct LookupModel {
    candidates: Vec<String>,
    cursor: usize,
}

impl Default for LookupModel {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            cursor: 0,
        }
    }
}

impl LookupModel {
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    pub fn candidate(&self, index: usize) -> Option<&str> {
        self.candidates.get(index).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn set_candidates(&mut self, candidates: Vec<String>) {
        self.candidates = candidates;
        self.cursor = 0;
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.cursor = 0;
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        if !self.candidates.is_empty() {
            self.cursor = cursor.min(self.candidates.len() - 1);
        }
    }

    pub fn move_cursor(&mut self, step: isize) -> usize {
        if self.candidates.is_empty() {
            self.cursor = 0;
        } else {
            self.cursor = (self.cursor as isize + step)
                .rem_euclid(self.candidates.len() as isize) as usize;
        }
        self.cursor
    }

    pub fn select_number(&self, number: usize) -> Option<usize> {
        if !(1..=9).contains(&number) {
            return None;
        }
        let index = number - 1;
        (index < self.candidates.len()).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::LookupModel;

    fn model() -> LookupModel {
        let mut model = LookupModel::default();
        model.set_candidates((0..20).map(|index| index.to_string()).collect());
        model
    }

    #[test]
    fn cursor_navigation_wraps_and_stays_in_range() {
        let mut model = model();
        assert_eq!(model.move_cursor(-1), 19);
        assert_eq!(model.move_cursor(1), 0);
        model.set_cursor(100);
        assert_eq!(model.cursor(), 19);
    }

    #[test]
    fn number_selection_is_limited_to_the_visible_nine_candidates() {
        let model = model();
        assert_eq!(model.select_number(1), Some(0));
        assert_eq!(model.select_number(9), Some(8));
        assert_eq!(model.select_number(10), None);
    }
}
