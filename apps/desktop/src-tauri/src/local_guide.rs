//! Fact-only kernel for the local RealmBox guide.
//!
//! This module owns validation, bounded parsing, provenance, uncertainty, and secret
//! redaction. It has no database, Docker, network, model, or Tauri dependency. A runtime
//! adapter may only provide rows from two audited, fixed, read-only lookups through
//! [`LocalGuideDataSource`]. Runtime availability is the adapter's responsibility.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

const MIN_SEARCH_TERM_CHARS: usize = 2;
const MAX_SEARCH_TERM_CHARS: usize = 64;
const MAX_TABULAR_BYTES: usize = 64 * 1_024;
const MAX_ENTRIES: usize = 8;
const MAX_TITLE_CHARS: usize = 120;
const MAX_SUMMARY_CHARS: usize = 320;
const MAX_CATEGORY_CHARS: usize = 80;
const MAX_SOURCE_ID_CHARS: usize = 96;
const MAX_OUTPUT_CHARS: usize = 2_400;

/// Closed lookup catalogue. Callers cannot supply tables, columns, SQL, or commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalGuideKind {
    Quest,
    Item,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalGuideLocale {
    #[serde(rename = "frFR")]
    FrFr,
    #[serde(rename = "enUS")]
    EnUs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalGuideQueryError {
    EmptyTerm,
    TermTooShort,
    TermTooLong,
    UnsupportedCharacter,
}

impl fmt::Display for LocalGuideQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyTerm => "terme de recherche vide",
            Self::TermTooShort => "terme de recherche trop court",
            Self::TermTooLong => "terme de recherche trop long",
            Self::UnsupportedCharacter => "caractère non pris en charge dans la recherche",
        })
    }
}

/// UTF-8 encoded as uppercase hexadecimal. Its private payload can only be created
/// after validating a player term, not from arbitrary caller-supplied SQL text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LocalGuideTermHex(String);

impl LocalGuideTermHex {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated internal query. Deserialization accepts a raw `term` only through the
/// validating constructor, then discards it. Caller-supplied hexadecimal is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGuideQuery {
    pub kind: LocalGuideKind,
    pub locale: LocalGuideLocale,
    pub term_hex: LocalGuideTermHex,
}

impl LocalGuideQuery {
    pub fn new(
        kind: LocalGuideKind,
        player_term: impl AsRef<str>,
        locale: LocalGuideLocale,
    ) -> Result<Self, LocalGuideQueryError> {
        let normalized = normalize_search_term(player_term.as_ref())?;
        Ok(Self {
            kind,
            locale,
            term_hex: LocalGuideTermHex(hex_encode(normalized.as_bytes())),
        })
    }

    pub fn kind(&self) -> LocalGuideKind {
        self.kind
    }

    pub fn locale(&self) -> LocalGuideLocale {
        self.locale
    }

