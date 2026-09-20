#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginProviderAuthKind {
    OAuth,
    ApiKey,
    DeviceCode,
    Cli,
    Hybrid,
    Local,
}

impl LoginProviderAuthKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::OAuth => "OAuth",
            Self::ApiKey => "API key",
            Self::DeviceCode => "device code",
            Self::Cli => "CLI",
            Self::Hybrid => "API key / CLI",
            Self::Local => "local endpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginProviderTarget {
    AutoImport,
    Jcode,
    Claude,
    ClaudeApiKey,
    OpenAi,
    OpenAiApiKey,
    OpenRouter,
    Bedrock,
    Azure,
    OpenAiCompatible(OpenAiCompatibleProfile),
    Cursor,
    GrokBuild,
    Copilot,
    Gemini,
    Antigravity,
    Google,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginProviderAuthStateKey {
    ExternalImport,
    Jcode,
    Anthropic,
    OpenAi,
    Azure,
    Bedrock,
    OpenRouterLike,
    Copilot,
    Gemini,
    Antigravity,
    Cursor,
    GrokBuild,
    Google,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginProviderSurface {
    CliLogin,
    TuiLogin,
    ServerBootstrap,
    AutoInit,
    AuthStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoginProviderSurfaceOrder {
    pub cli_login: Option<u8>,
    pub tui_login: Option<u8>,
    pub server_bootstrap: Option<u8>,
    pub auto_init: Option<u8>,
    pub auth_status: Option<u8>,
}

impl LoginProviderSurfaceOrder {
    pub const fn new(
        cli_login: Option<u8>,
        tui_login: Option<u8>,
        server_bootstrap: Option<u8>,
        auto_init: Option<u8>,
        auth_status: Option<u8>,
    ) -> Self {
        Self {
            cli_login,
            tui_login,
            server_bootstrap,
            auto_init,
            auth_status,
        }
    }

    pub const fn for_surface(self, surface: LoginProviderSurface) -> Option<u8> {
        match surface {
            LoginProviderSurface::CliLogin => self.cli_login,
            LoginProviderSurface::TuiLogin => self.tui_login,
            LoginProviderSurface::ServerBootstrap => self.server_bootstrap,
            LoginProviderSurface::AutoInit => self.auto_init,
            LoginProviderSurface::AuthStatus => self.auth_status,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoginProviderDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub auth_kind: LoginProviderAuthKind,
    pub auth_state_key: LoginProviderAuthStateKey,
    pub auth_status_method: &'static str,
    pub aliases: &'static [&'static str],
    pub menu_detail: &'static str,
    pub recommended: bool,
    pub target: LoginProviderTarget,
    pub order: LoginProviderSurfaceOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenAiCompatibleProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub api_base: &'static str,
    pub api_key_env: &'static str,
    pub env_file: &'static str,
    pub setup_url: &'static str,
    pub default_model: Option<&'static str>,
    pub requires_api_key: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedOpenAiCompatibleProfile {
    pub id: String,
    pub display_name: String,
    pub api_base: String,
    pub api_key_env: String,
    pub env_file: String,
    pub setup_url: String,
    pub default_model: Option<String>,
    pub requires_api_key: bool,
}

mod catalog;

pub use catalog::*;
use catalog::{LOGIN_PROVIDERS, OPENAI_COMPAT_PROFILES};

pub fn openai_compatible_profiles() -> &'static [OpenAiCompatibleProfile] {
    &OPENAI_COMPAT_PROFILES
}

pub fn login_providers() -> &'static [LoginProviderDescriptor] {
    &LOGIN_PROVIDERS
}

fn login_providers_for_surface(surface: LoginProviderSurface) -> Vec<LoginProviderDescriptor> {
    let mut providers = login_providers()
        .iter()
        .copied()
        .filter(|provider| provider.order.for_surface(surface).is_some())
        .collect::<Vec<_>>();
    providers.sort_by_key(|provider| provider.order.for_surface(surface).unwrap_or(u8::MAX));
    providers
}

pub fn cli_login_providers() -> Vec<LoginProviderDescriptor> {
    login_providers_for_surface(LoginProviderSurface::CliLogin)
}

pub fn tui_login_providers() -> Vec<LoginProviderDescriptor> {
    login_providers_for_surface(LoginProviderSurface::TuiLogin)
}

pub fn server_bootstrap_login_providers() -> Vec<LoginProviderDescriptor> {
    login_providers_for_surface(LoginProviderSurface::ServerBootstrap)
}

pub fn auto_init_login_providers() -> Vec<LoginProviderDescriptor> {
    login_providers_for_surface(LoginProviderSurface::AutoInit)
}

pub fn auth_status_login_providers() -> Vec<LoginProviderDescriptor> {
    login_providers_for_surface(LoginProviderSurface::AuthStatus)
}

pub fn resolve_login_provider(input: &str) -> Option<LoginProviderDescriptor> {
    let normalized = normalize_provider_input(input)?;
    login_providers().iter().copied().find(|provider| {
        provider.id == normalized || provider.aliases.iter().any(|alias| *alias == normalized)
    })
}

/// Resolve a login provider by id, alias, or display name.
///
/// Login completion events carry the human-readable provider label (e.g.
/// "Anthropic API") rather than the canonical id/alias, so the stricter
/// [`resolve_login_provider`] (id/alias only) misses them. Auth-change routing
/// needs to map those labels back to a provider id; matching the display name
/// here keeps the post-login model refresh attributed to the correct provider.
pub fn resolve_login_provider_loose(input: &str) -> Option<LoginProviderDescriptor> {
    if let Some(provider) = resolve_login_provider(input) {
        return Some(provider);
    }
    let normalized = normalize_provider_input(input)?;
    login_providers()
        .iter()
        .copied()
        .find(|provider| provider.display_name.to_ascii_lowercase() == normalized)
}

pub fn resolve_login_selection(
    input: &str,
    providers: &[LoginProviderDescriptor],
) -> Option<LoginProviderDescriptor> {
    let trimmed = input.trim();
    if let Ok(index) = trimmed.parse::<usize>() {
        return index
            .checked_sub(1)
            .and_then(|idx| providers.get(idx))
            .copied();
    }

    let provider = resolve_login_provider(trimmed)?;
    providers
        .iter()
        .copied()
        .find(|candidate| candidate.id == provider.id)
}

pub fn is_safe_env_key_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_safe_env_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Comma-separated opt-in allowlist that lets plain `http://` be used with
/// hosts jcode rejects by default (public hostnames and public IPs).
///
/// Entries are matched case-insensitively against the URL host:
///   * `example.com`      exact host
///   * `*.example.com`    any subdomain (a leading `.` also works)
///   * `example.com:8000` exact host, port must match too
///   * `*`                allow every `http://` host
///
/// Default behavior is unchanged when the variable is unset or empty.
pub const ALLOW_INSECURE_HTTP_HOSTS_ENV: &str = "JCODE_ALLOW_INSECURE_HTTP_HOSTS";

/// Resolvers that read a `JCODE_*` setting from jcode's config env files.
///
/// This leaf crate cannot see the config directory, so a higher-level crate
/// registers one at startup. That keeps settings resolved here (such as the
/// plain-HTTP host allowlist) consistent with the rest of the provider config,
/// which is also settable from the provider env file.
type EnvFileValueResolver = fn(&str) -> Option<String>;

static ENV_FILE_VALUE_RESOLVERS: std::sync::LazyLock<std::sync::RwLock<Vec<EnvFileValueResolver>>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(Vec::new()));

/// Register a fallback that reads a setting from jcode's config env files.
pub fn register_env_file_value_resolver(resolver: EnvFileValueResolver) {
    ENV_FILE_VALUE_RESOLVERS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(resolver);
}

fn resolve_env_file_value(name: &str) -> Option<String> {
    let resolvers = ENV_FILE_VALUE_RESOLVERS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    resolvers.iter().find_map(|resolver| resolver(name))
}

/// Process environment first, then registered config env files.
fn env_or_config_value(name: &str) -> Option<String> {
    if let Ok(value) = std::env::var(name) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    resolve_env_file_value(name).filter(|value| !value.trim().is_empty())
}

pub fn normalize_api_base(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = url::Url::parse(trimmed).ok()?;
    let scheme = parsed.scheme();
    if scheme != "https" && scheme != "http" {
        return None;
    }

    if scheme == "http" {
        let host = parsed.host_str()?;
        let port = parsed.port_or_known_default();
        if !allows_insecure_http_host(host, port) {
            return None;
        }
    }

    Some(trimmed.trim_end_matches('/').to_string())
}

/// Whether a plain `http://` host is allowed, honoring
/// [`ALLOW_INSECURE_HTTP_HOSTS_ENV`].
pub fn allows_insecure_http_host(host: &str, port: Option<u16>) -> bool {
    let extra = env_or_config_value(ALLOW_INSECURE_HTTP_HOSTS_ENV).unwrap_or_default();
    allows_insecure_http_host_with(host, port, &extra)
}

pub fn allows_insecure_http_host_with(host: &str, port: Option<u16>, extra_hosts: &str) -> bool {
    if matches_allowlist(host, port, extra_hosts) {
        return true;
    }

    let host = host.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host);
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost" || host_lower.ends_with(".local") {
        return true;
    }

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                let raw = u32::from(v4);
                let is_carrier_grade_nat = (raw & 0xffc0_0000) == 0x6440_0000;
                v4.is_loopback()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.is_unspecified()
                    || is_carrier_grade_nat
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_loopback()
                    || v6.is_unique_local()
                    || v6.is_unicast_link_local()
                    || v6.is_unspecified()
            }
        };
    }

    false
}

