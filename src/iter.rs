use crate::{Section, SectionData};

// FIXME: Consider whether we can avoid this, or if necessary, generate with
// macros.

pub struct FollowingSiblings<'a> {
    pub(crate) following_siblings: indextree::FollowingSiblings<'a, SectionData>,
}

impl<'a> Iterator for FollowingSiblings<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.following_siblings.next().map(|c| Section { id: c })
    }
}

pub struct PrecedingSiblings<'a> {
    pub(crate) preceding_siblings: indextree::PrecedingSiblings<'a, SectionData>,
}

impl<'a> Iterator for PrecedingSiblings<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.preceding_siblings.next().map(|c| Section { id: c })
    }
}

pub struct ReverseChildren<'a> {
    pub(crate) reverse_children: indextree::ReverseChildren<'a, SectionData>,
}

impl<'a> Iterator for ReverseChildren<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.reverse_children.next().map(|c| Section { id: c })
    }
}

pub struct Descendants<'a> {
    pub(crate) descendants: indextree::Descendants<'a, SectionData>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.descendants.next().map(|c| Section { id: c })
    }
}

pub struct Ancestors<'a> {
    pub(crate) ancestors: indextree::Ancestors<'a, SectionData>,
}

impl<'a> Iterator for Ancestors<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.ancestors.next().map(|c| Section { id: c })
    }
}

pub struct Children<'a> {
    pub(crate) children: indextree::Children<'a, SectionData>,
}

impl<'a> Iterator for Children<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        self.children.next().map(|c| Section { id: c })
    }
}

// Splitting on a value, unlike splitting on whitespace, will yield a single
// empty string. Strange, but consistent with Python.
pub struct Tags<'a> {
    pub(crate) split: Option<std::str::Split<'a, char>>,
}

impl<'a> Iterator for Tags<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        loop {
            let tag = self.split.as_mut()?.next()?;
            if !tag.is_empty() {
                break Some(tag);
            }
        }
    }
}
