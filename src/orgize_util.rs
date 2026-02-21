use std::borrow::Cow;

use rand::RngCore;
// Wrappers for things we delegate to Orgize. We prioritize convenience over
// performance for these.
//
// `orgize::Org` represents the entire document, but for our purposes here, it
// is a document consisting of at most one headline.

use crate::*;

pub(crate) fn has_property_internal(
    property: &str,
    org: &orgize::Org<'_>,
) -> Result<bool, HeadlineError> {
    let title = get_title_internal(org)?;
    Ok(title
        .properties
        .pairs
        .iter()
        .any(|(k, _)| k.as_ref() == property))
}

pub(crate) fn get_property_internal(
    property: &'_ str,
    org: &orgize::Org,
) -> Result<Option<Cow<'static, str>>, HeadlineError> {
    let title = get_title_internal(org)?;
    let p = title
        .properties
        .pairs
        .iter()
        .find_map(|(k, v)| (k.as_ref() == property).then(|| Cow::Owned(v.to_string())));
    Ok(p)
}

pub(crate) fn get_id_internal(
    org: &orgize::Org,
) -> Result<Option<Cow<'static, str>>, HeadlineError> {
    get_property_internal("ID", org)
}

pub(crate) fn properties_internal(
    org: &orgize::Org,
) -> Result<indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>, HeadlineError> {
    let p = get_title_internal(org)?
        .properties
        .iter()
        .map(|(k, v)| (Cow::Owned(k.to_string()), Cow::Owned(v.to_string())))
        .collect();
    Ok(p)
}

pub fn clear_property_internal(
    org: &mut orgize::Org,
    key: &str,
) -> Result<(), crate::errors::HeadlineError> {
    let t = &mut get_title_mut_internal(org)?;
    let pairs = &mut t.properties.pairs;
    if let Some(id_index) = pairs.iter().position(|(k, _)| k.as_ref() == key) {
        pairs.remove(id_index);
    }
    Ok(())
}

pub fn set_property_internal(
    org: &mut orgize::Org,
    key: &str,
    value: &str,
) -> Result<(), crate::errors::HeadlineError> {
    let t = &mut get_title_mut_internal(org)?;
    let pairs = &mut t.properties.pairs;
    match pairs.iter().position(|(k, _)| k.as_ref() == key) {
        Some(id_index) => match pairs[id_index].1 {
            Cow::Borrowed(..) => {
                pairs[id_index].1 = value.to_string().into();
            }
            Cow::Owned(..) => {
                pairs[id_index].1.to_mut().replace_range(.., value);
            }
        },
        None => pairs.push((key.to_string().into(), value.to_string().into())),
    };
    Ok(())
}

pub fn set_properties_internal(
    org: &mut orgize::Org,
    properties: indexmap::IndexMap<Cow<'static, str>, Cow<'static, str>>,
) -> Result<(), crate::errors::HeadlineError> {
    let t = &mut get_title_mut_internal(org)?;
    t.properties = properties.into_iter().collect();

    Ok(())
}

pub fn generate_id_internal(
    org: &mut orgize::Org,
) -> Result<Cow<'static, str>, crate::errors::HeadlineError> {
    if let Some(id) = get_property_internal("ID", org)? {
        return Ok(id.clone());
    }

    let mut bytes = [0; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let bytes = hex::encode(bytes);
    let bytes = format!(
        "{}-{}-{}-{}-{}",
        &bytes[..8],
        &bytes[8..12],
        &bytes[12..16],
        &bytes[16..20],
        &bytes[20..]
    );
    set_property_internal(org, "ID", &bytes)?;
    Ok(bytes.into())
}

fn get_title_mut_internal<'a, 'b>(
    org: &'a mut orgize::Org<'b>,
) -> Result<&'a mut orgize::elements::Title<'b>, crate::errors::HeadlineError> {
    let id = match org.headlines().next().map(|s| s.title_node()) {
        None => return Err(HeadlineError::InvalidHeadlineError),
        Some(headline) => headline,
    };

    match &mut org[id] {
        orgize::elements::Element::Title(t) => Ok(t),
        _ => Err(HeadlineError::InvalidHeadlineError),
    }
}

fn get_title_internal<'a, 'b>(
    org: &'a orgize::Org<'b>,
) -> Result<&'a orgize::elements::Title<'b>, crate::errors::HeadlineError> {
    let id = match org.headlines().next().map(|s| s.title_node()) {
        None => return Err(HeadlineError::InvalidHeadlineError),
        Some(headline) => headline,
    };

    match &org[id] {
        orgize::elements::Element::Title(t) => Ok(t),
        _ => Err(HeadlineError::InvalidHeadlineError),
    }
}
