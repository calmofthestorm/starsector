use std::borrow::Cow;

use indextree::NodeId;
use log::trace;
use ropey::Rope;

use crate::{emit::section_tree_to_rope, errors::*, iter::*, *};

#[cfg(feature = "headline-parser")]
use crate::headline::*;

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub struct Section {
    pub(crate) id: NodeId,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub root: Section,

    // This is only necessary to handle the annoying edge case where the
    // document section is just "\n". Instead of turning text into an option, we
    // use an empty string to represent both "\n" and "", since most uses will
    // want uniform treatment of the two. When writing the file out, this will
    // be special cased.
    pub empty_root_section: bool,

    // When this Section is written out, the terminal newline will be omitted if
    // this is false and it is the root section.
    pub terminal_newline: bool,
}

impl Document {
    pub fn to_rope(&self, arena: &Arena) -> Rope {
        section_tree_to_rope(
            self.root.id,
            arena,
            self.terminal_newline,
            self.empty_root_section,
        )
    }

    pub fn at(&self, arena: &Arena, mut pos: usize) -> Option<(Section, usize)> {
        let ct = self.to_rope(arena);
        let k = ct.len_chars();
        trace!("at: {} in buffer of length {}.", pos, k);

        if pos >= k {
            trace!("at: beyond end of buffer");
            return None;
        }

        if pos == 0 && (!self.empty_root_section || ct == "\n") {
            trace!("at: in implicit newline of newline empty root section");
            return Some((self.root, pos));
        }

        let s = Section { id: self.root.id };
        let terminal_newline_in_play = if pos == k - 1 && self.terminal_newline {
            trace!("at: terminal newline in play; adjusting pos");
            pos = k - 2;
            true
        } else {
            trace!("at: terminal not newline in play");
            false
        };
        if !self.empty_root_section && pos > 0 {
            trace!(
                "at: implicit newline of newline empty root section means we need to adjust pos"
            );
            pos -= 1;
        }
        trace!("at: delegating to section at function");
        match s.at(arena, pos) {
            Some((section, offset)) => {
                if terminal_newline_in_play {
                    Some((section, offset + 1))
                } else {
                    Some((section, offset))
                }
            }
            None => None,
        }
    }
}

// Misc methods.
impl Section {
    /// Formats the subtree rooted at `self` as a string.
    pub fn to_rope(self, arena: &Arena) -> Rope {
        section_tree_to_rope(
            self.id, arena, /*terminal_newline=*/ true, /*empty_root_section=*/ true,
        )
    }

    // FIXME: This is pretty inefficient.
    pub fn at(self, arena: &Arena, mut pos: usize) -> Option<(Section, usize)> {
        let root = &arena.arena[self.id].get();
        let k = root.text.len_chars();
        trace!("at: section at {} in {} char section", pos, k);
        if pos < k {
            trace!("at: in section text");
            return Some((self, pos));
        }
        pos -= k;
        for (j, child_id) in self.id.children(&arena.arena).enumerate() {
            trace!(
                "at: in child {} (id {}) of node id {}",
                j,
                child_id,
                self.id
            );
            let k = Section { id: child_id }.to_rope(&arena).len_chars();
            if pos < k {
                let s = Section { id: child_id };
                return match s.at(arena, pos) {
                    Some((section, offset)) => Some((section, offset)),
                    None => Some((s, pos)),
                };
            }
            pos -= k;
        }

        trace!("Exhausted all children of node {}", self.id);

        None
    }

    /// Creates a clone of the entire subtree rooted at `self`. Text is
    /// copy-on-write, but nodes are independent.
    pub fn clone_subtree(self, arena: &mut Arena) -> Section {
        let new_self = arena.clone_section(self);
        let mut stack = vec![(self, new_self)];
        let mut scratch = Vec::default();
        while let Some((old_parent, new_parent)) = stack.pop() {
            scratch.extend(old_parent.children(arena));
            for old_child in scratch.drain(..) {
                let new_child = arena.clone_section(old_child);
                new_parent.unchecked_append(arena, new_child);
                stack.push((old_child, new_child));
            }
        }
        new_self
    }
}

// Non-mutating accessors.
impl Section {
    // FIXME: Look at macros/templates to generate these, or just expose the
    // NodeIds directly to the user for use with the Arena.

