use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub networks: BTreeMap<String, NetworkConfig>,
    #[serde(default)]
    pub identities: BTreeMap<String, IdentityConfig>,
    #[serde(default)]
    pub wallets: BTreeMap<String, WalletConfig>,
    #[serde(default)]
    pub tokens: BTreeMap<String, TokenConfig>,
    #[serde(default)]
    pub contracts: BTreeMap<String, ContractConfig>,
    #[serde(default)]
    pub api: Option<ApiConfig>,
    #[serde(default)]
    pub frontend: Option<FrontendConfig>,
    #[serde(default)]
    pub release: BTreeMap<String, ReleaseConfig>,
    #[serde(default)]
    pub scenarios: BTreeMap<String, ScenarioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_package_manager")]
    pub package_manager: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    #[serde(default = "default_network")]
    pub network: String,
    #[serde(default = "default_identity")]
    pub identity: String,
    #[serde(default = "default_output")]
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub rpc_url: String,
    #[serde(default)]
    pub horizon_url: String,
    #[serde(default)]
    pub network_passphrase: String,
    #[serde(default)]
    pub allow_http: bool,
    #[serde(default)]
    pub friendbot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentityConfig {
    #[serde(default = "default_identity_source")]
    pub source: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    #[serde(default = "default_wallet_kind")]
    pub kind: String,
    #[serde(default)]
    pub identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onboarding_app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_contract: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenConfig {
    #[serde(default = "default_token_kind")]
    pub kind: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub distribution: String,
    #[serde(default)]
    pub auth_required: bool,
    #[serde(default)]
    pub auth_revocable: bool,
    #[serde(default)]
    pub clawback_enabled: bool,
    #[serde(default)]
    pub with_sac: bool,
    #[serde(default = "default_decimals")]
    pub decimals: u32,
    #[serde(default)]
    pub metadata_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub bindings: Vec<String>,
    #[serde(default)]
    pub deploy_on: Vec<String>,
    #[serde(default)]
    pub init: Option<ContractInitConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractInitConfig {
    #[serde(rename = "fn", default)]
    pub fn_name: String,
    #[serde(default, flatten)]
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_api_framework")]
    pub framework: String,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(default = "default_events_backend")]
    pub events_backend: String,
    #[serde(default)]
    pub openapi: bool,
    #[serde(default)]
    pub relayer: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            framework: default_api_framework(),
            database: default_database(),
            events_backend: default_events_backend(),
            openapi: false,
            relayer: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrontendConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_frontend_framework")]
    pub framework: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseConfig {
    #[serde(default)]
    pub deploy_contracts: Vec<String>,
    #[serde(default)]
    pub deploy_tokens: Vec<String>,
    #[serde(default)]
    pub generate_env: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioConfig {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub steps: Vec<ScenarioStep>,
    #[serde(default)]
    pub assertions: Vec<ScenarioAssertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "assertion")]
pub enum ScenarioAssertion {
    #[serde(rename = "status")]
    Status { status: String },
    #[serde(rename = "step")]
    Step {
        step: usize,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        command_contains: Vec<String>,
        #[serde(default)]
        artifact_contains: Vec<String>,
        #[serde(default)]
        warning_contains: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ScenarioStep {
    #[serde(rename = "project.validate")]
    ProjectValidate,
    #[serde(rename = "project.sync")]
    ProjectSync,
    #[serde(rename = "dev.up")]
    DevUp,
    #[serde(rename = "dev.reseed")]
    DevReseed,
    #[serde(rename = "dev.fund")]
    DevFund { target: String },
    #[serde(rename = "contract.build")]
    ContractBuild {
        #[serde(default)]
        contract: Option<String>,
        #[serde(default)]
        optimize: bool,
    },
    #[serde(rename = "contract.deploy")]
    ContractDeploy {
        contract: String,
        #[serde(default)]
        env: Option<String>,
    },
    #[serde(rename = "contract.call")]
    ContractCall {
        contract: String,
        function: String,
        #[serde(default)]
        send: Option<String>,
        #[serde(default)]
        build_only: bool,
        #[serde(default)]
        args: Vec<String>,
    },
    #[serde(rename = "token.mint")]
    TokenMint {
        token: String,
        to: String,
        amount: String,
        #[serde(default)]
        from: Option<String>,
    },
    #[serde(rename = "wallet.pay")]
    WalletPay {
        from: String,
        to: String,
        asset: String,
        amount: String,
        #[serde(default)]
        sep7: bool,
        #[serde(default)]
        build_only: bool,
        #[serde(default)]
        relayer: bool,
    },
    #[serde(rename = "release.plan")]
    ReleasePlan {
        #[serde(default)]
        env: Option<String>,
    },
    #[serde(rename = "release.verify")]
    ReleaseVerify {
        #[serde(default)]
        env: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    #[serde(default)]
    pub environments: BTreeMap<String, EnvironmentLock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentLock {
    #[serde(default)]
    pub contracts: BTreeMap<String, ContractDeployment>,
    #[serde(default)]
    pub tokens: BTreeMap<String, TokenDeployment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractDeployment {
    #[serde(default)]
    pub contract_id: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub wasm_hash: String,
    #[serde(default)]
    pub tx_hash: String,
    #[serde(default)]
    pub deployed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenDeployment {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub issuer_identity: String,
    #[serde(default)]
    pub distribution_identity: String,
    #[serde(default)]
    pub sac_contract_id: String,
    #[serde(default)]
    pub contract_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestRef {
    Identity(String),
    Wallet(String),
    Token(String),
    TokenSac(String),
    Contract(String),
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn validate(&self, root: &Path) -> Vec<String> {
        let mut errors = Vec::new();
        if self.project.name.trim().is_empty() {
            errors.push("project.name is required".to_string());
        }
        if self.project.slug.trim().is_empty() {
            errors.push("project.slug is required".to_string());
        }
        if !matches!(
            self.project.package_manager.as_str(),
            "pnpm" | "npm" | "yarn" | "bun"
        ) {
            errors.push(format!(
                "project.package_manager `{}` must be one of pnpm, npm, yarn, or bun",
                self.project.package_manager
            ));
        }
        push_unsafe_key_errors(&mut errors, "network", self.networks.keys());
        push_unsafe_key_errors(&mut errors, "identity", self.identities.keys());
        push_unsafe_key_errors(&mut errors, "wallet", self.wallets.keys());
        push_unsafe_key_errors(&mut errors, "token", self.tokens.keys());
        push_unsafe_key_errors(&mut errors, "contract", self.contracts.keys());
        push_unsafe_key_errors(&mut errors, "release target", self.release.keys());
        push_unsafe_key_errors(&mut errors, "scenario", self.scenarios.keys());
        if !self.defaults.network.is_empty() && !self.networks.contains_key(&self.defaults.network)
        {
            errors.push(format!(
                "defaults.network references missing network `{}`",
                self.defaults.network
            ));
        }
        if !self.defaults.identity.is_empty()
            && !self.identities.contains_key(&self.defaults.identity)
        {
            errors.push(format!(
                "defaults.identity references missing identity `{}`",
                self.defaults.identity
            ));
        }
        for (name, wallet) in &self.wallets {
            if wallet.kind == "classic"
                && !wallet.identity.is_empty()
                && !self.identities.contains_key(&wallet.identity)
            {
                errors.push(format!(
                    "wallet `{name}` references missing identity `{}`",
                    wallet.identity
                ));
            }
            if wallet.kind == "smart" {
                if wallet.mode.as_deref().unwrap_or_default().trim().is_empty() {
                    errors.push(format!("smart wallet `{name}` is missing `mode`"));
                }
                if let Some(controller_identity) = &wallet.controller_identity
                    && !controller_identity.trim().is_empty()
                    && !self.identities.contains_key(controller_identity)
                {
                    errors.push(format!(
                        "smart wallet `{name}` references missing controller identity `{controller_identity}`"
                    ));
                }
                if let Some(policy_contract) = &wallet.policy_contract
                    && !self.contracts.contains_key(policy_contract)
                {
                    errors.push(format!(
                        "smart wallet `{name}` references missing policy contract `{policy_contract}`"
                    ));
                }
            }
        }
        for (name, token) in &self.tokens {
            for field in [&token.issuer, &token.distribution] {
                if let Some(reference) = parse_manifest_ref(field) {
                    match reference {
                        ManifestRef::Identity(identity) if !is_safe_name(&identity) => {
                            errors.push(format!(
                                "token `{name}` references unsafe identity `{identity}`"
                            ));
                        }
                        ManifestRef::Wallet(wallet) if !is_safe_name(&wallet) => {
                            errors.push(format!(
                                "token `{name}` references unsafe wallet `{wallet}`"
                            ));
                        }
                        ManifestRef::Identity(identity)
                            if !self.identities.contains_key(&identity) =>
                        {
                            errors.push(format!(
                                "token `{name}` references missing identity `{identity}`"
                            ));
                        }
                        ManifestRef::Wallet(wallet) if !self.wallets.contains_key(&wallet) => {
                            errors.push(format!(
                                "token `{name}` references missing wallet `{wallet}`"
                            ));
                        }
                        _ => {}
                    }
                }
            }
            if token.kind == "contract" && !self.contracts.contains_key(name) {
                errors.push(format!(
                    "token `{name}` is declared as a contract token but no matching contract `{name}` exists in the manifest"
                ));
            }
        }
        for (name, scenario) in &self.scenarios {
            if scenario.steps.is_empty() {
                errors.push(format!("scenario `{name}` must declare at least one step"));
            }
            if let Some(network) = scenario.network.as_deref()
                && !self.networks.contains_key(network)
            {
                errors.push(format!(
                    "scenario `{name}` references missing network `{network}`"
                ));
            }
            if let Some(identity) = scenario.identity.as_deref()
                && !self.identities.contains_key(identity)
            {
                errors.push(format!(
                    "scenario `{name}` references missing identity `{identity}`"
                ));
            }
            for step in &scenario.steps {
                match step {
                    ScenarioStep::ContractBuild {
                        contract: Some(contract),
                        ..
                    }
                    | ScenarioStep::ContractDeploy { contract, .. }
                    | ScenarioStep::ContractCall { contract, .. } => {
                        if !self.contracts.contains_key(contract) {
                            errors.push(format!(
                                "scenario `{name}` references missing contract `{contract}`"
                            ));
                        }
                    }
                    _ => {}
                }
                if let ScenarioStep::TokenMint { token, .. } = step
                    && !self.tokens.contains_key(token)
                {
                    errors.push(format!(
                        "scenario `{name}` references missing token `{token}`"
                    ));
                }
                match step {
                    ScenarioStep::ReleasePlan { env: Some(env) }
                    | ScenarioStep::ReleaseVerify { env: Some(env) }
                    | ScenarioStep::ContractDeploy { env: Some(env), .. } => {
                        if !self.networks.contains_key(env) {
                            errors.push(format!(
                                "scenario `{name}` references missing network `{env}`"
                            ));
                        }
                    }
                    _ => {}
                }
            }
            for assertion in &scenario.assertions {
                match assertion {
                    ScenarioAssertion::Status { status } => {
                        if !matches!(status.as_str(), "ok" | "warn" | "error") {
                            errors.push(format!(
                                "scenario `{name}` assertion status `{status}` must be one of ok, warn, or error"
                            ));
                        }
                    }
                    ScenarioAssertion::Step {
                        step,
                        status,
                        command_contains,
                        artifact_contains,
                        warning_contains,
                    } => {
                        if *step == 0 || *step > scenario.steps.len() {
                            errors.push(format!(
                                "scenario `{name}` assertion references missing step `{step}`"
                            ));
                        }
                        if let Some(status) = status
                            && !matches!(status.as_str(), "ok" | "warn" | "error")
                        {
                            errors.push(format!(
                                "scenario `{name}` step assertion status `{status}` must be one of ok, warn, or error"
                            ));
                        }
                        if status.is_none()
                            && command_contains.is_empty()
                            && artifact_contains.is_empty()
                            && warning_contains.is_empty()
                        {
                            errors.push(format!(
                                "scenario `{name}` step assertion for step `{step}` must declare at least one expectation"
                            ));
                        }
                    }
                }
            }
        }
        for (name, contract) in &self.contracts {
            if contract.path.trim().is_empty() {
                errors.push(format!("contract `{name}` is missing path"));
            } else if !is_safe_relative_path(Path::new(&contract.path)) {
                errors.push(format!(
                    "contract `{name}` path `{}` must stay inside the project root",
                    contract.path
                ));
            } else if !root.join(&contract.path).exists() {
                errors.push(format!(
                    "contract `{name}` points to missing path `{}`",
                    contract.path
                ));
            }
        }
        errors
    }

    pub fn active_network<'a>(
        &'a self,
        override_name: Option<&str>,
    ) -> Result<(&'a str, &'a NetworkConfig)> {
        let name = override_name.unwrap_or(&self.defaults.network);
        let network = self
            .networks
            .get_key_value(name)
            .ok_or_else(|| anyhow::anyhow!("network `{name}` is not defined in the manifest"))?;
        Ok((network.0.as_str(), network.1))
    }

    pub fn active_identity<'a>(&'a self, override_name: Option<&'a str>) -> Result<&'a str> {
        let name = override_name.unwrap_or(&self.defaults.identity);
        if self.identities.contains_key(name) {
            Ok(name)
        } else {
            bail!("identity `{name}` is not defined in the manifest");
        }
    }
}

fn push_unsafe_key_errors<'a>(
    errors: &mut Vec<String>,
    label: &str,
    names: impl IntoIterator<Item = &'a String>,
) {
    for name in names {
        if !is_safe_name(name) {
            errors.push(format!(
                "{label} key `{name}` must be a single filesystem-safe name"
            ));
        }
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self {
            version: 1,
            environments: BTreeMap::new(),
        }
    }
}

impl Lockfile {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn environment_mut(&mut self, env: &str) -> &mut EnvironmentLock {
        self.environments.entry(env.to_string()).or_default()
    }
}

pub fn parse_manifest_ref(input: &str) -> Option<ManifestRef> {
    let value = input.strip_prefix('@')?;
    let parts: Vec<&str> = value.split(':').collect();
    match parts.as_slice() {
        ["identity", name] => Some(ManifestRef::Identity((*name).to_string())),
        ["wallet", name] => Some(ManifestRef::Wallet((*name).to_string())),
        ["token", name] => Some(ManifestRef::Token((*name).to_string())),
        ["token", name, "sac"] => Some(ManifestRef::TokenSac((*name).to_string())),
        ["contract", name] => Some(ManifestRef::Contract((*name).to_string())),
        _ => None,
    }
}

pub(crate) fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) fn is_safe_name(value: &str) -> bool {
    let path = Path::new(value);
    let mut components = path.components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    )
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_package_manager() -> String {
    "pnpm".to_string()
}

fn default_network() -> String {
    "testnet".to_string()
}

fn default_identity() -> String {
    "alice".to_string()
}

fn default_output() -> String {
    "human".to_string()
}

fn default_identity_source() -> String {
    "stellar-cli".to_string()
}

fn default_wallet_kind() -> String {
    "classic".to_string()
}

fn default_token_kind() -> String {
    "asset".to_string()
}

fn default_decimals() -> u32 {
    7
}

fn default_api_framework() -> String {
    "fastify".to_string()
}

fn default_database() -> String {
    "sqlite".to_string()
}

fn default_events_backend() -> String {
    "rpc-poller".to_string()
}

fn default_frontend_framework() -> String {
    "react-vite".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ContractConfig, IdentityConfig, Manifest, ManifestRef, NetworkConfig, ProjectConfig,
        WalletConfig, is_safe_name, is_safe_relative_path, parse_manifest_ref,
    };
    use std::collections::BTreeMap;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn parse_manifest_ref_supports_token_sac_references() {
        assert_eq!(
            parse_manifest_ref("@token:points:sac"),
            Some(ManifestRef::TokenSac("points".to_string()))
        );
        assert_eq!(
            parse_manifest_ref("@wallet:alice"),
            Some(ManifestRef::Wallet("alice".to_string()))
        );
        assert_eq!(parse_manifest_ref("points"), None);
    }

    #[test]
    fn safe_relative_path_rejects_absolute_and_parent_segments() {
        assert!(is_safe_relative_path(Path::new("contracts/app")));
        assert!(!is_safe_relative_path(Path::new("../contracts/app")));
        assert!(!is_safe_relative_path(Path::new("/tmp/contracts/app")));
        assert!(!is_safe_relative_path(Path::new("./contracts/app")));
    }

    #[test]
    fn safe_name_accepts_single_segment_and_rejects_traversal() {
        assert!(is_safe_name("rewards"));
        assert!(is_safe_name("rewards_v2"));
        assert!(is_safe_name("rewards-v2"));
        assert!(!is_safe_name("../rewards"));
        assert!(!is_safe_name("nested/rewards"));
        assert!(!is_safe_name("./rewards"));
        assert!(!is_safe_name(""));
    }

    #[test]
    fn manifest_validation_rejects_contract_paths_outside_project_root() {
        let root = tempdir().expect("tempdir should be created");
        let manifest = Manifest {
            project: ProjectConfig {
                name: "demo".to_string(),
                slug: "demo".to_string(),
                ..ProjectConfig::default()
            },
            defaults: super::DefaultsConfig {
                network: "testnet".to_string(),
                identity: "alice".to_string(),
                ..super::DefaultsConfig::default()
            },
            networks: BTreeMap::from([(
                "testnet".to_string(),
                NetworkConfig {
                    kind: "testnet".to_string(),
                    ..NetworkConfig::default()
                },
            )]),
            identities: BTreeMap::from([(
                "alice".to_string(),
                IdentityConfig {
                    name: "alice".to_string(),
                    ..IdentityConfig::default()
                },
            )]),
            wallets: BTreeMap::from([(
                "alice".to_string(),
                WalletConfig {
                    identity: "alice".to_string(),
                    ..WalletConfig::default()
                },
            )]),
            contracts: BTreeMap::from([(
                "app".to_string(),
                ContractConfig {
                    path: "../escape".to_string(),
                    alias: "app".to_string(),
                    ..ContractConfig::default()
                },
            )]),
            ..Manifest::default()
        };

        let errors = manifest.validate(root.path());
        assert!(errors.iter().any(|error| {
            error.contains("contract `app` path `../escape` must stay inside the project root")
        }));
    }

    #[test]
    fn manifest_validation_rejects_unsafe_named_entries() {
        let root = tempdir().expect("tempdir should be created");
        let contracts_root = root.path().join("contracts").join("app");
        std::fs::create_dir_all(&contracts_root).expect("contract directory should be created");

        let manifest = Manifest {
            project: ProjectConfig {
                name: "demo".to_string(),
                slug: "demo".to_string(),
                ..ProjectConfig::default()
            },
            defaults: super::DefaultsConfig {
                network: "testnet".to_string(),
                identity: "alice".to_string(),
                ..super::DefaultsConfig::default()
            },
            networks: BTreeMap::from([(
                "../testnet".to_string(),
                NetworkConfig {
                    kind: "testnet".to_string(),
                    ..NetworkConfig::default()
                },
            )]),
            identities: BTreeMap::from([(
                "alice".to_string(),
                IdentityConfig {
                    name: "alice".to_string(),
                    ..IdentityConfig::default()
                },
            )]),
            wallets: BTreeMap::from([(
                "wallet/team".to_string(),
                WalletConfig {
                    identity: "alice".to_string(),
                    ..WalletConfig::default()
                },
            )]),
            tokens: BTreeMap::from([(
                "points/season-1".to_string(),
                super::TokenConfig::default(),
            )]),
            contracts: BTreeMap::from([(
                "app".to_string(),
                ContractConfig {
                    path: "contracts/app".to_string(),
                    alias: "app".to_string(),
                    ..ContractConfig::default()
                },
            )]),
            release: BTreeMap::from([("../prod".to_string(), super::ReleaseConfig::default())]),
            ..Manifest::default()
        };

        let errors = manifest.validate(root.path());
        assert!(errors.iter().any(|error| {
            error.contains("network key `../testnet` must be a single filesystem-safe name")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("wallet key `wallet/team` must be a single filesystem-safe name")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("token key `points/season-1` must be a single filesystem-safe name")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("release target key `../prod` must be a single filesystem-safe name")
        }));
    }
}
