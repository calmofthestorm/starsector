use std::borrow::Cow;
use std::collections::HashSet;
use std::io::Read;

use itertools::Itertools;
use ropey::Rope;

use crate::*;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HeadlineBuilder(pub(crate) HeadlinePod);

impl HeadlineBuilder {
    pub fn level(&mut self, level: u16) -> &mut HeadlineBuilder {
        self.0.level = level;
        self
    }

    pub fn priority(&mut self, priority: Option<char>) -> &mut HeadlineBuilder {
        self.0.priority = priority;
        self
    }

    // Strictly speaking, Org does not prohibit duplicate tags, nor
    // sequences of ::, so we will preserve that if you parse it. This
    // function will canonicalize the tags.
    pub fn canonical_tags(&mut self) -> &mut HeadlineBuilder {
        if !self.0.raw_tags_string.is_empty() {
            let tags = self
                .0
                .raw_tags_string
                .split(':')
                .filter(|t| !t.is_empty())
                .unique()
                .join(":");
            if tags != self.0.raw_tags_string {
                self.set_raw_tags_string(tags);
            }
        }
        self
    }

    pub fn set_raw_tags(&mut self, raw_tags: &str) -> &mut HeadlineBuilder {
        if raw_tags != self.0.raw_tags_string {
            let raw_tags = raw_tags.to_string();
            self.0.raw_tags_string = raw_tags.clone();
            self.0.raw_tags_rope = Rope::from(raw_tags);
        }
        self
    }

    pub fn set_raw_tags_string(&mut self, raw_tags: String) -> &mut HeadlineBuilder {
        if raw_tags != self.0.raw_tags_string {
            self.0.raw_tags_string = raw_tags.clone();
            self.0.raw_tags_rope = Rope::from(raw_tags);
        }
        self
    }

    pub fn set_tags<'a, I>(&mut self, mut tags: I) -> &mut HeadlineBuilder
    where
        I: Iterator<Item = Cow<'a, str>>,
    {
        self.set_raw_tags_string(tags.join(":"))
    }

    pub fn update_tags<'a, I>(&mut self, tags: I) -> &mut HeadlineBuilder
    where
        I: Iterator<Item = Cow<'a, str>>,
    {
        if self.0.raw_tags_string.is_empty() {
            self.set_tags(tags)
        } else {
            let mut tag_set = HashSet::new();
            let mut tag_str = String::with_capacity(self.0.raw_tags_string.len());

            for tag in self.0.raw_tags_string.split(':') {
                if !tag.is_empty() && tag_set.insert(tag.into()) {
                    if !tag_str.is_empty() {
                        tag_str.push(':');
                    }
                    tag_str.push_str(tag);
                }
            }

            for tag in tags {
                // FIXME: use get_or_insert once stable.
                let ck = tag_str.len();
                if !tag_str.is_empty() {
                    tag_str.push(':');
                }
                tag_str.push_str(&*tag);
                if tag.is_empty() || !tag_set.insert(tag) {
                    tag_str.truncate(ck);
                }
            }

            if tag_str != self.0.raw_tags_string {
                self.set_raw_tags_string(tag_str);
            }
            self
        }
    }

    pub fn clear_tag(&mut self, tag: &str) -> &mut HeadlineBuilder {
        if self.has_tag(tag) {
            let tags = self
                .0
                .raw_tags_string
                .split(':')
                .filter(|t| tag != *t)
                .join(":");
            self.set_raw_tags_string(tags);
        }
        self
    }

    pub fn remove_tags(&mut self, tags: &[&str]) -> &mut HeadlineBuilder {
        let tags = self
            .0
            .raw_tags_string
            .split(':')
            .filter(|t| !tags.contains(t))
            .join(":");
        self.set_raw_tags_string(tags)
    }

    pub fn clear_tags(&mut self) -> &mut HeadlineBuilder {
        self.set_raw_tags("")
    }

    pub fn add_tag(&mut self, tag: &str) -> &mut HeadlineBuilder {
        if !self.has_tag(tag) {
            let mut tags = std::mem::take(&mut self.0.raw_tags_string);
            tags.reserve(tag.len() + 1);
            if !tags.is_empty() {
                tags.push(':');
            }
            tags += tag;
            self.set_raw_tags_string(tags);
        }
        self
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.0.raw_tags_string.split(':').any(|t| t == tag)
    }