fn matches_allowlist(host: &str, port: Option<u16>, extra_hosts: &str) -> bool {
    let host = strip_brackets(host.trim()).to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }

    extra_hosts.split(',').any(|entry| {
        let entry = entry.trim().to_ascii_lowercase();
        if entry.is_empty() {
            return false;
        }
        if entry == "*" {
            return true;
        }

        let (entry_host, entry_port) = match entry.rsplit_once(':') {
            // Keep IPv6 literals like `[fd00::1]` intact.
            Some((head, tail)) if !tail.is_empty() => match tail.parse::<u16>() {
                Ok(parsed_port) => (head.to_string(), Some(parsed_port)),
                Err(_) => (entry.clone(), None),
            },
            _ => (entry.clone(), None),
        };
        if entry_port.is_some() && entry_port != port {
            return false;
        }

        let entry_host = strip_brackets(&entry_host);
        if entry_host == "*" {
            return true;
        }
        if let Some(suffix) = entry_host.strip_prefix("*.").or_else(|| {
            entry_host
                .starts_with('.')
                .then(|| entry_host.trim_start_matches('.'))
        }) {
            return !suffix.is_empty() && host.ends_with(&format!(".{suffix}"));
        }
        entry_host == host
    })
}

fn strip_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host)
}

fn normalize_provider_input(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Serializes tests that mutate process-wide environment variables.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn env_lock() -> &'static std::sync::Mutex<()> {
        &ENV_LOCK
    }

    mod test_env {
        pub fn var(name: &str) -> Option<String> {
            std::env::var(name).ok()
        }

        pub fn set_var(name: &str, value: &str) {
            // SAFETY: callers hold ENV_LOCK, so no other test thread reads or
            // writes the environment concurrently.
            unsafe { std::env::set_var(name, value) }
        }

        pub fn remove_var(name: &str) {
            // SAFETY: see `set_var`.
            unsafe { std::env::remove_var(name) }
        }
    }

    #[test]
    fn matrix_profiles_have_unique_ids_and_safe_metadata() {
        let mut ids = HashSet::new();
        for profile in openai_compatible_profiles() {
            assert!(
                ids.insert(profile.id),
                "duplicate provider profile id: {}",
                profile.id
            );
            assert!(is_safe_env_key_name(profile.api_key_env));
            assert!(is_safe_env_file_name(profile.env_file));
            assert_eq!(
                normalize_api_base(profile.api_base).as_deref(),
                Some(profile.api_base)
            );
        }
    }

    #[test]
    fn zai_login_identifies_coding_plan_subscription_key() {
        let provider = resolve_login_provider("zai").expect("Z.AI provider");
        assert_eq!(provider.auth_kind, LoginProviderAuthKind::ApiKey);
        assert_eq!(provider.menu_detail, "Coding Plan subscription API key");

        let LoginProviderTarget::OpenAiCompatible(profile) = provider.target else {
            panic!("Z.AI must use its OpenAI-compatible Coding Plan endpoint");
        };
        assert_eq!(profile.api_base, "https://api.z.ai/api/coding/paas/v4");
        assert_eq!(profile.setup_url, "https://docs.z.ai/devpack/quick-start");
    }

    #[test]
    fn orcarouter_login_identifies_openai_compatible_endpoint() {
        let provider = resolve_login_selection("orcarouter", &cli_login_providers())
            .expect("OrcaRouter CLI login provider");
        let LoginProviderTarget::OpenAiCompatible(profile) = provider.target else {
            panic!("OrcaRouter should use the OpenAI-compatible runtime");
        };

        assert_eq!(profile.id, "orcarouter");
        assert_eq!(profile.api_base, "https://api.orcarouter.ai/v1");
        assert_eq!(profile.api_key_env, "ORCAROUTER_API_KEY");
        assert!(profile.requires_api_key);
    }

    #[test]
    fn normalize_api_base_accepts_private_http_hosts() {
        assert_eq!(
            normalize_api_base("http://192.168.1.25:8000/v1/").as_deref(),
            Some("http://192.168.1.25:8000/v1")
        );
        assert_eq!(
            normalize_api_base("http://10.0.0.8:11434/v1").as_deref(),
            Some("http://10.0.0.8:11434/v1")
        );
        assert_eq!(
            normalize_api_base("http://100.103.78.84:11434/v1").as_deref(),
            Some("http://100.103.78.84:11434/v1")
        );
        assert_eq!(
            normalize_api_base("http://hsv.local:11434/v1").as_deref(),
            Some("http://hsv.local:11434/v1")
        );
        assert_eq!(
            normalize_api_base("http://[fd00::1]:8080/v1").as_deref(),
            Some("http://[fd00::1]:8080/v1")
        );
    }

    #[test]
    fn normalize_api_base_rejects_public_http_hosts() {
        let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
        test_env::remove_var(ALLOW_INSECURE_HTTP_HOSTS_ENV);
        assert_eq!(normalize_api_base("http://example.com/v1"), None);
        assert_eq!(normalize_api_base("http://8.8.8.8/v1"), None);
    }

    #[test]
    fn insecure_http_allowlist_matches_exact_hosts() {
        assert!(allows_insecure_http_host_with(
            "llm.example.com",
            Some(8000),
            "llm.example.com"
        ));
        assert!(allows_insecure_http_host_with(
            "LLM.Example.com",
            Some(8000),
            " llm.example.com , other.host "
        ));
        assert!(!allows_insecure_http_host_with(
            "evil.example.com",
            Some(8000),
            "llm.example.com"
        ));
        // A suffix entry must not match the bare domain it is a suffix of.
        assert!(!allows_insecure_http_host_with(
            "example.com",
            Some(8000),
            "*.example.com"
        ));
    }

    #[test]
    fn insecure_http_allowlist_matches_wildcards_and_ports() {
        assert!(allows_insecure_http_host_with(
            "a.example.com",
            Some(80),
            "*.example.com"
        ));
        assert!(allows_insecure_http_host_with(
            "a.example.com",
            Some(80),
            ".example.com"
        ));
        assert!(!allows_insecure_http_host_with(
            "a.example.org",
            Some(80),
            "*.example.com"
        ));
        assert!(allows_insecure_http_host_with("8.8.8.8", Some(8080), "*"));
        assert!(allows_insecure_http_host_with(
            "gateway.example.com",
            Some(8080),
            "gateway.example.com:8080"
        ));
        assert!(!allows_insecure_http_host_with(
            "gateway.example.com",
            Some(9090),
            "gateway.example.com:8080"
        ));
        assert!(allows_insecure_http_host_with(
            "gateway.example.com",
            Some(8080),
            "*:8080"
        ));
        assert!(!allows_insecure_http_host_with(
            "gateway.example.com",
            Some(80),
            "*:8080"
        ));
        // Public IPv6 literals need the allowlist, and the port must match.
        assert!(allows_insecure_http_host_with(
            "[2001:db8::1]",
            Some(8080),
            "[2001:db8::1]"
        ));
        assert!(allows_insecure_http_host_with(
            "[2001:db8::1]",
            Some(8080),
            "[2001:db8::1]:8080"
        ));
        assert!(!allows_insecure_http_host_with(
            "[2001:db8::1]",
            Some(9090),
            "[2001:db8::1]:8080"
        ));
        assert!(!allows_insecure_http_host_with("", Some(80), "*"));
    }

    #[test]
    fn insecure_http_allowlist_env_var_enables_public_http() {
        let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
        let original = test_env::var(ALLOW_INSECURE_HTTP_HOSTS_ENV);
        test_env::set_var(
            ALLOW_INSECURE_HTTP_HOSTS_ENV,
            " api.example.com , 8.8.4.4:8080 ",
        );

        assert_eq!(
            normalize_api_base("http://api.example.com/v1/").as_deref(),
            Some("http://api.example.com/v1")
        );
        assert_eq!(
            normalize_api_base("http://8.8.4.4:8080/v1").as_deref(),
            Some("http://8.8.4.4:8080/v1")
        );
        // Ports not in the allowlist stay rejected.
        assert_eq!(normalize_api_base("http://8.8.4.4/v1"), None);
        // Unlisted hosts stay rejected.
        assert_eq!(normalize_api_base("http://other.example.com/v1"), None);
        // Private hosts keep working without the allowlist.
        assert_eq!(
            normalize_api_base("http://192.168.1.25:8000/v1").as_deref(),
            Some("http://192.168.1.25:8000/v1")
        );

        match original {
            Some(value) => test_env::set_var(ALLOW_INSECURE_HTTP_HOSTS_ENV, &value),
            None => test_env::remove_var(ALLOW_INSECURE_HTTP_HOSTS_ENV),
        }
    }

    #[test]
    fn insecure_http_allowlist_reads_registered_env_file_resolver() {
        let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
        test_env::remove_var(ALLOW_INSECURE_HTTP_HOSTS_ENV);
        register_env_file_value_resolver(|name| {
            (name == ALLOW_INSECURE_HTTP_HOSTS_ENV).then(|| "envfile.example.net".to_string())
        });

        assert_eq!(
            normalize_api_base("http://envfile.example.net/v1").as_deref(),
            Some("http://envfile.example.net/v1")
        );
        assert_eq!(normalize_api_base("http://other.example.net/v1"), None);
    }

    #[test]
    fn alibaba_coding_plan_uses_current_international_endpoint() {
        assert_eq!(
            ALIBABA_CODING_PLAN_PROFILE.api_base,
            "https://coding-intl.dashscope.aliyuncs.com/v1"
        );
    }

    #[test]
    fn resolve_login_provider_loose_matches_id_alias_and_display_name() {
        // id
        assert_eq!(
            resolve_login_provider_loose("anthropic-api").map(|d| d.id),
            Some("anthropic-api")
        );
        // alias
        assert_eq!(
            resolve_login_provider_loose("claude-api").map(|d| d.id),
            Some("anthropic-api")
        );
        // display name (the form LoginCompleted carries for API-key paste logins)
        assert_eq!(
            resolve_login_provider_loose("Anthropic API").map(|d| d.id),
            Some("anthropic-api")
        );
        // display name is matched case-insensitively
        assert_eq!(
            resolve_login_provider_loose("anthropic api").map(|d| d.id),
            Some("anthropic-api")
        );
        // unknown input stays unresolved
        assert_eq!(resolve_login_provider_loose("not-a-provider"), None);
    }

    #[test]
    fn resolve_login_provider_loose_resolves_every_descriptor_by_id_and_display_name() {
        // Guards the LoginCompleted attribution path: the TUI publishes either a
        // descriptor id (OAuth logins) or a display label (API-key paste logins),
        // and both must resolve so the post-login auth-change refresh is
        // attributed to the right provider instead of falling back to the
        // session's active provider.
        for descriptor in login_providers() {
            assert_eq!(
                resolve_login_provider_loose(descriptor.id).map(|d| d.id),
                Some(descriptor.id),
                "descriptor id {:?} should resolve",
                descriptor.id
            );
            assert_eq!(
                resolve_login_provider_loose(descriptor.display_name).map(|d| d.id),
                Some(descriptor.id),
                "display name {:?} (id {:?}) should resolve",
                descriptor.display_name,
                descriptor.id
            );
        }
    }

    #[test]
    fn minimax_profile_uses_official_openai_compatible_configuration() {
        assert_eq!(MINIMAX_PROFILE.api_base, "https://api.minimax.io/v1");
        assert_eq!(MINIMAX_PROFILE.api_key_env, "MINIMAX_API_KEY");
    }

    #[test]
    fn novita_profile_and_login_are_available_on_all_surfaces() {
        assert_eq!(NOVITA_PROFILE.api_base, "https://api.novita.ai/openai");
        assert_eq!(NOVITA_PROFILE.api_key_env, "NOVITA_API_KEY");
        assert_eq!(NOVITA_PROFILE.env_file, "novita.env");
        assert_eq!(NOVITA_PROFILE.default_model, Some("zai-org/glm-5.3"));
        assert!(NOVITA_PROFILE.requires_api_key);
        assert!(openai_compatible_profiles().contains(&NOVITA_PROFILE));
        for input in ["novita", "novita-ai", "novita.ai", " NOVITA "] {
            assert_eq!(resolve_login_provider(input), Some(NOVITA_LOGIN_PROVIDER));
        }
        assert_eq!(
            resolve_login_provider_loose("Novita AI"),
            Some(NOVITA_LOGIN_PROVIDER)
        );
        assert_eq!(
            NOVITA_LOGIN_PROVIDER.auth_kind,
            LoginProviderAuthKind::ApiKey
        );
        assert_eq!(
            NOVITA_LOGIN_PROVIDER.target,
            LoginProviderTarget::OpenAiCompatible(NOVITA_PROFILE)
        );
        for providers in [
            cli_login_providers(),
            tui_login_providers(),
            server_bootstrap_login_providers(),
            auto_init_login_providers(),
            auth_status_login_providers(),
        ] {
            assert_eq!(
                resolve_login_selection("novita", &providers),
                Some(NOVITA_LOGIN_PROVIDER)
            );
        }
    }

    #[test]
    fn nvidia_nim_profile_uses_hosted_openai_compatible_configuration() {
        assert_eq!(
            NVIDIA_NIM_PROFILE.api_base,
            "https://integrate.api.nvidia.com/v1"
        );
        assert_eq!(NVIDIA_NIM_PROFILE.api_key_env, "NVIDIA_API_KEY");
        assert_eq!(NVIDIA_NIM_PROFILE.env_file, "nvidia-nim.env");
        assert_eq!(
            NVIDIA_NIM_PROFILE.default_model,
            Some("nvidia/llama-3.1-nemotron-ultra-253b-v1")
        );
        assert!(matches!(
            NVIDIA_NIM_LOGIN_PROVIDER.target,
            LoginProviderTarget::OpenAiCompatible(profile) if profile.id == "nvidia-nim"
        ));
    }

    #[test]
    fn cerebras_profile_uses_official_openai_compatible_configuration() {
        assert_eq!(CEREBRAS_PROFILE.id, "cerebras");
        assert_eq!(CEREBRAS_PROFILE.display_name, "Cerebras");
        assert_eq!(CEREBRAS_PROFILE.api_base, "https://api.cerebras.ai/v1");
        assert_eq!(CEREBRAS_PROFILE.api_key_env, "CEREBRAS_API_KEY");
        assert_eq!(CEREBRAS_PROFILE.env_file, "cerebras.env");
        assert_eq!(
            CEREBRAS_PROFILE.setup_url,
            "https://inference-docs.cerebras.ai/introduction"
        );
        assert_eq!(CEREBRAS_PROFILE.default_model, Some("gpt-oss-120b"));
        const { assert!(CEREBRAS_PROFILE.requires_api_key) };
        assert_eq!(
            CEREBRAS_LOGIN_PROVIDER.auth_kind,
            LoginProviderAuthKind::ApiKey
        );
        assert_eq!(
            CEREBRAS_LOGIN_PROVIDER.auth_state_key,
            LoginProviderAuthStateKey::OpenRouterLike
        );
        assert!(matches!(
            CEREBRAS_LOGIN_PROVIDER.target,
            LoginProviderTarget::OpenAiCompatible(profile) if profile.id == "cerebras"
        ));
    }

    #[test]
    fn belvedir_profile_uses_official_inference_router_configuration() {
        assert_eq!(BELVEDIR_PROFILE.id, "belvedir");
        assert_eq!(BELVEDIR_PROFILE.display_name, "Belvedir");
        assert_eq!(
            BELVEDIR_PROFILE.api_base,
            "https://platform.belvedir.ai/api/v1/route"
        );
        assert_eq!(BELVEDIR_PROFILE.api_key_env, "BELVEDIR_API_KEY");
        assert_eq!(BELVEDIR_PROFILE.env_file, "belvedir.env");
        assert_eq!(BELVEDIR_PROFILE.default_model, Some("auto"));
        assert!(BELVEDIR_PROFILE.requires_api_key);

        let provider = resolve_login_provider("belvedir.ai").expect("Belvedir alias resolves");
        assert_eq!(provider.id, "belvedir");
        assert_eq!(
            provider.target,
            LoginProviderTarget::OpenAiCompatible(BELVEDIR_PROFILE)
        );
    }

    #[test]
    fn ollama_profile_is_local_openai_compatible_without_required_api_key() {
        assert_eq!(OLLAMA_PROFILE.id, "ollama");
        assert_eq!(OLLAMA_PROFILE.api_base, "http://localhost:11434/v1");
        assert_eq!(OLLAMA_PROFILE.api_key_env, "OLLAMA_API_KEY");
        assert_eq!(OLLAMA_PROFILE.env_file, "ollama.env");
        assert_eq!(
            OLLAMA_PROFILE.setup_url,
            "https://docs.ollama.com/api/openai-compatibility"
        );
        assert_eq!(OLLAMA_PROFILE.default_model, None);
        const {
            assert!(!OLLAMA_PROFILE.requires_api_key);
        }

        assert_eq!(
            OLLAMA_LOGIN_PROVIDER.auth_kind,
            LoginProviderAuthKind::Local
        );
        assert_eq!(OLLAMA_LOGIN_PROVIDER.auth_status_method, "local endpoint");
        assert!(matches!(
            OLLAMA_LOGIN_PROVIDER.target,
            LoginProviderTarget::OpenAiCompatible(profile) if profile.id == "ollama"
        ));
    }

    #[test]
    fn matrix_login_provider_aliases_resolve_to_canonical_ids() {
        assert_eq!(
            resolve_login_provider("subscription").map(|provider| provider.id),
            Some("jcode")
        );
        assert_eq!(
            resolve_login_provider("anthropic").map(|provider| provider.id),
            Some("claude")
        );
        assert_eq!(
            resolve_login_provider("opencodego").map(|provider| provider.id),
            Some("opencode-go")
        );
        assert_eq!(
            resolve_login_provider("z.ai").map(|provider| provider.id),
            Some("zai")
        );
        assert_eq!(
            resolve_login_provider("zhipu").map(|provider| provider.id),
            Some("zai")
        );
        assert_eq!(
            resolve_login_provider("kimi").map(|provider| provider.id),
            Some("kimi")
        );
        assert_eq!(
            resolve_login_provider("kimi-for-coding").map(|provider| provider.id),
            Some("kimi")
        );
        assert_eq!(
            resolve_login_provider("compat").map(|provider| provider.id),
            Some("openai-compatible")
        );
        assert_eq!(
            resolve_login_provider("aoai").map(|provider| provider.id),
            Some("azure")
        );
        assert_eq!(
            resolve_login_provider("cerberascode").map(|provider| provider.id),
            Some("cerebras")
        );
        assert_eq!(
            resolve_login_provider("bailian").map(|provider| provider.id),
            Some("alibaba-coding-plan")
        );
        assert_eq!(
            resolve_login_provider("302.ai").map(|provider| provider.id),
            Some("302ai")
        );
        assert_eq!(
            resolve_login_provider("hf").map(|provider| provider.id),
            Some("huggingface")
        );
        assert_eq!(
            resolve_login_provider("moonshot").map(|provider| provider.id),
            Some("moonshotai")
        );
        assert_eq!(
            resolve_login_provider("mistralai").map(|provider| provider.id),
            Some("mistral")
        );
        assert_eq!(
            resolve_login_provider("pplx").map(|provider| provider.id),
            Some("perplexity")
        );
        assert_eq!(
            resolve_login_provider("together").map(|provider| provider.id),
            Some("togetherai")
        );
        assert_eq!(
            resolve_login_provider("deep-infra").map(|provider| provider.id),
            Some("deepinfra")
        );
        assert_eq!(
            resolve_login_provider("fireworks.ai").map(|provider| provider.id),
            Some("fireworks")
        );
        assert_eq!(
            resolve_login_provider("minimax-ai").map(|provider| provider.id),
            Some("minimax")
        );
        assert_eq!(
            resolve_login_provider("grok").map(|provider| provider.id),
            Some("xai")
        );
        assert_eq!(
            resolve_login_provider("lm-studio").map(|provider| provider.id),
            Some("lmstudio")
        );
        assert_eq!(
            resolve_login_provider("gmail").map(|provider| provider.id),
            Some("google")
        );
    }

    #[test]
    fn matrix_login_provider_ids_and_aliases_are_unique() {
        let mut seen = HashSet::new();
        for provider in login_providers() {
            assert!(
                seen.insert(provider.id),
                "duplicate login provider identifier: {}",
                provider.id
            );
            for alias in provider.aliases {
                assert!(
                    seen.insert(*alias),
                    "duplicate login provider alias: {}",
                    alias
                );
            }
        }
    }

    #[test]
    fn matrix_tui_login_selection_supports_numbers_and_names() {
        let providers = tui_login_providers();
        assert_eq!(
            resolve_login_selection("1", &providers).map(|provider| provider.id),
            Some("auto-import")
        );
        assert_eq!(
            resolve_login_selection("2", &providers).map(|provider| provider.id),
            Some("claude")
        );
        // `anthropic-api` sits at 3 (between claude and openai), shifting the
        // rest of the list down one slot relative to the pre-May-2026 order.
        assert_eq!(
            resolve_login_selection("3", &providers).map(|provider| provider.id),
            Some("anthropic-api")
        );
        assert_eq!(
            resolve_login_selection("7", &providers).map(|provider| provider.id),
            Some("bedrock")
        );
        assert_eq!(
            resolve_login_selection("compat", &providers).map(|provider| provider.id),
            Some("openai-compatible")
        );
        assert!(resolve_login_selection("google", &providers).is_none());
    }

    #[test]
    fn matrix_cli_login_selection_preserves_existing_order() {
        let providers = cli_login_providers();
        assert_eq!(
            resolve_login_selection("1", &providers).map(|provider| provider.id),
            Some("auto-import")
        );
        // `anthropic-api` at 3 shifted everything after it down one slot.
        assert_eq!(
            resolve_login_selection("3", &providers).map(|provider| provider.id),
            Some("anthropic-api")
        );
        assert_eq!(
            resolve_login_selection("5", &providers).map(|provider| provider.id),
            Some("jcode")
        );
        assert_eq!(
            resolve_login_selection("6", &providers).map(|provider| provider.id),
            Some("copilot")
        );
        assert_eq!(
            resolve_login_selection("7", &providers).map(|provider| provider.id),
            Some("openrouter")
        );
        assert_eq!(
            resolve_login_selection("8", &providers).map(|provider| provider.id),
            Some("bedrock")
        );
        assert_eq!(
            resolve_login_selection("9", &providers).map(|provider| provider.id),
            Some("azure")
        );
        assert_eq!(
            resolve_login_selection("bedrock", &providers).map(|provider| provider.id),
            Some("bedrock")
        );
    }
}
