use serde::Deserialize;
use serde::Serialize;

/// Base tool availability selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tools {
    List(Vec<String>),
    Preset { r#type: String, preset: String },
}

impl Tools {
    pub fn claude_code_preset() -> Self {
        Self::Preset {
            r#type: "preset".to_owned(),
            preset: "claude_code".to_owned(),
        }
    }
}

/// Skill selection for the main session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Skills {
    All(String),
    List(Vec<String>),
}

impl Skills {
    pub fn all() -> Self {
        Self::All("all".to_owned())
    }
}

/// Per-tool built-in tool behavior configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_user_question: Option<AskUserQuestionToolConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserQuestionToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_format: Option<QuestionPreviewFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionPreviewFormat {
    Markdown,
    Html,
}

impl QuestionPreviewFormat {
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
        }
    }
}