    pub fn keyword(&mut self, keyword: Option<Rope>) -> &mut HeadlineBuilder {
        self.0.keyword = keyword;
        self
    }

    pub fn title(&mut self, title: Rope) -> &mut HeadlineBuilder {
        self.0.title = title;
        self
    }

    pub fn commented(&mut self, commented: bool) -> &mut HeadlineBuilder {
        self.0.commented = commented;
        self
    }

    pub fn body(&mut self, body: Rope) -> &mut HeadlineBuilder {
        self.0.body = body;
        self
    }

    #[cfg(feature = "orgize-integration")]
    pub fn clear_property(
        &mut self,
        property: &str,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        clear_property_internal(&mut org, property)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn property(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_property_internal(&mut org, key, value)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn properties(
        &mut self,
        properties: indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_properties_internal(&mut org, properties)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn generate_id(&mut self) -> Result<Cow<'static, str>, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        if let Some(id) = get_property_internal("ID", &org)? {
            return Ok(id.to_owned());
        }
        let id = generate_id_internal(&mut org)?;
        self.0.body = emit_orgize(&org);
        Ok(id)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn planning(
        &mut self,
        planning: Option<orgize::elements::Planning<'static>>,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_planning_internal(&mut org, planning)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn scheduled(
        &mut self,
        scheduled: Option<orgize::elements::Timestamp<'static>>,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_scheduled_internal(&mut org, scheduled)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn closed(
        &mut self,
        closed: Option<orgize::elements::Timestamp<'static>>,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_closed_internal(&mut org, closed)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn deadline(
        &mut self,
        deadline: Option<orgize::elements::Timestamp<'static>>,
    ) -> Result<&mut HeadlineBuilder, crate::errors::HeadlineError> {
        let mut org = parse_orgize(&self.0.body);
        set_deadline_internal(&mut org, deadline)?;
        self.0.body = emit_orgize(&org);
        Ok(self)
    }
}

// We don't need to give Orgize a non-default context because we do not use it
// to parse the stars line itself.
pub(crate) fn parse_orgize(body: &Rope) -> orgize::Org<'static> {
    // FIXME: Limit to planning and drawer. That way it won't mess up other
    // formatting. Do keep in mind children are safe.
    orgize::Org::parse_string(format!("* a\n{}", body))
}

pub(crate) fn emit_orgize(org: &orgize::Org) -> Rope {
    // FIXME: Limit to planning and drawer. That way it won't mess up other
    // formatting. Do keep in mind children are safe.
    let mut s = String::default();
    let mut iob = iobuffer::IoBuffer::default();
    org.write_org(&mut iob).unwrap();
    iob.read_full_line(b'\n').unwrap();
    iob.read_to_string(&mut s).unwrap();

    if s.chars().last() == Some('\n') {
        s.pop();
    }

    Rope::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_properties() {
        let con = crate::headline::parser::Context::default();
        let a = Rope::from("* Hello\n:PROPERTIES:\n:FOO: bar\n:END:");
        let headline = crate::headline::parser::parse_valid_single_headline(a.slice(..), &con);
        let mut h = headline.to_builder();

        let mut p = headline.properties().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p.get("FOO").unwrap(), "bar");

        p.insert("FOO".into(), "baz".into());
        p.insert("other".into(), "ones".into());
        p.insert("nothing".into(), "".into());

        h.properties(p).unwrap();

        let a = h.headline(None).unwrap().to_rope();
        let h = crate::headline::parser::parse_valid_single_headline(a.slice(..), &con);
        let p = h.properties().unwrap();
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn test_parse_emit() {
        let a = Rope::from("* Hello");
        let org = parse_orgize(&a);
        let b = emit_orgize(&org);
        assert_eq!(a, b);

        let a = Rope::from("* Hello\n:PROPERTIES:\n:FOO: bar\n:END:");
        let org = parse_orgize(&a);
        let b = emit_orgize(&org);
        assert_eq!(a, b);
    }
}
