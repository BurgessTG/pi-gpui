#![allow(dead_code)]

use std::collections::BTreeMap;

use super::canvas_session::SessionNodePrimitive;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasNodeRuntime {
    None,
    PiSession,
    WorkerProcess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasNodeRenderMode {
    SceneOnly,
    GpuiIsland,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasNodeDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub status_label: &'static str,
    pub minimap_symbol: &'static str,
    pub primitive: Option<SessionNodePrimitive>,
    pub runtime: CanvasNodeRuntime,
    pub render_mode: CanvasNodeRenderMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasNodeManifest {
    pub id: String,
    pub label: String,
    pub package: Option<String>,
    pub runtime: CanvasNodeRuntime,
    pub render_mode: CanvasNodeRenderMode,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanvasNodeRegistry {
    manifests: BTreeMap<String, CanvasNodeManifest>,
}

impl CanvasNodeRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        for definition in BUILTIN_SESSION_NODE_DEFINITIONS {
            let manifest = CanvasNodeManifest {
                id: definition.id.to_owned(),
                label: definition.label.to_owned(),
                package: None,
                runtime: definition.runtime,
                render_mode: definition.render_mode,
            };
            let _inserted = registry.register(manifest);
        }
        registry
    }

    pub fn register(&mut self, manifest: CanvasNodeManifest) -> bool {
        if manifest.id.trim().is_empty() || manifest.label.trim().is_empty() {
            return false;
        }
        self.manifests
            .insert(manifest.id.clone(), manifest)
            .is_none()
    }

    pub fn get(&self, id: &str) -> Option<&CanvasNodeManifest> {
        self.manifests.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CanvasNodeManifest> {
        self.manifests.values()
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}

pub const BUILTIN_SESSION_NODE_DEFINITIONS: [CanvasNodeDefinition; 3] = [
    CanvasNodeDefinition {
        id: "pi.session.new",
        label: "New session",
        status_label: "Pi NewSession",
        minimap_symbol: "●",
        primitive: Some(SessionNodePrimitive::NewSession),
        runtime: CanvasNodeRuntime::PiSession,
        render_mode: CanvasNodeRenderMode::GpuiIsland,
    },
    CanvasNodeDefinition {
        id: "pi.session.fork",
        label: "Fork session",
        status_label: "Pi Fork",
        minimap_symbol: "◆",
        primitive: Some(SessionNodePrimitive::ForkSession),
        runtime: CanvasNodeRuntime::PiSession,
        render_mode: CanvasNodeRenderMode::GpuiIsland,
    },
    CanvasNodeDefinition {
        id: "pi.session.resume",
        label: "Resume session",
        status_label: "Pi SwitchSession",
        minimap_symbol: "■",
        primitive: Some(SessionNodePrimitive::ResumeSession),
        runtime: CanvasNodeRuntime::PiSession,
        render_mode: CanvasNodeRenderMode::GpuiIsland,
    },
];

pub fn session_node_definition(primitive: SessionNodePrimitive) -> &'static CanvasNodeDefinition {
    BUILTIN_SESSION_NODE_DEFINITIONS
        .iter()
        .find(|definition| definition.primitive == Some(primitive))
        .unwrap_or(&BUILTIN_SESSION_NODE_DEFINITIONS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_starts_with_builtin_session_nodes() {
        let registry = CanvasNodeRegistry::with_builtins();
        assert_eq!(registry.len(), 3);
        assert!(registry.get("pi.session.new").is_some());
        assert!(registry.get("pi.session.fork").is_some());
        assert!(registry.get("pi.session.resume").is_some());
    }

    #[test]
    fn registry_rejects_empty_or_duplicate_manifests() {
        let mut registry = CanvasNodeRegistry::default();
        assert!(!registry.register(CanvasNodeManifest {
            id: String::new(),
            label: "Bad".to_owned(),
            package: None,
            runtime: CanvasNodeRuntime::WorkerProcess,
            render_mode: CanvasNodeRenderMode::GpuiIsland,
        }));
        assert!(registry.register(CanvasNodeManifest {
            id: "package.node".to_owned(),
            label: "Package node".to_owned(),
            package: Some("package".to_owned()),
            runtime: CanvasNodeRuntime::WorkerProcess,
            render_mode: CanvasNodeRenderMode::GpuiIsland,
        }));
        assert!(!registry.register(CanvasNodeManifest {
            id: "package.node".to_owned(),
            label: "Other package node".to_owned(),
            package: Some("package".to_owned()),
            runtime: CanvasNodeRuntime::WorkerProcess,
            render_mode: CanvasNodeRenderMode::SceneOnly,
        }));
    }
}
