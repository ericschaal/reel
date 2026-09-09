use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::{Collection, Error};

const CATALOGUE_COLLECTIONS_PATH: &str = "/v1/catalogue/collections";
const CURSOR_VERSION: u8 = 1;

pub(super) fn first_page(collection: &Collection, language: Option<&str>) -> String {
    href(collection, language, None)
}

pub(super) fn next_page(collection: &Collection, language: Option<&str>, page: u32) -> String {
    let cursor = encode_cursor(collection, language, page);
    href(collection, language, Some(&cursor))
}

pub(super) fn page_from_cursor(
    cursor: Option<&str>,
    collection: &Collection,
    language: Option<&str>,
) -> Result<u32, Error> {
    let Some(cursor) = cursor else {
        return Ok(1);
    };
    let encoded = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| Error::InvalidCursor)?;
    let cursor: Cursor = serde_json::from_slice(&encoded).map_err(|_| Error::InvalidCursor)?;
    if cursor.version != CURSOR_VERSION
        || cursor.collection != collection.id()
        || cursor.language.as_deref() != language
        || cursor.page < 2
    {
        return Err(Error::InvalidCursor);
    }
    Ok(cursor.page)
}

fn href(collection: &Collection, language: Option<&str>, cursor: Option<&str>) -> String {
    let path = format!("{CATALOGUE_COLLECTIONS_PATH}/{}", collection.id());
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    if let Some(language) = language {
        query.append_pair("language", language);
    }
    if let Some(cursor) = cursor {
        query.append_pair("cursor", cursor);
    }
    let query = query.finish();
    if query.is_empty() {
        path
    } else {
        format!("{path}?{query}")
    }
}

fn encode_cursor(collection: &Collection, language: Option<&str>, page: u32) -> String {
    let cursor = Cursor {
        version: CURSOR_VERSION,
        collection: collection.id(),
        language: language.map(str::to_owned),
        page,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("cursor serialization cannot fail"))
}

#[derive(Deserialize, Serialize)]
struct Cursor {
    version: u8,
    collection: String,
    language: Option<String>,
    page: u32,
}
