//! Native path-bearing event leaves, separate from provider/public JSON.

use codex_protocol::models::ImageDetail;
use codex_protocol::models::ImageReference;
use codex_protocol::user_input::TextElement;
use codex_protocol::user_input::UserInput;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::parse_command::ParsedCommand")]
pub(crate) enum ParsedCommandWire {
    Read {
        cmd: String,
        name: String,
        #[serde(with = "super::paths::native")]
        path: PathBuf,
    },
    ListFiles {
        cmd: String,
        path: Option<String>,
    },
    Search {
        cmd: String,
        query: Option<String>,
        path: Option<String>,
    },
    Unknown {
        cmd: String,
    },
}

remote_adapter!(
    parsed_command,
    codex_protocol::parse_command::ParsedCommand,
    ParsedCommandWire,
    "ParsedCommandWire"
);

// UserInput is non-exhaustive. Explicit conversion fails closed if an added
// native variant has not yet acquired a lossless component representation.
#[derive(serde::Serialize, serde::Deserialize)]
enum UserInputWire {
    Text {
        text: String,
        text_elements: Vec<TextElement>,
    },
    Image {
        image: ImageReference,
        detail: Option<ImageDetail>,
    },
    LocalImage {
        #[serde(with = "super::paths::native")]
        path: PathBuf,
        detail: Option<ImageDetail>,
    },
    Audio {
        audio_url: String,
    },
    LocalAudio {
        #[serde(with = "super::paths::native")]
        path: PathBuf,
    },
    Skill {
        name: String,
        #[serde(with = "super::paths::native")]
        path: PathBuf,
    },
    Mention {
        name: String,
        path: String,
    },
}

impl TryFrom<&UserInput> for UserInputWire {
    type Error = &'static str;

    fn try_from(value: &UserInput) -> Result<Self, Self::Error> {
        Ok(match value {
            UserInput::Text {
                text,
                text_elements,
            } => Self::Text {
                text: text.clone(),
                text_elements: text_elements.clone(),
            },
            UserInput::Image { image, detail } => Self::Image {
                image: image.clone(),
                detail: *detail,
            },
            UserInput::LocalImage { path, detail } => Self::LocalImage {
                path: path.clone(),
                detail: *detail,
            },
            UserInput::Audio { audio_url } => Self::Audio {
                audio_url: audio_url.clone(),
            },
            UserInput::LocalAudio { path } => Self::LocalAudio { path: path.clone() },
            UserInput::Skill { name, path } => Self::Skill {
                name: name.clone(),
                path: path.clone(),
            },
            UserInput::Mention { name, path } => Self::Mention {
                name: name.clone(),
                path: path.clone(),
            },
            _ => return Err("unsupported trusted user input variant"),
        })
    }
}

impl From<UserInputWire> for UserInput {
    fn from(value: UserInputWire) -> Self {
        match value {
            UserInputWire::Text {
                text,
                text_elements,
            } => Self::Text {
                text,
                text_elements,
            },
            UserInputWire::Image { image, detail } => Self::Image { image, detail },
            UserInputWire::LocalImage { path, detail } => Self::LocalImage { path, detail },
            UserInputWire::Audio { audio_url } => Self::Audio { audio_url },
            UserInputWire::LocalAudio { path } => Self::LocalAudio { path },
            UserInputWire::Skill { name, path } => Self::Skill { name, path },
            UserInputWire::Mention { name, path } => Self::Mention { name, path },
        }
    }
}

mod user_inputs {
    pub(crate) fn serialize<S: serde::Serializer>(
        values: &[super::UserInput],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let owned = values
            .iter()
            .map(super::UserInputWire::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::ser::Error::custom)?;
        serde::Serialize::serialize(&owned, serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<super::UserInput>, D::Error> {
        let owned = <Vec<super::UserInputWire> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(owned.into_iter().map(Into::into).collect())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::UserMessageItem")]
pub(crate) struct UserMessageItemWire {
    id: String,
    client_id: Option<String>,
    #[serde(with = "user_inputs")]
    content: Vec<UserInput>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::UserMessageEvent")]
pub(crate) struct UserMessageEventWire {
    client_id: Option<String>,
    message: String,
    images: Option<Vec<String>>,
    image_details: Vec<Option<ImageDetail>>,
    file_ids: Option<Vec<String>>,
    file_id_details: Vec<Option<ImageDetail>>,
    image_order: Vec<codex_protocol::protocol::UserMessageImageKind>,
    #[serde(with = "super::paths::native::vec")]
    local_images: Vec<PathBuf>,
    local_image_details: Vec<Option<ImageDetail>>,
    audio: Option<Vec<String>>,
    #[serde(with = "super::paths::native::vec")]
    local_audio: Vec<PathBuf>,
    text_elements: Vec<TextElement>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ImageGenerationEndEvent")]
pub(crate) struct ImageGenerationEndEventWire {
    call_id: String,
    status: String,
    revised_prompt: Option<String>,
    result: String,
    transparent_background: Option<bool>,
    failure: Option<codex_extension_items::image_generation::ImageGenerationFailure>,
    #[serde(with = "super::paths::absolute::option")]
    saved_path: Option<codex_utils_absolute_path::AbsolutePathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::ImageGenerationItem")]
pub(crate) struct ImageGenerationItemWire {
    id: String,
    status: String,
    revised_prompt: Option<String>,
    result: String,
    #[serde(with = "super::paths::absolute::option")]
    saved_path: Option<codex_utils_absolute_path::AbsolutePathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ExecCommandBeginEvent")]
pub(crate) struct ExecCommandBeginEventWire {
    call_id: String,
    plugin_id: Option<String>,
    script_path: Option<String>,
    process_id: Option<String>,
    turn_id: String,
    started_at_ms: i64,
    command: Vec<String>,
    cwd: codex_utils_path_uri::PathUri,
    #[serde(with = "parsed_command::vec")]
    parsed_cmd: Vec<codex_protocol::parse_command::ParsedCommand>,
    source: codex_protocol::protocol::ExecCommandSource,
    interaction_input: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ExecCommandEndEvent")]
pub(crate) struct ExecCommandEndEventWire {
    call_id: String,
    plugin_id: Option<String>,
    script_path: Option<String>,
    process_id: Option<String>,
    turn_id: String,
    completed_at_ms: i64,
    command: Vec<String>,
    cwd: codex_utils_path_uri::PathUri,
    #[serde(with = "parsed_command::vec")]
    parsed_cmd: Vec<codex_protocol::parse_command::ParsedCommand>,
    source: codex_protocol::protocol::ExecCommandSource,
    interaction_input: Option<String>,
    stdout: String,
    stderr: String,
    aggregated_output: String,
    exit_code: i32,
    duration: std::time::Duration,
    formatted_output: String,
    status: codex_protocol::protocol::ExecCommandStatus,
}
