use std::borrow::Cow;
use std::result::Result;

use itertools::Itertools;
use ropey::{Rope, RopeSlice};

use crate::{
    Arena, Headline, HeadlineBuilder, HeadlineError, HeadlinePod, RopeExt, Section, StructureError,
};

lazy_static! {
    static ref TAG_VALIDATION_RE: regex::Regex =
        regex::Regex::new("[\\w@#%:]*").expect("failed to assemble headline regex");
    static ref CONTAINS_HEADLINE_RE: regex::Regex =
        regex::Regex::new("(^|.*\n)\\*\\** .*").expect("failed to assemble headline regex");
    static ref DEFAULT_CONTEXT: Context<'static> = Context::default();
}

#[derive(Debug, Clone)]
pub struct Context<'a> {
    pub(crate) keywords: Cow<'a, str>,
}

impl Context<'_> {
    pub fn default() -> Context<'static> {
        Context::new("TODO:DONE".into())
    }

    pub fn new<'a>(keywords: Cow<'a, str>) -> Context<'a> {
        Context { keywords }
    }

    pub fn from_keywords(keywords: &[&str]) -> Context<'static> {
        Context {
            keywords: Cow::Owned(keywords.iter().join(":")),
        }
    }
}

pub(crate) fn context_or<'a, 'b>(context: Option<&'b Context<'a>>) -> &'b Context<'a> {
    match context {
        Some(context) => context,
        None => &DEFAULT_CONTEXT,
    }
}

impl Section {
    pub fn parse_headline(self, arena: &Arena, context: Option<&Context>) -> Option<Headline> {
        if self.level(&arena) > 0 {
            Some(parse_valid_single_headline(
                self.text(&arena).slice(..),
                context_or(context),
            ))
        } else {
            None
        }
    }

    pub fn set_headline(self, arena: &mut Arena, headline: &Headline) -> Result<(), HeadlineError> {
        match arena.new_section(headline.to_rope()) {
            None => Err(HeadlineError::NonEquivalentReparseError),
            Some(section) => {
                if let Some(parent) = self.parent(arena) {
                    if parent.level(arena) >= section.level(arena) {
                        // FIXME: structureerror
                        return Err(HeadlineError::InvalidLevelError);
                    }
                }

                // FIXME: Refactor
                *arena.arena[self.id].get_mut() = std::mem::take(arena.arena[section.id].get_mut());
                section.id.remove(&mut arena.arena);

                Ok(())
            }
        }
    }

    pub fn set_level(self, arena: &mut Arena, level: u16) -> Result<(), StructureError> {
        if let Some(parent) = self.parent(arena) {
            if parent.level(arena) >= level {
                return Err(StructureError::LevelError);
            }
        }

        arena.set_level(self, level);
        Ok(())
    }

    pub fn set_raw(self, arena: &mut Arena, raw: Rope) -> Result<(), HeadlineError> {
        match arena.new_section(raw) {
            None => Err(HeadlineError::InvalidBodyError),
            Some(section) if section.children(arena).count() > 0 => {
                Err(HeadlineError::InvalidBodyError)
            }
            Some(section) => {
                if let Some(parent) = self.parent(arena) {
                    if parent.level(arena) >= section.level(arena) {
                        // FIXME: structureerror
                        return Err(HeadlineError::InvalidLevelError);
                    }
                }

                // FIXME: Refactor
                *arena.arena[self.id].get_mut() = std::mem::take(arena.arena[section.id].get_mut());
                section.id.remove(&mut arena.arena);
                Ok(())
            }
        }
    }
}

impl HeadlineBuilder {
    // Doesn't check title for tricks like injecting keywords, priority,
    // etc. Otherwise, should be complete.
    pub fn validate_partially(&self, context: Option<&Context>) -> Result<(), HeadlineError> {
        let context = context_or(context);

        if self.0.level == 0 {
            return Err(HeadlineError::InvalidLevelError);
        };

        if let Some(c) = self.0.priority {
            if !c.is_ascii_uppercase() {
                return Err(HeadlineError::InvalidPriorityError);
            }
        }

        if !self.0.raw_tags_string.is_empty()
            && !TAG_VALIDATION_RE.is_match(&self.0.raw_tags_string)
        {
            return Err(HeadlineError::InvalidTagsError);
        }

        if let Some(keyword) = &self.0.keyword {
            if !context.keywords.split(':').any(|k| k == keyword) {
                return Err(HeadlineError::InvalidKeywordError);
            }
        }

        if CONTAINS_HEADLINE_RE.is_match(&*self.0.body.to_contiguous()) {
            return Err(HeadlineError::InvalidBodyError);
        }

        Ok(())
    }

    pub fn headline(&self, context: Option<&Context>) -> Result<Headline, HeadlineError> {
        let headline = self.to_rope(context)?;
        let headline = parse_valid_single_headline(headline.slice(..), context_or(context));

        if headline.to_builder() != *self {
            return Err(HeadlineError::NonEquivalentReparseError);
        }

        // We use this instead of the freshly parsed version, after verifying
        // they are identical, to take advantage of borrowed values where
        // possible.
        Ok(Headline(HeadlinePod {
            level: self.0.level,
            priority: self.0.priority,
            raw_tags_string: self.0.raw_tags_string.clone(),
            raw_tags_rope: self.0.raw_tags_rope.clone(),
            keyword: self.0.keyword.clone(),
            title: self.0.title.clone(),
            commented: self.0.commented,
            body: self.0.body.clone(),
        }))
    }

    pub fn to_rope(&self, context: Option<&Context>) -> Result<Rope, HeadlineError> {
        self.validate_partially(context)?;
        Ok(self.0.to_rope())
    }
}

impl HeadlinePod {
    // Call on HeadlineBuilder or Headline instead.
    pub(crate) fn to_rope(&self) -> Rope {
        let mut capacity = 0;

        if self.level > 0 {
            capacity += self.level as usize + 1;
        }

        if let Some(k) = &self.keyword {
            capacity += k.len_bytes() + 1;
        }

        if self.priority.is_some() {
            capacity += 5;
        }

        if self.commented {
            if self.title.is_empty() {
                capacity += 7;
            } else {
                capacity += 8;
            }
        }

        let mut prefix = String::with_capacity(capacity);

        for _ in 0..self.level {
            prefix.push('*');
        }
        prefix.push(' ');

        if let Some(k) = &self.keyword {
            for chunk in k.chunks() {
                prefix.push_str(chunk);
            }
            prefix.push(' ');
        }

        if let Some(p) = self.priority {
            prefix.push('[');
            prefix.push('#');
            prefix.push(p);
            prefix.push(']');
            prefix.push(' ');
        }

        if self.commented {
            if self.title.is_empty() {
                prefix.push_str("COMMENT");
            } else {
                prefix.push_str("COMMENT ");
            }
        }

        let mut headline = Rope::from(prefix);
        headline.append(self.title.clone());

        if !self.raw_tags_string.is_empty() {
            // FIXME: We could include this in the raw to avoid needing a new string here.
            headline.push_str(" :");
            headline.append(self.raw_tags_rope.clone());
            headline.push(':');
        }

        if !self.body.is_empty() {
            headline.push('\n');
            headline.append(self.body.clone());
        }

        headline
    }
}

// Requires that the string is a valid headline (may include a body, but not
// child headlines).
pub fn parse_valid_single_headline(text: RopeSlice, context: &Context) -> Headline {
    crate::parser::headline::parse_headline(text, context).unwrap()
}
