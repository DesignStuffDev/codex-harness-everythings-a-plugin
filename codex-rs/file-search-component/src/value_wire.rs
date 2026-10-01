//! Language-neutral numbers and lossless native values at the process boundary.

use std::fmt;
use std::num::NonZeroUsize;
use std::path::Component;
use std::path::PathBuf;

use codex_file_search_api::FileMatch;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::MatchType;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchFrame;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Visitor;

use crate::contract::invalid;

/// Canonical decimal strings retain every u64 bit in JavaScript and other peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WireU64(pub u64);

impl Serialize for WireU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WireU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Decimal;
        impl Visitor<'_> for Decimal {
            type Value = WireU64;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a canonical decimal u64 string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<WireU64, E> {
                if value.is_empty()
                    || value.len() > 20
                    || (value.len() > 1 && value.starts_with('0'))
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(E::custom("noncanonical file-search integer"));
                }
                value
                    .parse()
                    .map(WireU64)
                    .map_err(|_| E::custom("file-search integer overflow"))
            }
        }
        deserializer.deserialize_str(Decimal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireBudget {
    pub max_index_entries: WireU64,
    pub max_index_bytes: WireU64,
    pub max_worker_threads: WireU64,
}

impl WireBudget {
    pub fn from_native(budget: SearchBudget) -> Self {
        Self {
            max_index_entries: WireU64(budget.max_index_entries.get() as u64),
            max_index_bytes: WireU64(budget.max_index_bytes.get() as u64),
            max_worker_threads: WireU64(budget.max_worker_threads.get() as u64),
        }
    }

    pub fn into_native(self) -> Result<SearchBudget, SearchError> {
        let positive = |value: WireU64| {
            usize::try_from(value.0)
                .ok()
                .and_then(NonZeroUsize::new)
                .ok_or_else(|| invalid("file-search budget is zero or outside the native range"))
        };
        Ok(SearchBudget {
            max_index_entries: positive(self.max_index_entries)?,
            max_index_bytes: positive(self.max_index_bytes)?,
            max_worker_threads: positive(self.max_worker_threads)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireOptions {
    pub limit: u32,
    pub threads: u32,
    pub exclude: Vec<String>,
    pub compute_indices: bool,
    pub respect_gitignore: bool,
}

impl WireOptions {
    pub fn from_native(options: &FileSearchOptions) -> Result<Self, SearchError> {
        Ok(Self {
            limit: options
                .limit
                .get()
                .try_into()
                .map_err(|_| invalid("file-search result limit is outside the wire range"))?,
            threads: options
                .threads
                .get()
                .try_into()
                .map_err(|_| invalid("file-search worker count is outside the wire range"))?,
            exclude: options.exclude.clone(),
            compute_indices: options.compute_indices,
            respect_gitignore: options.respect_gitignore,
        })
    }

    pub fn into_native(self) -> Result<FileSearchOptions, SearchError> {
        let positive = |value: u32| {
            usize::try_from(value)
                .ok()
                .and_then(NonZeroUsize::new)
                .ok_or_else(|| invalid("file-search option is zero or outside the native range"))
        };
        Ok(FileSearchOptions {
            limit: positive(self.limit)?,
            threads: positive(self.threads)?,
            exclude: self.exclude,
            compute_indices: self.compute_indices,
            respect_gitignore: self.respect_gitignore,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireMatchType {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireMatch {
    pub score: u32,
    pub root_index: u32,
    #[serde(with = "codex_component_path_codec::native_path")]
    pub path: PathBuf,
    pub match_type: WireMatchType,
    pub indices: Option<Vec<u32>>,
}

impl WireMatch {
    fn validate(&self) -> Result<(), SearchError> {
        if self
            .path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        {
            return Err(invalid(
                "file-search match path must be relative without parent traversal",
            ));
        }
        if self
            .indices
            .as_ref()
            .is_some_and(|indices| indices.windows(2).any(|pair| pair[0] >= pair[1]))
        {
            return Err(invalid(
                "file-search highlight indices must be sorted and unique",
            ));
        }
        Ok(())
    }

    fn from_native(value: FileMatch, roots: &[PathBuf]) -> Result<Self, SearchError> {
        // Path equality normalizes components; the wire must identify the exact
        // original lexical root, including redundant separators and dot segments.
        let root_index = roots
            .iter()
            .position(|root| root.as_os_str() == value.root.as_os_str())
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| invalid("file-search match does not identify an original root"))?;
        let result = Self {
            score: value.score,
            root_index,
            path: value.path,
            match_type: match value.match_type {
                MatchType::File => WireMatchType::File,
                MatchType::Directory => WireMatchType::Directory,
            },
            indices: value.indices,
        };
        result.validate()?;
        Ok(result)
    }

    fn into_native(self, roots: &[PathBuf]) -> Result<FileMatch, SearchError> {
        self.validate()?;
        let root = usize::try_from(self.root_index)
            .ok()
            .and_then(|index| roots.get(index))
            .ok_or_else(|| invalid("file-search match root index is outside the original roots"))?;
        Ok(FileMatch {
            score: self.score,
            path: self.path,
            root: root.clone(),
            indices: self.indices,
            match_type: match self.match_type {
                WireMatchType::File => MatchType::File,
                WireMatchType::Directory => MatchType::Directory,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireSnapshot {
    pub matches: Vec<WireMatch>,
    pub total_match_count: WireU64,
    pub scanned_file_count: WireU64,
    pub walk_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum WirePhase {
    Running,
    Idle,
    Cancelled,
    Closed,
    Failed { error: SearchError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireFrame {
    pub revision: WireU64,
    pub query_epoch: WireU64,
    pub query: String,
    pub phase: WirePhase,
    pub snapshot: Option<WireSnapshot>,
}

impl WireFrame {
    fn from_native(frame: SearchFrame, roots: &[PathBuf]) -> Result<Self, SearchError> {
        frame.validate()?;
        let snapshot = frame
            .snapshot
            .map(|snapshot| -> Result<WireSnapshot, SearchError> {
                Ok(WireSnapshot {
                    matches: snapshot
                        .matches
                        .into_iter()
                        .map(|value| WireMatch::from_native(value, roots))
                        .collect::<Result<_, SearchError>>()?,
                    total_match_count: WireU64(snapshot.total_match_count as u64),
                    scanned_file_count: WireU64(snapshot.scanned_file_count as u64),
                    walk_complete: snapshot.walk_complete,
                })
            })
            .transpose()?;
        Ok(Self {
            revision: WireU64(frame.revision),
            query_epoch: WireU64(frame.query_id),
            query: frame.query,
            snapshot,
            phase: match frame.phase {
                SearchPhase::Running => WirePhase::Running,
                SearchPhase::Idle => WirePhase::Idle,
                SearchPhase::Cancelled => WirePhase::Cancelled,
                SearchPhase::Closed => WirePhase::Closed,
                SearchPhase::Failed(error) => WirePhase::Failed { error },
            },
        })
    }

    fn into_native(self, roots: &[PathBuf]) -> Result<SearchFrame, SearchError> {
        let snapshot =
            self.snapshot
                .map(|snapshot| -> Result<FileSearchSnapshot, SearchError> {
                    Ok(FileSearchSnapshot {
                        query_id: self.query_epoch.0,
                        query: self.query.clone(),
                        matches: snapshot
                            .matches
                            .into_iter()
                            .map(|value| value.into_native(roots))
                            .collect::<Result<_, SearchError>>()?,
                        total_match_count: snapshot.total_match_count.0.try_into().map_err(
                            |_| invalid("file-search match count exceeds the native range"),
                        )?,
                        scanned_file_count: snapshot.scanned_file_count.0.try_into().map_err(
                            |_| invalid("file-search scanned count exceeds the native range"),
                        )?,
                        walk_complete: snapshot.walk_complete,
                    })
                })
                .transpose()?;
        let result = SearchFrame {
            revision: self.revision.0,
            query_id: self.query_epoch.0,
            query: self.query,
            snapshot,
            phase: match self.phase {
                WirePhase::Running => SearchPhase::Running,
                WirePhase::Idle => SearchPhase::Idle,
                WirePhase::Cancelled => SearchPhase::Cancelled,
                WirePhase::Closed => SearchPhase::Closed,
                WirePhase::Failed { error } => SearchPhase::Failed(error),
            },
        };
        result.validate()?;
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum WirePoll {
    Changed { frame: WireFrame },
    Unchanged { revision: WireU64 },
}

impl WirePoll {
    pub fn from_native(poll: SearchPoll, roots: &[PathBuf]) -> Result<Self, SearchError> {
        match poll {
            SearchPoll::Changed(frame) => Ok(Self::Changed {
                frame: WireFrame::from_native(frame, roots)?,
            }),
            SearchPoll::Unchanged { revision } => Ok(Self::Unchanged {
                revision: WireU64(revision),
            }),
        }
    }

    pub fn into_native(self, roots: &[PathBuf]) -> Result<SearchPoll, SearchError> {
        match self {
            Self::Changed { frame } => Ok(SearchPoll::Changed(frame.into_native(roots)?)),
            Self::Unchanged { revision } => Ok(SearchPoll::Unchanged {
                revision: revision.0,
            }),
        }
    }
}

#[cfg(test)]
#[path = "value_wire_tests.rs"]
mod tests;
