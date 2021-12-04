use std::borrow::Cow;

use ropey::Rope;

use crate::*;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Headline(pub(crate) HeadlinePod);

impl Headline {
    pub fn level(&self) -> u16 {
        self.0.level
    }

    pub fn priority(&self) -> Option<char> {
        self.0.priority
    }

    pub fn raw_tags(&self) -> &Rope {
        &self.0.raw_tags_rope
    }

    pub fn tags(&self) -> Tags {
        if self.0.raw_tags_string.is_empty() {
            Tags { split: None }
        } else {
            Tags {
                split: Some(self.0.raw_tags_string.split(':')),
            }
        }
    }

    pub fn has_tag(&self, needle: &str) -> bool {
        for tag in self.tags() {
            if tag == needle {
                return true;
            }
        }

        false
    }

    pub fn keyword(&self) -> Option<&Rope> {
        self.0.keyword.as_ref()
    }

    pub fn title(&self) -> &Rope {
        &self.0.title
    }

    pub fn commented(&self) -> bool {
        self.0.commented
    }

    pub fn body(&self) -> &Rope {
        &self.0.body
    }

    pub fn to_builder(&self) -> HeadlineBuilder {
        self.0.to_builder()
    }

    pub fn to_rope(&self) -> Rope {
        self.0.to_rope()
    }

    #[cfg(feature = "orgize-integration")]
    pub fn properties(
        &self,
    ) -> Result<indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>, HeadlineError> {
        properties_internal(&parse_orgize(&self.0.body))
    }

    #[cfg(feature = "orgize-integration")]
    pub fn get_property(&self, property: &str) -> Result<Option<Rope>, HeadlineError> {
        let org = parse_orgize(&self.body());
        let p = org.headlines().next().and_then(|headline| {
            let pairs = &headline.title(&org).properties.pairs;
            match pairs.binary_search_by(|(k, _)| k.as_ref().cmp(property)) {
                Ok(p_index) => Some(pairs[p_index].1.to_string().into()),
                _ => None,
            }
        });
        Ok(p)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn planning(&self) -> Result<Option<orgize::elements::Planning<'static>>, HeadlineError> {
        let org = parse_orgize(&self.0.body);
        planning_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn scheduled(&self) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = parse_orgize(&self.0.body);
        get_scheduled_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn deadline(&self) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = parse_orgize(&self.0.body);
        get_deadline_internal(&org)
    }

    #[cfg(feature = "orgize-integration")]
    pub fn closed(&self) -> Result<Option<orgize::elements::Timestamp<'static>>, HeadlineError> {
        let org = parse_orgize(&self.0.body);
        get_closed_internal(&org)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HeadlinePod {
    pub level: u16,
    pub priority: Option<char>,

    // https://github.com/cessen/ropey/issues/47
    pub raw_tags_rope: Rope,
    pub raw_tags_string: String,

    pub keyword: Option<Rope>,
    pub title: Rope,
    pub commented: bool,

    pub(crate) body: Rope,
}

impl HeadlinePod {
    pub fn to_builder(&self) -> HeadlineBuilder {
        HeadlineBuilder(self.clone())
    }
}

impl Default for HeadlinePod {
    fn default() -> HeadlinePod {
        HeadlinePod {
            level: 1,
            priority: None,
            raw_tags_rope: Rope::default(),
            raw_tags_string: String::default(),
            keyword: None,
            title: Rope::default(),
            commented: false,
            body: Rope::default(),
        }
    }
}