    pub fn level(self, arena: &Arena) -> u16 {
        arena.arena[self.id].get().level
    }

    pub fn text(self, arena: &Arena) -> &Rope {
        &arena.arena[self.id].get().text
    }

    pub fn parent(self, arena: &Arena) -> Option<Section> {
        arena.arena[self.id].parent().map(|p| Section { id: p })
    }

    pub fn children(self, arena: &Arena) -> Children {
        Children {
            children: self.id.children(&arena.arena),
        }
    }

    pub fn ancestors(self, arena: &Arena) -> Ancestors {
        Ancestors {
            ancestors: self.id.ancestors(&arena.arena),
        }
    }

    pub fn descendants(self, arena: &Arena) -> Descendants {
        Descendants {
            descendants: self.id.descendants(&arena.arena),
        }
    }

    pub fn preceding_siblings(self, arena: &Arena) -> PrecedingSiblings {
        PrecedingSiblings {
            preceding_siblings: self.id.preceding_siblings(&arena.arena),
        }
    }

    pub fn following_siblings(self, arena: &Arena) -> FollowingSiblings {
        FollowingSiblings {
            following_siblings: self.id.following_siblings(&arena.arena),
        }
    }

    pub fn reverse_children(self, arena: &Arena) -> ReverseChildren {
        ReverseChildren {
            reverse_children: self.id.reverse_children(&arena.arena),
        }
    }
}

