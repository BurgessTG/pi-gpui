use super::canvas_model::{WorldPoint, WorldSize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum SessionNodePrimitive {
    NewSession,
    ForkSession,
    ResumeSession,
}

impl SessionNodePrimitive {
    pub fn label(self) -> &'static str {
        match self {
            Self::NewSession => "New session",
            Self::ForkSession => "Fork session",
            Self::ResumeSession => "Resume session",
        }
    }

    pub fn status_label(self) -> &'static str {
        match self {
            Self::NewSession => "Pi NewSession",
            Self::ForkSession => "Pi Fork",
            Self::ResumeSession => "Pi SwitchSession",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionNodeMetadata {
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub session_file: Option<String>,
    pub cwd: Option<String>,
    pub message_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionNode {
    pub(super) id: usize,
    pub(super) primitive: SessionNodePrimitive,
    pub(super) position: WorldPoint,
    pub(super) size: WorldSize,
    pub(super) metadata: SessionNodeMetadata,
}

impl SessionNode {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn primitive(&self) -> SessionNodePrimitive {
        self.primitive
    }

    pub fn position(&self) -> WorldPoint {
        self.position
    }

    pub fn size(&self) -> WorldSize {
        self.size
    }

    pub fn title(&self) -> String {
        self.metadata
            .session_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Chat Session".to_owned())
    }

    pub fn metadata(&self) -> &SessionNodeMetadata {
        &self.metadata
    }
}
