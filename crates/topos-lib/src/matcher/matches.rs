use crate::{matcher::instance::BibleMatch, segments::segments::BookSegments};

#[derive(Clone, Debug, Default)]
pub struct ComplexFilter {
    inside_of: Vec<BookSegments>,
    outside_of: Vec<BookSegments>,
}

impl ComplexFilter {
    pub fn new(inside_of: Vec<BookSegments>, outside_of: Vec<BookSegments>) -> Self {
        Self {
            inside_of,
            outside_of,
        }
    }

    pub fn inside(&mut self, psg: BookSegments) {
        self.inside_of.push(psg);
    }

    pub fn outside(&mut self, psg: BookSegments) {
        self.outside_of.push(psg);
    }

    pub fn keep(&self, psg: &BookSegments) -> bool {
        let is_inside = self.inside_of.is_empty()
            || self
                .inside_of
                .iter()
                .any(|inside| inside.overlaps_with(psg));

        if is_inside == false {
            return false;
        }

        let is_outside = self
            .outside_of
            .iter()
            .all(|outside| !outside.overlaps_with(psg));

        is_outside
    }
}

pub struct FilteredBibleMatches<'a> {
    filter: &'a ComplexFilter,
    matches: Vec<BibleMatch>,
}

impl<'a> FilteredBibleMatches<'a> {
    pub fn new(filter: &'a ComplexFilter) -> Self {
        Self {
            filter,
            matches: vec![],
        }
    }

    pub fn try_add(&mut self, m: BibleMatch) {
        if self.filter.keep(&m.psg) {
            self.matches.push(m);
        }
    }

    pub fn matches(self) -> Vec<BibleMatch> {
        self.matches
    }
}