    pub fn term_hex(&self) -> LocalGuideTermHex {
        self.term_hex.clone()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedLocalGuideQuery {
    kind: LocalGuideKind,
    term: String,
    locale: LocalGuideLocale,
}

impl<'de> Deserialize<'de> for LocalGuideQuery {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: Deserializer<'de>,
    {
        let unchecked = UncheckedLocalGuideQuery::deserialize(deserializer)?;
        Self::new(unchecked.kind, unchecked.term, unchecked.locale)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalGuideSourceScope {
    BundledReference,
    InstallationState,
    RuntimeSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGuideProvenance {
    pub scope: LocalGuideSourceScope,
    /// Logical dataset identifier only; never a path, URL, or connection string.
    pub source_id: String,
    /// `None` is preserved as explicit uncertainty rather than guessed.
    pub observed_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGuideMetadata {
    pub level: Option<u16>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGuideEntry {
    pub id: u32,
    pub title: String,
    pub summary: String,
    pub metadata: LocalGuideMetadata,
    pub source: LocalGuideProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalGuideUncertainty {
    None,
    Partial,
    Unavailable,
}

/// Serializable player-safe result. `Unavailable` is quiet: source and parser errors
/// never reach this type. An empty valid lookup instead has provenance and `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGuideSearchResult {
    pub entries: Vec<LocalGuideEntry>,
    pub provenance: Option<LocalGuideProvenance>,
    pub uncertainty: LocalGuideUncertainty,
    pub truncated: bool,
}

impl LocalGuideSearchResult {
    pub fn unavailable() -> Self {
        Self {
            entries: Vec::new(),
            provenance: None,
            uncertainty: LocalGuideUncertainty::Unavailable,
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalGuideSourceError {
    Unavailable,
    InvalidSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGuideSnapshot {
    /// Expected format: `id<TAB>title_hex<TAB>summary_hex<TAB>level<TAB>category_hex`.
    pub rows: String,
    pub provenance: LocalGuideProvenance,
}

/// Typed, local-only, read-only boundary. Implementations must use exactly one audited
/// fixed lookup selected by the method, inside a read-only transaction. They must not
/// use network access, construct SQL from input, mutate a database, or trigger runtime
/// operations. Only validated hexadecimal UTF-8 crosses this boundary.
pub trait LocalGuideDataSource {
    fn quest_rows(
        &self,
        term_hex: &LocalGuideTermHex,
        locale: LocalGuideLocale,
    ) -> Result<Option<LocalGuideSnapshot>, LocalGuideSourceError>;

    fn item_rows(
        &self,
        term_hex: &LocalGuideTermHex,
        locale: LocalGuideLocale,
    ) -> Result<Option<LocalGuideSnapshot>, LocalGuideSourceError>;
}

pub struct LocalGuide<D> {
    source: D,
}

// Compatibility names retained for the runtime adapter's first integration. They
// denote the same validated types and do not introduce an alternate parsing path.
pub type HexEncodedGuideTerm = LocalGuideTermHex;
pub type LocalGuideResponse = LocalGuideSearchResult;
pub type LocalGuideTabularSnapshot = LocalGuideSnapshot;
pub type LocalProvenance = LocalGuideProvenance;
pub type LocalSourceScope = LocalGuideSourceScope;
pub type LocalSourceError = LocalGuideSourceError;
pub type LocalGuideSearch<D> = LocalGuide<D>;
pub use LocalGuideDataSource as LocalGuideSearchDataSource;

impl<D: LocalGuideDataSource> LocalGuide<D> {
    pub fn new(source: D) -> Self {
        Self { source }
    }

    /// Exactly one allowlisted local read; no model, remote, or second-source fallback.
    pub fn search(&self, query: &LocalGuideQuery) -> LocalGuideSearchResult {
        let term_hex = query.term_hex();
        let snapshot = match query.kind() {
            LocalGuideKind::Quest => self.source.quest_rows(&term_hex, query.locale()),
            LocalGuideKind::Item => self.source.item_rows(&term_hex, query.locale()),
        };
        let Some(snapshot) = snapshot.ok().flatten() else {
            return LocalGuideSearchResult::unavailable();
        };
        parse_search_snapshot(query, snapshot)
            .map_err(|_| LocalGuideSourceError::InvalidSnapshot)
            .unwrap_or_else(|_| LocalGuideSearchResult::unavailable())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalGuideParseError {
    InputTooLarge,
    InvalidColumnCount,
    InvalidId,
    DuplicateId,
    InvalidHex,
    InvalidUtf8,
    InvalidLevel,
    MissingTitle,
    MissingProvenance,
}

impl fmt::Display for LocalGuideParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputTooLarge => "résultat local trop volumineux",
            Self::InvalidColumnCount => "colonnes locales invalides",
            Self::InvalidId => "identifiant local invalide",
            Self::DuplicateId => "identifiant local dupliqué",
            Self::InvalidHex => "champ local hexadécimal invalide",
            Self::InvalidUtf8 => "champ local UTF-8 invalide",
            Self::InvalidLevel => "niveau local invalide",
            Self::MissingTitle => "titre local manquant",
            Self::MissingProvenance => "provenance locale manquante",
        })
    }
}

/// Convenience parser for the fixed local runtime lookup. With no clock or adapter
/// metadata, observation time truthfully remains unknown. Exact adapter metadata can
/// instead be passed through [`parse_search_snapshot`].
pub fn parse_search_output(
    query: &LocalGuideQuery,
    output: &str,
) -> Result<LocalGuideSearchResult, LocalGuideParseError> {
    if output.len() > MAX_TABULAR_BYTES {
        return Err(LocalGuideParseError::InputTooLarge);
    }
    parse_search_snapshot(
        query,
        LocalGuideSnapshot {
            rows: output.to_owned(),
            provenance: LocalGuideProvenance {
                scope: LocalGuideSourceScope::RuntimeSnapshot,
                source_id: logical_source_id(query),
                observed_at_unix_ms: None,
            },
        },
    )
}

pub fn parse_search_snapshot(
    _query: &LocalGuideQuery,
    snapshot: LocalGuideSnapshot,
) -> Result<LocalGuideSearchResult, LocalGuideParseError> {
    if snapshot.rows.len() > MAX_TABULAR_BYTES {
        return Err(LocalGuideParseError::InputTooLarge);
    }
    let source_id = truncate_chars(
        redact_sensitive(&snapshot.provenance.source_id).trim(),
        MAX_SOURCE_ID_CHARS,
    );
    if source_id.is_empty() {
        return Err(LocalGuideParseError::MissingProvenance);
    }
    let provenance = LocalGuideProvenance {
        scope: snapshot.provenance.scope,
        source_id,
        observed_at_unix_ms: snapshot.provenance.observed_at_unix_ms,
    };
    let mut entries = Vec::new();
    let mut seen_ids = Vec::new();
    let mut output_chars = 0_usize;
    let mut truncated = false;
    let mut partial = provenance.observed_at_unix_ms.is_none();

    for raw_line in snapshot.rows.lines().filter(|line| !line.trim().is_empty()) {
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            partial = true;
            continue;
        }
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let columns: Vec<_> = line.split('\t').collect();
        if columns.len() != 5 {
            return Err(LocalGuideParseError::InvalidColumnCount);
        }
        let id = columns[0]
            .parse::<u32>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or(LocalGuideParseError::InvalidId)?;
        if seen_ids.contains(&id) {
            return Err(LocalGuideParseError::DuplicateId);
        }
        seen_ids.push(id);

        let (title, title_truncated) = normalize_output_text(
            &decode_hex_utf8(columns[1], MAX_TABULAR_BYTES / 2)?,
            MAX_TITLE_CHARS,
        );
        if title.is_empty() {
            return Err(LocalGuideParseError::MissingTitle);
        }
        let (summary, summary_truncated) = normalize_output_text(
            &decode_hex_utf8(columns[2], MAX_TABULAR_BYTES / 2)?,
            MAX_SUMMARY_CHARS,
        );
        let (category, category_truncated) = normalize_output_text(
            &decode_hex_utf8(columns[4], MAX_TABULAR_BYTES / 2)?,
            MAX_CATEGORY_CHARS,
        );
        if title_truncated || summary_truncated || category_truncated {
            truncated = true;
            partial = true;
        }
        let level = match columns[3] {
            "" | "-" => None,
            raw => Some(
                raw.parse::<u16>()
                    .ok()
                    .filter(|level| *level <= 1_000)
                    .ok_or(LocalGuideParseError::InvalidLevel)?,
            ),
        };
        if summary.is_empty() {
            partial = true;
        }
        let entry_chars =
            title.chars().count() + summary.chars().count() + category.chars().count();
        if output_chars.saturating_add(entry_chars) > MAX_OUTPUT_CHARS {
            truncated = true;
            partial = true;
            break;
        }
        output_chars += entry_chars;
        entries.push(LocalGuideEntry {
            id,
            title,
            summary,
            metadata: LocalGuideMetadata {
                level,
                category: (!category.is_empty()).then_some(category),
            },
            source: provenance.clone(),
        });
    }

    // Zero matching rows is a successful factual result, not source unavailability.
    let uncertainty = if entries.is_empty() {
        LocalGuideUncertainty::None
    } else if partial {
        LocalGuideUncertainty::Partial
    } else {
        LocalGuideUncertainty::None
    };
    Ok(LocalGuideSearchResult {
        entries,
        provenance: Some(provenance),
        uncertainty,
        truncated,
    })
}

fn logical_source_id(query: &LocalGuideQuery) -> String {
    let kind = match query.kind {
        LocalGuideKind::Quest => "quest",
        LocalGuideKind::Item => "item",
    };
    let locale = match query.locale {
        LocalGuideLocale::FrFr => "frFR",
        LocalGuideLocale::EnUs => "enUS",
    };
    format!("azerothcore-world/{kind}/{locale}")
}

fn normalize_search_term(input: &str) -> Result<String, LocalGuideQueryError> {
    if input.chars().any(char::is_control) {
        return Err(LocalGuideQueryError::UnsupportedCharacter);
    }
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let character_count = normalized.chars().count();
    if character_count == 0 {
        return Err(LocalGuideQueryError::EmptyTerm);
    }
    if character_count < MIN_SEARCH_TERM_CHARS {
        return Err(LocalGuideQueryError::TermTooShort);
    }
    if character_count > MAX_SEARCH_TERM_CHARS {
        return Err(LocalGuideQueryError::TermTooLong);
    }
    if normalized.chars().any(|character| {
        !(character.is_alphanumeric()
            || character.is_whitespace()
            || matches!(character, '\'' | '’' | '-' | '.' | ',' | '(' | ')' | '&'))
    }) {
        return Err(LocalGuideQueryError::UnsupportedCharacter);
    }
    Ok(normalized)
}

fn normalize_output_text(input: &str, max_chars: usize) -> (String, bool) {
    let redacted = redact_sensitive(input);
    let printable: String = redacted
        .chars()
        .filter(|character| !character.is_control() || character.is_whitespace())
        .collect();
    let normalized = printable.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated = normalized.chars().count() > max_chars;
    (truncate_chars(&normalized, max_chars), truncated)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex_utf8(value: &str, max_decoded_bytes: usize) -> Result<String, LocalGuideParseError> {
    if !value.len().is_multiple_of(2) || value.len() / 2 > max_decoded_bytes {
        return Err(LocalGuideParseError::InvalidHex);
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0]).ok_or(LocalGuideParseError::InvalidHex)?;
        let low = hex_nibble(pair[1]).ok_or(LocalGuideParseError::InvalidHex)?;
        bytes.push((high << 4) | low);
    }
    String::from_utf8(bytes).map_err(|_| LocalGuideParseError::InvalidUtf8)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() && max_chars > 0 {
        truncated.pop();
        truncated.push('…');
    }
    truncated
}

fn redact_sensitive(input: &str) -> String {
    let mut output = redact_bearer(&redact_url_passwords(input));
    for key in [
        "password",
        "passwd",
        "pwd",
        "token",
        "api_key",
        "apikey",
        "secret",
        "authorization",
    ] {
        output = redact_assignment(&output, key);
    }
    output
}

fn redact_assignment(input: &str, sensitive_key: &str) -> String {
    let lowercase = input.to_ascii_lowercase();
    let mut ranges = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = lowercase[search_from..].find(sensitive_key) {
        let key_start = search_from + relative_start;
        let key_end = key_start + sensitive_key.len();
        let boundary_before =
            key_start == 0 || !lowercase.as_bytes()[key_start - 1].is_ascii_alphanumeric();
        let boundary_after =
            key_end == lowercase.len() || !lowercase.as_bytes()[key_end].is_ascii_alphanumeric();
        if !boundary_before || !boundary_after {
            search_from = key_end;
            continue;
        }
        let bytes = input.as_bytes();
        let mut separator = key_end;
        if separator < bytes.len() && matches!(bytes[separator], b'\'' | b'"') {
            separator += 1;
        }
        while separator < bytes.len() && bytes[separator].is_ascii_whitespace() {
            separator += 1;
        }
        if separator >= bytes.len() || !matches!(bytes[separator], b'=' | b':') {
            search_from = key_end;
            continue;
        }
        separator += 1;
        while separator < bytes.len() && bytes[separator].is_ascii_whitespace() {
            separator += 1;
        }
        let value_end = sensitive_value_end(input, separator, sensitive_key == "authorization");
        if value_end > separator {
            ranges.push((separator, value_end));
        }
        search_from = value_end.max(key_end);
    }
    replace_ranges(input, &ranges)
}

fn sensitive_value_end(input: &str, start: usize, authorization: bool) -> usize {
    let bytes = input.as_bytes();
    if start >= bytes.len() {
        return start;
    }
    // Authentication headers may include a scheme and a credential separated by a
    // space. Conservatively hide the remainder of that line, not only the scheme.
    if authorization {
        return input[start..]
            .find(['\r', '\n'])
            .map_or(input.len(), |offset| start + offset);
    }
    if matches!(bytes[start], b'\'' | b'"') {
        let quote = bytes[start];
        let mut index = start + 1;
        let mut escaped = false;
        while index < bytes.len() {
            if bytes[index] == quote && !escaped {
                return index + 1;
            }
            escaped = bytes[index] == b'\\' && !escaped;
            index += 1;
        }
        return input.len();
    }
    let mut end = start;
    while end < bytes.len()
        && !bytes[end].is_ascii_whitespace()
        && !matches!(bytes[end], b',' | b';' | b'&')
    {
        end += 1;
    }
    end
}

fn redact_bearer(input: &str) -> String {
    let lowercase = input.to_ascii_lowercase();
    let bytes = input.as_bytes();
    let mut ranges = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = lowercase[search_from..].find("bearer ") {
        let value_start = search_from + relative_start + "bearer ".len();
        let mut value_end = value_start;
        while value_end < bytes.len()
            && !bytes[value_end].is_ascii_whitespace()
            && !matches!(bytes[value_end], b',' | b';')
        {
            value_end += 1;
        }
        if value_end > value_start {
            ranges.push((value_start, value_end));
        }
        search_from = value_end.max(value_start);
    }
    replace_ranges(input, &ranges)
}

fn redact_url_passwords(input: &str) -> String {
    let mut ranges = Vec::new();
    let mut search_from = 0;
    while let Some(relative_scheme) = input[search_from..].find("://") {
        let authority_start = search_from + relative_scheme + 3;
        let authority_end = input[authority_start..]
            .find(['/', '?', '#', ' '])
            .map_or(input.len(), |offset| authority_start + offset);
        let authority = &input[authority_start..authority_end];
        if let (Some(colon), Some(at)) = (authority.find(':'), authority.rfind('@'))
            && colon < at
        {
            ranges.push((authority_start + colon + 1, authority_start + at));
        }
        search_from = authority_end.max(authority_start);
        if search_from >= input.len() {
            break;
        }
    }
    replace_ranges(input, &ranges)
}

fn replace_ranges(input: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return input.to_owned();
    }
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    for &(start, end) in ranges {
        if start < cursor || end > input.len() || start > end {
            continue;
        }
        output.push_str(&input[cursor..start]);
        output.push_str("[REDACTED]");
        cursor = end;
    }
    output.push_str(&input[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct FakeLocalSource {
        calls: RefCell<Vec<(LocalGuideKind, String, LocalGuideLocale)>>,
        result: RefCell<Option<Result<Option<LocalGuideSnapshot>, LocalGuideSourceError>>>,
    }

    impl FakeLocalSource {
        fn returning(result: Result<Option<LocalGuideSnapshot>, LocalGuideSourceError>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                result: RefCell::new(Some(result)),
            }
        }

        fn take(
            &self,
            kind: LocalGuideKind,
            term_hex: &LocalGuideTermHex,
            locale: LocalGuideLocale,
        ) -> Result<Option<LocalGuideSnapshot>, LocalGuideSourceError> {
            self.calls
                .borrow_mut()
                .push((kind, term_hex.as_str().into(), locale));
            self.result
                .borrow_mut()
                .take()
                .unwrap_or(Err(LocalGuideSourceError::Unavailable))
        }
    }

    impl LocalGuideDataSource for FakeLocalSource {
        fn quest_rows(
            &self,
            term_hex: &LocalGuideTermHex,
            locale: LocalGuideLocale,
        ) -> Result<Option<LocalGuideSnapshot>, LocalGuideSourceError> {
            self.take(LocalGuideKind::Quest, term_hex, locale)
        }

        fn item_rows(
            &self,
            term_hex: &LocalGuideTermHex,
            locale: LocalGuideLocale,
        ) -> Result<Option<LocalGuideSnapshot>, LocalGuideSourceError> {
            self.take(LocalGuideKind::Item, term_hex, locale)
        }
    }

    fn query(kind: LocalGuideKind) -> LocalGuideQuery {
        LocalGuideQuery::new(kind, "histoire", LocalGuideLocale::FrFr).expect("query")
    }

    fn snapshot(rows: impl Into<String>) -> LocalGuideSnapshot {
        LocalGuideSnapshot {
            rows: rows.into(),
            provenance: LocalGuideProvenance {
                scope: LocalGuideSourceScope::RuntimeSnapshot,
                source_id: "azerothcore-world-local".into(),
                observed_at_unix_ms: Some(1_788_430_000_000),
            },
        }
    }

    fn row(id: u32, title: &str, summary: &str, level: &str, category: &str) -> String {
        format!(
            "{id}\t{}\t{}\t{level}\t{}",
            hex_encode(title.as_bytes()),
            hex_encode(summary.as_bytes()),
            hex_encode(category.as_bytes())
        )
    }

    #[test]
    fn query_validation_fails_closed_and_discards_raw_term() {
        let query = LocalGuideQuery::new(
            LocalGuideKind::Quest,
            "  L’épée   perdue  ",
            LocalGuideLocale::FrFr,
        )
        .expect("query");
        assert!(
            query
                .term_hex
                .as_str()
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        assert_eq!(
            decode_hex_utf8(query.term_hex.as_str(), MAX_SEARCH_TERM_CHARS * 4).expect("valid hex"),
            "L’épée perdue"
        );
        for term in ["x'; DROP TABLE item; --", "bad\nterm", "bad\0term"] {
            assert_eq!(
                LocalGuideQuery::new(LocalGuideKind::Item, term, LocalGuideLocale::EnUs),
                Err(LocalGuideQueryError::UnsupportedCharacter)
            );
        }
        assert_eq!(
            LocalGuideQuery::new(LocalGuideKind::Item, " ", LocalGuideLocale::EnUs),
            Err(LocalGuideQueryError::EmptyTerm)
        );
        assert_eq!(
            LocalGuideQuery::new(LocalGuideKind::Item, "é", LocalGuideLocale::EnUs),
            Err(LocalGuideQueryError::TermTooShort)
        );
        assert_eq!(
            LocalGuideQuery::new(LocalGuideKind::Item, "x".repeat(65), LocalGuideLocale::EnUs),
            Err(LocalGuideQueryError::TermTooLong)
        );
    }

    #[test]
    fn serialized_query_contains_hex_not_the_player_term() {
        let query = LocalGuideQuery::new(
            LocalGuideKind::Item,
            "Pierre de foyer",
            LocalGuideLocale::FrFr,
        )
        .expect("query");
        let serialized = serde_json::to_string(&query).expect("serialize");
        assert!(serialized.contains("termHex"));
        assert!(!serialized.contains("Pierre"));
        assert!(!serialized.contains("foyer"));
    }

    #[test]
    fn deserialization_cannot_bypass_validation_with_controls_or_hex_payloads() {
        for json in [
            r#"{"kind":"quest","term":"bad\nterm","locale":"frFR"}"#,
            r#"{"kind":"item","term":"x","locale":"frFR"}"#,
            r#"{"kind":"item","termHex":"44524F50","locale":"frFR"}"#,
            r#"{"kind":"item","term":"pierre","termHex":"44524F50","locale":"frFR"}"#,
        ] {
            assert!(serde_json::from_str::<LocalGuideQuery>(json).is_err());
        }
        let query = serde_json::from_str::<LocalGuideQuery>(
            r#"{"kind":"item","term":"Pierre de foyer","locale":"frFR"}"#,
        )
        .expect("validated ingress");
        assert_eq!(query.kind, LocalGuideKind::Item);
    }

    #[test]
    fn typed_source_receives_exactly_one_allowlisted_read() {
        let source = FakeLocalSource::returning(Ok(Some(snapshot(row(
            42,
            "Une étrange histoire",
            "Parlez au garde local.",
            "12",
            "Forêt d’Elwynn",
        )))));
        let query = query(LocalGuideKind::Quest);
        let expected_hex = query.term_hex.as_str().to_owned();
        let guide = LocalGuide::new(source);
        let answer = guide.search(&query);
        assert_eq!(answer.entries.len(), 1);
        assert_eq!(answer.entries[0].id, 42);
        assert_eq!(answer.entries[0].metadata.level, Some(12));
        assert_eq!(answer.uncertainty, LocalGuideUncertainty::None);
        assert_eq!(
            guide.source.calls.borrow().as_slice(),
            &[(LocalGuideKind::Quest, expected_hex, LocalGuideLocale::FrFr)]
        );
    }

    #[test]
    fn source_failure_is_silently_unavailable() {
        let query = query(LocalGuideKind::Item);
        for result in [
            Err(LocalGuideSourceError::Unavailable),
            Err(LocalGuideSourceError::InvalidSnapshot),
            Ok(None),
        ] {
            assert_eq!(
                LocalGuide::new(FakeLocalSource::returning(result)).search(&query),
                LocalGuideSearchResult::unavailable()
            );
        }
    }

    #[test]
    fn zero_results_are_valid_empty_not_unavailable() {
        let answer = parse_search_output(&query(LocalGuideKind::Item), "").expect("empty output");
        assert!(answer.entries.is_empty());
        assert!(answer.provenance.is_some());
        assert_eq!(answer.uncertainty, LocalGuideUncertainty::None);
        assert_ne!(answer, LocalGuideSearchResult::unavailable());
    }

    #[test]
    fn parser_preserves_local_provenance_and_unknown_observation_time() {
        let answer = parse_search_output(
            &query(LocalGuideKind::Item),
            &row(
                6948,
                "Pierre de foyer",
                "Vous ramène à votre foyer.",
                "1",
                "Divers",
            ),
        )
        .expect("output");
        assert_eq!(answer.entries[0].title, "Pierre de foyer");
        assert_eq!(
            answer.entries[0].metadata.category.as_deref(),
            Some("Divers")
        );
        assert_eq!(
            answer.entries[0].source.scope,
            LocalGuideSourceScope::RuntimeSnapshot
        );
        assert_eq!(answer.uncertainty, LocalGuideUncertainty::Partial);
        assert!(
            answer
                .provenance
                .expect("provenance")
                .observed_at_unix_ms
                .is_none()
        );
    }

    #[test]
    fn malformed_rows_fail_closed() {
        let query = query(LocalGuideKind::Quest);
        assert_eq!(
            parse_search_output(&query, "1\tnot-hex\t00\t1\t00"),
            Err(LocalGuideParseError::InvalidHex)
        );
        assert_eq!(
            parse_search_output(&query, &row(1, "Titre", "Résumé", "1001", "Zone")),
            Err(LocalGuideParseError::InvalidLevel)
        );
        let duplicate = format!(
            "{}\n{}",
            row(1, "Titre", "Résumé", "1", "Zone"),
            row(1, "Autre", "Résumé", "1", "Zone")
        );
        assert_eq!(
            parse_search_output(&query, &duplicate),
            Err(LocalGuideParseError::DuplicateId)
        );
        let malformed = LocalGuide::new(FakeLocalSource::returning(Ok(Some(snapshot("broken")))));
        assert_eq!(
            malformed.search(&query),
            LocalGuideSearchResult::unavailable()
        );
    }

    #[test]
    fn source_controlled_secrets_are_redacted_from_all_fields() {
        let query = query(LocalGuideKind::Item);
        let mut unsafe_snapshot = snapshot(row(
            1,
            "password=title-secret",
            "Authorization: Bearer abc.def.ghi token=summary-secret",
            "1",
            "mysql://user:category-secret@localhost/world",
        ));
        unsafe_snapshot.provenance.source_id = "snapshot?secret=source-secret".into();
        let answer = parse_search_snapshot(&query, unsafe_snapshot).expect("safe result");
        let rendered = format!("{answer:?}");
        for secret in [
            "title-secret",
            "abc.def.ghi",
            "summary-secret",
            "category-secret",
            "source-secret",
        ] {
            assert!(!rendered.contains(secret), "leaked {secret}");
        }
        assert!(rendered.matches("[REDACTED]").count() >= 5);
    }

    #[test]
    fn quoted_json_credentials_and_basic_authorization_are_redacted() {
        let text = redact_sensitive(
            "{\"api_key\": \"secret with spaces\", \"password\": 'quoted secret'}\nAuthorization: Basic YWJjOmRlZg==",
        );
        for secret in ["secret with spaces", "quoted secret", "YWJjOmRlZg=="] {
            assert!(!text.contains(secret), "leaked {secret}");
        }
        assert!(text.contains("[REDACTED]"));
    }

    #[test]
    fn long_valid_description_is_truncated_not_rejected() {
        let answer = parse_search_snapshot(
            &query(LocalGuideKind::Quest),
            snapshot(row(1, "Longue quête", &"é".repeat(5_000), "80", "Quête")),
        )
        .expect("long valid field within raw budget");
        assert_eq!(answer.entries.len(), 1);
        assert_eq!(answer.entries[0].summary.chars().count(), MAX_SUMMARY_CHARS);
        assert!(answer.entries[0].summary.ends_with('…'));
        assert!(answer.truncated);
        assert_eq!(answer.uncertainty, LocalGuideUncertainty::Partial);
    }

    #[test]
    fn entry_count_field_lengths_and_total_output_are_bounded() {
        let query = query(LocalGuideKind::Item);
        let rows = (1..=20)
            .map(|id| {
                row(
                    id,
                    &"é".repeat(200),
                    &"x".repeat(600),
                    "80",
                    &"c".repeat(200),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let answer = parse_search_snapshot(&query, snapshot(rows)).expect("bounded result");
        assert!(answer.entries.len() <= MAX_ENTRIES);
        assert!(answer.truncated);
        assert_eq!(answer.uncertainty, LocalGuideUncertainty::Partial);
        let total: usize = answer
            .entries
            .iter()
            .map(|entry| {
                entry.title.chars().count()
                    + entry.summary.chars().count()
                    + entry
                        .metadata
                        .category
                        .as_ref()
                        .map_or(0, |category| category.chars().count())
            })
            .sum();
        assert!(total <= MAX_OUTPUT_CHARS);
        for entry in answer.entries {
            assert!(entry.title.chars().count() <= MAX_TITLE_CHARS);
            assert!(entry.summary.chars().count() <= MAX_SUMMARY_CHARS);
            assert!(
                entry
                    .metadata
                    .category
                    .as_ref()
                    .map_or(0, |category| category.chars().count())
                    <= MAX_CATEGORY_CHARS
            );
        }
    }

    #[test]
    fn raw_input_budget_fails_closed() {
        assert_eq!(
            parse_search_output(
                &query(LocalGuideKind::Quest),
                &"x".repeat(MAX_TABULAR_BYTES + 1)
            ),
            Err(LocalGuideParseError::InputTooLarge)
        );
    }

    #[test]
    fn unicode_truncation_does_not_split_characters() {
        let truncated = truncate_chars("Échappée 🐉 fantastique", 10);
        assert_eq!(truncated.chars().count(), 10);
        assert!(truncated.ends_with('…'));
    }
}
