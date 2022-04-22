use std::borrow::Cow;

use ropey::Rope;

use crate::*;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Headline(pub(crate) HeadlinePod);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanningKeyword {
    Deadline,
    Scheduled,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPattern {
    pub keyword: PlanningKeyword,
    pub timestamp: Timestamp<'static>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Planning<'a> {
    pub deadline: Option<Timestamp<'a>>,
    pub scheduled: Option<Timestamp<'a>>,
    pub closed: Option<Timestamp<'a>>,
}

impl Planning<'_> {
    pub fn into_owned(self) -> Planning<'static> {
        Planning {
            scheduled: self.scheduled.map(|s| s.into_owned()),
            closed: self.closed.map(|s| s.into_owned()),
            deadline: self.deadline.map(|s| s.into_owned()),
        }
    }

    pub fn to_borrowed<'a>(&'a self) -> Planning<'a> {
        Planning {
            scheduled: self.scheduled.as_ref().map(|s| s.to_borrowed()),
            closed: self.closed.as_ref().map(|s| s.to_borrowed()),
            deadline: self.deadline.as_ref().map(|s| s.to_borrowed()),
        }
    }
}

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

    // A missing planning line is denoted as having the default value.
    pub fn planning(&self) -> &Planning {
        &self.0.planning
    }

    pub fn scheduled(&self) -> Option<Timestamp<'_>> {
        self.0.planning.scheduled.as_ref().map(|s| s.to_borrowed())
    }

    pub fn deadline(&self) -> Option<Timestamp<'_>> {
        self.0.planning.deadline.as_ref().map(|s| s.to_borrowed())
    }

    pub fn closed(&self) -> Option<Timestamp<'_>> {
        self.0.planning.closed.as_ref().map(|s| s.to_borrowed())
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeadlinePod {
    pub level: u16,
    pub priority: Option<char>,

    // https://github.com/cessen/ropey/issues/47
    pub raw_tags_rope: Rope,
    pub raw_tags_string: String,

    pub keyword: Option<Rope>,
    pub title: Rope,
    pub commented: bool,

    pub planning: Planning<'static>,

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
            planning: Planning::default(),
            body: Rope::default(),
        }
    }
}
