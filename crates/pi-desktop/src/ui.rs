use std::path::{Path, PathBuf};

use pi_bridge_types::AuthSource;

pub fn logo_path() -> String {
    workspace_root()
        .join("assets/pi-logo-auto.svg")
        .display()
        .to_string()
}

pub fn auth_source_label(source: Option<AuthSource>) -> &'static str {
    match source {
        Some(AuthSource::Stored) => "stored in auth.json",
        Some(AuthSource::Runtime) => "active for this run",
        Some(AuthSource::Environment) => "environment variable",
        Some(AuthSource::Fallback) => "fallback resolver",
        Some(AuthSource::ModelsJsonKey) => "models.json key",
        Some(AuthSource::ModelsJsonCommand) => "models.json command",
        None => "not configured",
    }
}

pub fn provider_env_hint(provider: &str) -> String {
    format!(
        "{} API key or existing Pi auth",
        provider.to_uppercase().replace('-', "_")
    )
}

pub fn provider_logo_path(provider: &str) -> Option<String> {
    let file = provider_logo_file(provider)?;
    let path = workspace_root().join("assets/provider-logos").join(file);
    path.exists().then(|| path.display().to_string())
}

fn provider_logo_file(provider: &str) -> Option<&'static str> {
    match provider {
        "amazon-bedrock" => Some("amazon-bedrock.svg"),
        "anthropic" => Some("anthropic.svg"),
        "azure-openai-responses" => Some("azure-openai.svg"),
        "cerebras" => Some("cerebras.svg"),
        "cloudflare-ai-gateway" => Some("cloudflare.svg"),
        "cloudflare-workers-ai" => Some("cloudflare-workers.svg"),
        "deepseek" => Some("deepseek.svg"),
        "fireworks" => Some("fireworks.svg"),
        "github-copilot" => Some("github-copilot.svg"),
        "google" => Some("google-gemini.svg"),
        "google-vertex" => Some("google-vertex.svg"),
        "groq" => Some("groq.svg"),
        "huggingface" => Some("huggingface.svg"),
        "kimi-coding" => Some("kimi.svg"),
        "minimax" | "minimax-cn" => Some("minimax.svg"),
        "mistral" => Some("mistral.svg"),
        "moonshotai" | "moonshotai-cn" => Some("moonshot.svg"),
        "openai" | "openai-codex" => Some("openai.svg"),
        "opencode" | "opencode-go" => Some("opencode.svg"),
        "openrouter" => Some("openrouter.svg"),
        "together" => Some("together.svg"),
        "vercel-ai-gateway" => Some("vercel.svg"),
        "xai" => Some("xai.svg"),
        "xiaomi" | "xiaomi-token-plan-ams" | "xiaomi-token-plan-cn" | "xiaomi-token-plan-sgp" => {
            Some("xiaomi.svg")
        }
        "zai" => Some("zai.svg"),
        _ => None,
    }
}

pub fn provider_initials(display_name: &str) -> String {
    display_name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}

#[cfg(test)]
mod tests {
    use super::{provider_logo_file, provider_logo_path};

    const BUILTIN_PROVIDER_SLUGS: &[&str] = &[
        "amazon-bedrock",
        "anthropic",
        "azure-openai-responses",
        "cerebras",
        "cloudflare-ai-gateway",
        "cloudflare-workers-ai",
        "deepseek",
        "fireworks",
        "github-copilot",
        "google",
        "google-vertex",
        "groq",
        "huggingface",
        "kimi-coding",
        "minimax",
        "minimax-cn",
        "mistral",
        "moonshotai",
        "moonshotai-cn",
        "openai",
        "openai-codex",
        "opencode",
        "opencode-go",
        "openrouter",
        "together",
        "vercel-ai-gateway",
        "xai",
        "xiaomi",
        "xiaomi-token-plan-ams",
        "xiaomi-token-plan-cn",
        "xiaomi-token-plan-sgp",
        "zai",
    ];

    #[test]
    fn built_in_providers_have_logo_mappings() {
        for provider in BUILTIN_PROVIDER_SLUGS {
            assert!(
                provider_logo_file(provider).is_some(),
                "missing logo mapping for {provider}"
            );
        }
    }

    #[test]
    fn built_in_provider_logo_files_exist() {
        for provider in BUILTIN_PROVIDER_SLUGS {
            assert!(
                provider_logo_path(provider).is_some(),
                "missing logo asset for {provider}"
            );
        }
    }
}
