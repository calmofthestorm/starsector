use ropey::{Rope, RopeSlice};

use crate::{parser::structure::parse_document, Document, RopeExt, Section};

#[derive(Default)]
pub struct Arena {
    pub(crate) arena: indextree::Arena<SectionData>,
}

impl Arena {
    pub fn parse_reader<T: std::io::Read>(&mut self, reader: &mut T) -> std::io::Result<Document> {
        let text = Rope::from_reader(reader)?;
        Ok(parse_document(self, &text.slice(..)))
    }

    pub fn parse_string(&mut self, input: String) -> Document {
        self.parse_rope(Rope::from(input))
    }

    pub fn parse_str(&mut self, input: &str) -> Document {
        self.parse_rope(Rope::from(input))
    }

    pub fn parse_rope(&mut self, input: Rope) -> Document {
        parse_document(self, &input.slice(..))
    }

    pub fn parse(&mut self, input: &RopeSlice) -> Document {
        parse_document(self, input)
    }

    /// Parse the provided input as a single section -- a headline if possible,
    /// if not, a root document. The text must have at most one root headline --
    /// that is, "* A\n** B" is OK, but "* A\n* B" is not.
    ///
    /// Unlike the other parsing sections that return a Document, we make no
    /// particular effort to ensure that (parse * write) is idempotent -- i.e.,
    /// we may not write out exactly what you put in in terms of newline,
    /// whitespace, etc.
    pub fn new_section(&mut self, input: Rope) -> Option<Section> {
        let root = self.parse_rope(input);
        let mut children = root.root.id.children(&self.arena);

        if let Some(child) = children.next() {
            if children.next().is_some() {
                // If we parsed more than one child, there isn't just one section.
                None
            } else if self.arena[root.root.id].get().text.is_empty() && root.empty_root_section {
                // For convenience, an empty root with a single child will
                // resolve to that child.
                Some(Section { id: child })
            } else {
                // A non-empty root means we return it.
                Some(root.root)
            }
        } else {
            // Root is the only section.
            Some(root.root)
        }
    }

    /// Creates a new section (Org headline) with the same content as that
    /// specified. The new section will not have any children.
    pub fn clone_section(&mut self, section: Section) -> Section {
        let data = self.arena[section.id].get().clone();
        let id = self.arena.new_node(data);
        Section { id }
    }

    pub(crate) fn set_level(&mut self, new_child: Section, level: u16) {
        let data = self.arena[new_child.id].get();
        if data.level > level {
            self.section_max_level(new_child, level)
        } else if data.level < level {
            self.section_min_level(new_child, level)
        }
    }

    pub(crate) fn section_max_level(&mut self, new_child: Section, max_level: u16) {
        let data = self.arena[new_child.id].get();
        let level = data.level;
        if level <= max_level {
            return;
        }

        let mut text = data.text.clone();
        let mut change = (level - max_level) as usize;
        if level > 0 && max_level == 0 {
            change += 1;
        }
        text.remove(..change);

        *self.arena[new_child.id].get_mut() = SectionData {
            level: max_level,
            text,
        };
    }

    pub(crate) fn section_min_level(&mut self, new_child: Section, min_level: u16) {
        let data = self.arena[new_child.id].get();
        let level = data.level;
        if level >= min_level {
            return;
        }

        let text_len = data.text.len_bytes();
        let change = (min_level - level) as usize;

        let mut text = String::default();
        text.reserve(change + text_len + if level == 0 { 1 } else { 0 });
        text.extend(std::iter::repeat('*').take(change));
        if level == 0 {
            text.push(' ');
        }
        let mut text = Rope::from(text);
        text.append(data.text.clone());

        *self.arena[new_child.id].get_mut() = SectionData {
            level: min_level,
            text,
        };
    }
}

// This also includes the preceding headline if applicable, which differs from
// Org spec terminology.
#[derive(Debug, Clone, Default)]
pub(crate) struct SectionData {
    pub(crate) level: u16,
    pub(crate) text: Rope,
}
