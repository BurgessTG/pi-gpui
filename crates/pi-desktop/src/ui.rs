use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pi_bridge_types::ProviderAuthStatus;

pub fn logo_path() -> String {
    workspace_root()
        .join("assets/pi-logo-auto.svg")
        .display()
        .to_string()
}

pub fn pin_icon_path() -> String {
    workspace_root()
        .join("assets/icons/lucide-pin.svg")
        .display()
        .to_string()
}

pub fn folder_plus_icon_path() -> String {
    workspace_root()
        .join("assets/icons/lucide-folder-plus.svg")
        .display()
        .to_string()
}

#[allow(dead_code)]
pub fn drawing_tool_icon_path(name: &str) -> String {
    workspace_root()
        .join("assets/icons")
        .join(format!("lucide-{name}.svg"))
        .display()
        .to_string()
}

pub fn provider_logo_path(provider: &str) -> Option<String> {
    let file = provider_logo_file(provider)?;
    let path = provider_logo_dir().join(file);
    path.exists().then(|| path.display().to_string())
}

fn provider_logo_dir() -> &'static Path {
    static LOGO_DIR: OnceLock<PathBuf> = OnceLock::new();
    LOGO_DIR
        .get_or_init(|| workspace_root().join("assets/provider-logos"))
        .as_path()
}

pub const BUILTIN_PROVIDER_SLUGS: &[&str] = &[
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

pub fn builtin_provider_auth_statuses() -> Vec<ProviderAuthStatus> {
    BUILTIN_PROVIDER_SLUGS
        .iter()
        .map(|provider| ProviderAuthStatus {
            provider: (*provider).to_owned(),
            display_name: provider_display_name(provider),
            configured: false,
            source: None,
            label: None,
        })
        .collect()
}

fn provider_display_name(provider: &str) -> String {
    match provider {
        "amazon-bedrock" => "Amazon Bedrock",
        "anthropic" => "Anthropic",
        "azure-openai-responses" => "Azure OpenAI",
        "cerebras" => "Cerebras",
        "cloudflare-ai-gateway" => "Cloudflare AI Gateway",
        "cloudflare-workers-ai" => "Cloudflare Workers AI",
        "deepseek" => "DeepSeek",
        "fireworks" => "Fireworks",
        "github-copilot" => "GitHub Copilot",
        "google" => "Google Gemini",
        "google-vertex" => "Google Vertex AI",
        "groq" => "Groq",
        "huggingface" => "Hugging Face",
        "kimi-coding" => "Kimi Coding",
        "minimax" => "MiniMax",
        "minimax-cn" => "MiniMax CN",
        "mistral" => "Mistral",
        "moonshotai" => "Moonshot AI",
        "moonshotai-cn" => "Moonshot AI CN",
        "openai" => "OpenAI",
        "openai-codex" => "OpenAI Codex",
        "opencode" => "OpenCode",
        "opencode-go" => "OpenCode Go",
        "openrouter" => "OpenRouter",
        "together" => "Together AI",
        "vercel-ai-gateway" => "Vercel AI Gateway",
        "xai" => "xAI",
        "xiaomi" => "Xiaomi",
        "xiaomi-token-plan-ams" => "Xiaomi Token Plan AMS",
        "xiaomi-token-plan-cn" => "Xiaomi Token Plan CN",
        "xiaomi-token-plan-sgp" => "Xiaomi Token Plan SGP",
        "zai" => "Z.ai",
        _ => provider,
    }
    .to_owned()
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