// Structure mutators
impl Section {
    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the last child of `self`, setting its level to `self`'s + 1
    /// if invalid.
    pub fn append(self, arena: &mut Arena, new_child: Section) -> Result<(), StructureError> {
        let min_level = arena.arena[self.id].get().level + 1;
        arena.section_min_level(new_child, min_level);
        Ok(self.id.checked_append(new_child.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the first child of `self`, setting its level to `self`'s + 1
    /// if invalid.
    pub fn prepend(self, arena: &mut Arena, new_child: Section) -> Result<(), StructureError> {
        let min_level = arena.arena[self.id].get().level + 1;
        arena.section_min_level(new_child, min_level);
        Ok(self.id.checked_prepend(new_child.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it after `self`, setting its level to `self`'s if invalid.
    pub fn insert_after(
        self,
        arena: &mut Arena,
        new_sibling: Section,
    ) -> Result<(), StructureError> {
        let min_level = match self.parent(arena) {
            Some(parent) => parent.level(arena) + 1,
            None => {
                return Err(StructureError::LevelError);
            }
        };

        arena.section_min_level(new_sibling, min_level);
        Ok(self
            .id
            .checked_insert_after(new_sibling.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it before `self`, setting its level to `self`'s if invalid.
    pub fn insert_before(
        self,
        arena: &mut Arena,
        new_sibling: Section,
    ) -> Result<(), StructureError> {
        let min_level = match self.parent(arena) {
            Some(parent) => parent.level(arena) + 1,
            None => {
                return Err(StructureError::LevelError);
            }
        };

        arena.section_min_level(new_sibling, min_level);
        Ok(self
            .id
            .checked_insert_before(new_sibling.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the last child of `self`, returning an error if its level is
    /// not strictly greater.
    pub fn checked_append(
        self,
        arena: &mut Arena,
        new_child: Section,
    ) -> Result<(), StructureError> {
        if arena.arena[new_child.id].get().level <= arena.arena[self.id].get().level {
            return Err(StructureError::LevelError);
        } else {
            Ok(self.id.checked_append(new_child.id, &mut arena.arena)?)
        }
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the first child of `self`, returning an error if its level is
    /// not strictly greater.
    pub fn checked_prepend(
        self,
        arena: &mut Arena,
        new_child: Section,
    ) -> Result<(), StructureError> {
        if arena.arena[new_child.id].get().level <= arena.arena[self.id].get().level {
            return Err(StructureError::LevelError);
        } else {
            Ok(self.id.checked_prepend(new_child.id, &mut arena.arena)?)
        }
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it after `self`, returning an error if its level is not strictly
    /// greater than its new parent's.
    pub fn checked_insert_after(
        self,
        arena: &mut Arena,
        new_sibling: Section,
    ) -> Result<(), StructureError> {
        if let Some(parent) = arena.arena[self.id].parent() {
            if arena.arena[new_sibling.id].get().level <= arena.arena[parent].get().level {
                return Err(StructureError::LevelError);
            }
        }

        Ok(self
            .id
            .checked_insert_after(new_sibling.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it before `self`, returning an error if its level is not strictly
    /// greater than its new parent's.
    pub fn checked_insert_before(
        self,
        arena: &mut Arena,
        new_sibling: Section,
    ) -> Result<(), StructureError> {
        if let Some(parent) = arena.arena[self.id].parent() {
            if arena.arena[new_sibling.id].get().level <= arena.arena[parent].get().level {
                return Err(StructureError::LevelError);
            }
        }

        Ok(self
            .id
            .checked_insert_before(new_sibling.id, &mut arena.arena)?)
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the last child of `self`, panicking if its level is not
    /// strictly greater.
    pub fn unchecked_append(self, arena: &mut Arena, new_child: Section) {
        self.checked_append(arena, new_child)
            .expect("Checked append failed")
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it as the first child of `self`, panicking if its level is not
    /// strictly greater.
    pub fn unchecked_insert_after(self, arena: &mut Arena, new_sibling: Section) {
        self.checked_insert_after(arena, new_sibling)
            .expect("Checked insert after failed")
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it after `self`, panicking if its level is not strictly greater
    /// than its new parent's.
    pub fn unchecked_insert_before(self, arena: &mut Arena, new_sibling: Section) {
        self.checked_insert_before(arena, new_sibling)
            .expect("Checked insert before failed")
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any), and
    /// adds it before `self`, panicking if its level is not strictly greater
    /// than its new parent's.
    pub fn unchecked_prepend(self, arena: &mut Arena, new_sibling: Section) {
        self.checked_prepend(arena, new_sibling)
            .expect("Checked prepend failed")
    }

    /// Detaches the subtree rooted at `new_child` from its parent (if any). The
    /// nodes remain in the arena and may be reused, or their memory will be
    /// freed when the document is emitted and reparsed.
    pub fn remove_subtree(self, arena: &mut Arena) {
        self.id.detach(&mut arena.arena)
    }

    /// Removes this node, attaching its children to its former parent in the
    /// same place. The node remains in the arena and may be reused.
    pub fn replace_with_children(self, arena: &mut Arena) {
        self.id.remove(&mut arena.arena)
    }

    /// Detaches all children.
    pub fn remove_children(self, arena: &mut Arena) {
        while let Some(child) = self.children(arena).next() {
            child.remove_subtree(arena);
        }
    }
}

// Convenience accessors that parse the headline to return the value.
// It's about as efficient as https://www.youtube.com/watch?v=-DKCFjm0DvE
// For more efficient use, call parse_headline and use that to access multiple.
//
// Note that `set_raw` and `set_level` are available even without
// `headline-parser` feature.
#[cfg(feature = "headline-parser")]
impl Section {
    pub fn priority(
        self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<char>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(h.priority()),
        }
    }

    pub fn raw_tags<'a>(
        self,
        arena: &'a Arena,
        context: Option<&Context>,
    ) -> Result<Cow<'a, str>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(Cow::Owned(h.raw_tags().to_string())),
        }
    }

    pub fn tags<'a>(
        self,
        arena: &'a Arena,
        context: Option<&Context>,
    ) -> Result<Vec<String>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(h.tags().map(|s| s.to_string()).collect()),
        }
    }

    pub fn has_tag(
        self,
        tag: &str,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<bool, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(h.has_tag(tag)),
        }
    }

    pub fn keyword<'a>(
        self,
        arena: &'a Arena,
        context: Option<&Context>,
    ) -> Result<Option<Cow<'a, str>>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(h.keyword().map(|s| Cow::Owned(s.to_string()))),
        }
    }

    pub fn title<'a>(
        self,
        arena: &'a Arena,
        context: Option<&Context>,
    ) -> Result<Cow<'a, str>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(Cow::Owned(h.title().to_string())),
        }
    }

    pub fn commented(
        self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<bool, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(h.commented()),
        }
    }

    pub fn body<'a>(
        self,
        arena: &'a Arena,
        context: Option<&Context>,
    ) -> Result<Cow<'a, str>, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(Cow::Owned(h.body().to_string())),
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn has_property(
        &self,
        arena: &Arena,
        property: &str,
        context: Option<&Context>,
    ) -> Result<bool, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        has_property_internal(property, &org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_property(
        &self,
        arena: &Arena,
        property: &str,
        context: Option<&Context>,
    ) -> Result<Option<Cow<'static, str>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        get_property_internal(property, &org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_closed(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        get_closed_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_deadline(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        get_deadline_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_scheduled(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        get_scheduled_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_id(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<Cow<'static, str>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        get_id_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn properties(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        properties_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn planning(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<Option<orgize::elements::Planning<'static>>, HeadlineError> {
        let org = self.orgize_headline(arena, context)?;
        planning_internal(&org)
    }

    // Not public because we don't support Orgize keyword context.
    #[cfg(feature = "orgize-integration")]
    fn orgize_headline(
        &self,
        arena: &Arena,
        context: Option<&Context>,
    ) -> Result<orgize::Org, HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => Ok(parse_orgize(&h.body())),
        }
    }
}

// Convenience mutators that change the headline in-place. These are somewhat
// inefficient, as each time you call one, it will parse the entire headline,
// change it, then generate it as text. Use a HeadlineBuilder if you care.
#[cfg(feature = "headline-parser")]
impl Section {
    pub fn set_raw_tags(
        self,
        arena: &mut Arena,
        raw_tags: &str,
        context: Option<&Context>,
    ) -> Result<(), HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.set_raw_tags(raw_tags);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn set_tags<'a, I>(
        self,
        arena: &'a mut Arena,
        tags: I,
        context: Option<&Context>,
    ) -> Result<(), HeadlineError>
    where
        I: Iterator<Item = Cow<'a, str>>,
    {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                // FIXME: Figure out lifetime.
                h.set_tags(tags.map(|s| s.to_owned()));
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn update_tags<'a, I>(
        self,
        arena: &'a mut Arena,
        tags: I,
        context: Option<&Context>,
    ) -> Result<(), HeadlineError>
    where
        I: Iterator<Item = Cow<'a, str>>,
    {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                // FIXME: Figure out lifetime.
                h.update_tags(tags.map(|s| s.to_owned()));
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn remove_tags(
        self,
        arena: &mut Arena,
        tags: &[&str],
        context: Option<&Context>,
    ) -> Result<(), HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.remove_tags(tags);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn clear_tags(
        self,
        arena: &mut Arena,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, None).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.clear_tags();
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn add_tag(
        self,
        arena: &mut Arena,
        tag: &str,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.add_tag(tag);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn clear_tag<'a>(
        self,
        arena: &mut Arena,
        tag: &str,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.clear_tag(tag);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn set_keyword(
        self,
        arena: &mut Arena,
        keyword: Option<Rope>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.keyword(keyword);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn set_title(
        self,
        arena: &mut Arena,
        title: Rope,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.title(title);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn set_commented(
        self,
        arena: &mut Arena,
        commented: bool,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.commented(commented);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    pub fn set_body(
        self,
        arena: &mut Arena,
        body: Rope,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context).map(|h| h.to_owned()) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut h = h.to_builder();
                h.body(body);
                self.set_headline(arena, &h.headline(context)?)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_property(
        self,
        arena: &mut Arena,
        property: &str,
        value: &str,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                set_property_internal(&mut org, property, value)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn clear_property(
        self,
        arena: &mut Arena,
        property: &str,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                clear_property_internal(&mut org, property)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_properties(
        self,
        arena: &mut Arena,
        properties: indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                set_properties_internal(&mut org, properties)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn generate_id(
        self,
        arena: &mut Arena,
        context: Option<&Context>,
    ) -> Result<Cow<'static, str>, crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                if let Some(id) = get_property_internal("ID", &org)? {
                    return Ok(id.to_owned());
                }
                let id = generate_id_internal(&mut org)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)?;
                Ok(id)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_planning(
        self,
        arena: &mut Arena,
        planning: Option<orgize::elements::Planning<'static>>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                let id = set_planning_internal(&mut org, planning)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)?;
                Ok(id)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_scheduled(
        self,
        arena: &mut Arena,
        scheduled: Option<orgize::elements::Timestamp<'static>>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                let id = set_scheduled_internal(&mut org, scheduled)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)?;
                Ok(id)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_closed(
        self,
        arena: &mut Arena,
        closed: Option<orgize::elements::Timestamp<'static>>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                let id = set_closed_internal(&mut org, closed)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)?;
                Ok(id)
            }
        }
    }

    #[cfg(feature = "orgize-integration")]
    pub fn set_deadline(
        self,
        arena: &mut Arena,
        deadline: Option<orgize::elements::Timestamp<'static>>,
        context: Option<&Context>,
    ) -> Result<(), crate::errors::HeadlineError> {
        match self.parse_headline(arena, context) {
            None => Err(HeadlineError::InvalidHeadlineError),
            Some(h) => {
                let mut org = parse_orgize(h.body());
                let id = set_deadline_internal(&mut org, deadline)?;
                let mut h = h.to_builder();
                h.body(emit_orgize(&org));
                let h = h.headline(context)?;
                self.set_headline(arena, &h)?;
                Ok(id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newline() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("");

        assert!(doc.at(&arena, 0).is_none());

        let doc = arena.parse_str("\n");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.id);
        assert_eq!(offset, 0);

        assert!(doc.at(&arena, 1).is_none());

        let doc = arena.parse_str("\n\n");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 1).unwrap();
        assert_eq!(section.id, doc.root.id);
        assert_eq!(offset, 1);

        assert!(doc.at(&arena, 2).is_none());
    }

    #[test]
    fn test_empty() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("");
        assert!(doc.at(&arena, 0).is_none());
    }

    #[test]
    fn test_at_o() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("* foo\n* bar\n");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 1).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 2).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 3).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 4).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 4);

        // The newline between "foo" and "bar".
        let (section, offset) = doc.at(&arena, 5).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 5);

        let (section, offset) = doc.at(&arena, 6).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 7).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 8).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 9).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 10).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 4);

        let (section, offset) = doc.at(&arena, 11).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 5);

        assert!(doc.at(&arena, 12).is_none());
    }

    #[test]
    fn test_at_two() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("* foo\n* bar");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 1).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 2).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 3).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 4).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 4);

        // The newline between "foo" and "bar".
        let (section, offset) = doc.at(&arena, 5).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 5);

        let (section, offset) = doc.at(&arena, 6).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 7).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 8).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 9).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 10).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 4);

        assert!(doc.at(&arena, 11).is_none());
    }

    #[test]
    fn test_at_thre() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("\n* foo\n* bar");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 1).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 2).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 3).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 4).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 5).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 4);

        // The newline between "foo" and "bar".
        let (section, offset) = doc.at(&arena, 6).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 5);

        let (section, offset) = doc.at(&arena, 7).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 8).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 9).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 10).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 11).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 4);

        assert!(doc.at(&arena, 12).is_none());
    }

    #[test]
    fn test_at_fo() {
        let mut arena = Arena::default();
        let doc = arena.parse_str("\n* foo\n* bar\n");

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, doc.root.id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 1).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 2).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 3).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 4).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 5).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 4);

        // The newline between "foo" and "bar".
        let (section, offset) = doc.at(&arena, 6).unwrap();
        assert_eq!(section.id, doc.root.children(&arena).next().unwrap().id);
        assert_eq!(offset, 5);

        let (section, offset) = doc.at(&arena, 7).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 8).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 1);

        let (section, offset) = doc.at(&arena, 9).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 2);

        let (section, offset) = doc.at(&arena, 10).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 11).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 4);

        let (section, offset) = doc.at(&arena, 12).unwrap();
        assert_eq!(
            section.id,
            doc.root.children(&arena).skip(1).next().unwrap().id
        );
        assert_eq!(offset, 5);

        assert!(doc.at(&arena, 13).is_none());
    }

    #[test]
    fn test_at_nest() {
        let mut arena = Arena::default();
        let s = "* foo\n** bar\n*** baz\n* qux\n".to_string();
        let doc = arena.parse_str(&s);
        let foo = doc.root.children(&arena).next().unwrap();
        let bar = foo.children(&arena).next().unwrap();
        let baz = bar.children(&arena).next().unwrap();

        let (section, offset) = doc.at(&arena, 0).unwrap();
        assert_eq!(section.id, foo.id);
        assert_eq!(offset, 0);

        let (section, offset) = doc.at(&arena, 8).unwrap();
        assert_eq!(section.id, bar.id);
        assert_eq!(offset, 3);

        let (section, offset) = doc.at(&arena, 18).unwrap();
        assert_eq!(section.id, baz.id);
        assert_eq!(offset, 7);
    }
}
