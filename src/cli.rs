use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[command(
    name = "stellar-forge",
    bin_name = "stellar-forge",
    version,
    about = "Opinionated DX and orchestration layer for Stellar projects."
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Args, Clone, Default)]
pub struct GlobalOptions {
    #[arg(long, global = true)]
    pub manifest: Option<PathBuf>,
    #[arg(long, global = true)]
    pub cwd: Option<PathBuf>,
    #[arg(long, global = true)]
    pub network: Option<String>,
    #[arg(long, global = true)]
    pub identity: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub quiet: bool,
    #[arg(long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    #[arg(long, global = true)]
    pub dry_run: bool,
    #[arg(long, global = true)]
    pub yes: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    Init(InitArgs),
    Project(ProjectArgs),
    Dev(DevArgs),
    Contract(ContractArgs),
    Token(TokenArgs),
    Wallet(WalletArgs),
    Api(ApiArgs),
    Events(EventsArgs),
    Release(ReleaseArgs),
    Doctor(DoctorArgs),
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    pub name: String,
    #[arg(long, value_enum, default_value = "fullstack")]
    pub template: ProjectTemplate,
    #[arg(long, default_value = "react-vite")]
    pub frontend: String,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub api: bool,
    #[arg(long)]
    pub no_api: bool,
    #[arg(long, default_value = "classic")]
    pub wallet: String,
    #[arg(long, default_value_t = 1)]
    pub contracts: usize,
    #[arg(long, default_value = "testnet")]
    pub network: String,
    #[arg(long, default_value = "pnpm")]
    pub package_manager: String,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub git: bool,
    #[arg(long)]
    pub no_git: bool,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub install: bool,
    #[arg(long)]
    pub no_install: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProjectTemplate {
    MinimalContract,
    Fullstack,
    IssuerWallet,
    MerchantCheckout,
    RewardsLoyalty,
    ApiOnly,
    MultiContract,
}

#[derive(Debug, Args, Clone)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ProjectCommand {
    Info,
    Sync,
    Validate,
    Add(ProjectAddArgs),
    Adopt(ProjectAdoptArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ProjectAddArgs {
    #[command(subcommand)]
    pub target: ProjectAddTarget,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ProjectAddTarget {
    Contract {
        name: String,
        #[arg(long, default_value = "basic")]
        template: String,
    },
    Api,
    Frontend {
        #[arg(long, default_value = "react-vite")]
        framework: String,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ProjectAdoptArgs {
    #[command(subcommand)]
    pub source: ProjectAdoptSource,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ProjectAdoptSource {
    Scaffold,
}

#[derive(Debug, Args, Clone)]
pub struct DevArgs {
    #[command(subcommand)]
    pub command: DevCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DevCommand {
    Up,
    Down,
    Status,
    Reset,
    Reseed,
    Fund {
        target: String,
    },
    Watch {
        #[arg(long, default_value_t = 1500)]
        interval_ms: u64,
        #[arg(long)]
        once: bool,
    },
    Events {
        resource: Option<String>,
    },
    Logs,
}

#[derive(Debug, Args, Clone)]
pub struct ContractArgs {
    #[command(subcommand)]
    pub command: ContractCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ContractCommand {
    New {
        name: String,
        #[arg(long, default_value = "basic")]
        template: String,
    },
    Build {
        name: Option<String>,
        #[arg(long)]
        optimize: bool,
    },
    Deploy {
        name: String,
        #[arg(long)]
        env: Option<String>,
    },
    Call(ContractCallArgs),
    Bind {
        contract: String,
        #[arg(long = "lang", value_delimiter = ',')]
        langs: Vec<String>,
    },
    Info {
        contract: String,
    },
    Fetch {
        contract: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Ttl(ContractTtlArgs),
    Spec {
        contract: String,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ContractTtlArgs {
    #[command(subcommand)]
    pub command: ContractTtlCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ContractTtlCommand {
    Extend(ContractTtlMutationArgs),
    Restore(ContractTtlMutationArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ContractTtlMutationArgs {
    pub contract: String,
    #[arg(long, default_value_t = 17280)]
    pub ledgers: u32,
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long, value_enum, default_value = "persistent")]
    pub durability: StorageDurability,
    #[arg(long)]
    pub ttl_ledger_only: bool,
    #[arg(long)]
    pub build_only: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StorageDurability {
    Persistent,
    Temporary,
}

#[derive(Debug, Args, Clone)]
pub struct ContractCallArgs {
    pub contract: String,
    pub function: String,
    #[arg(long, default_value = "default")]
    pub send: String,
    #[arg(long)]
    pub build_only: bool,
    #[arg(last = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenArgs {
    #[command(subcommand)]
    pub command: TokenCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenCommand {
    Create(TokenCreateArgs),
    Info {
        name: String,
    },
    Mint(TokenMoveArgs),
    Burn(TokenBurnArgs),
    Transfer(TokenMoveArgs),
    Trust {
        name: String,
        wallet: String,
    },
    Freeze {
        name: String,
        holder: String,
    },
    Unfreeze {
        name: String,
        holder: String,
    },
    Clawback {
        name: String,
        from: String,
        amount: String,
    },
    Sac(TokenSacArgs),
    Contract(TokenContractArgs),
    Balance {
        name: String,
        #[arg(long)]
        holder: Option<String>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct TokenCreateArgs {
    pub name: String,
    #[arg(long, default_value = "asset")]
    pub mode: String,
    #[arg(long)]
    pub code: Option<String>,
    #[arg(long, default_value = "issuer")]
    pub issuer: String,
    #[arg(long, default_value = "treasury")]
    pub distribution: String,
    #[arg(long)]
    pub with_sac: bool,
    #[arg(long, default_value = "0")]
    pub initial_supply: String,
    #[arg(long)]
    pub auth_required: bool,
    #[arg(long)]
    pub auth_revocable: bool,
    #[arg(long)]
    pub clawback_enabled: bool,
    #[arg(long)]
    pub metadata_name: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenMoveArgs {
    pub name: String,
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub amount: String,
    #[arg(long)]
    pub from: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenBurnArgs {
    pub name: String,
    #[arg(long)]
    pub amount: String,
    #[arg(long)]
    pub from: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenSacArgs {
    #[command(subcommand)]
    pub command: TokenSacCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenSacCommand {
    Id { name: String },
    Deploy { name: String },
}

#[derive(Debug, Args, Clone)]
pub struct TokenContractArgs {
    #[command(subcommand)]
    pub command: TokenContractCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenContractCommand {
    Init { name: String },
}

#[derive(Debug, Args, Clone)]
pub struct WalletArgs {
    #[command(subcommand)]
    pub command: WalletCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WalletCommand {
    Create {
        name: String,
        #[arg(long)]
        fund: bool,
    },
    Ls,
    Address {
        name: String,
    },
    Fund {
        name_or_address: String,
    },
    Balances {
        name_or_address: String,
    },
    Trust {
        wallet: String,
        token: String,
    },
    Pay(WalletPayArgs),
    Receive {
        wallet: String,
        #[arg(long)]
        sep7: bool,
        #[arg(long)]
        qr: bool,
        #[arg(long)]
        asset: Option<String>,
    },
    Sep7(WalletSep7Args),
    Smart(WalletSmartArgs),
}

#[derive(Debug, Args, Clone)]
pub struct WalletPayArgs {
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub asset: String,
    #[arg(long)]
    pub amount: String,
    #[arg(long)]
    pub sep7: bool,
    #[arg(long)]
    pub build_only: bool,
    #[arg(long)]
    pub relayer: bool,
}

#[derive(Debug, Args, Clone)]
pub struct WalletSep7Args {
    #[command(subcommand)]
    pub command: WalletSep7Command,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WalletSep7Command {
    Payment(WalletPayArgs),
    ContractCall(ContractCallArgs),
}

#[derive(Debug, Args, Clone)]
pub struct WalletSmartArgs {
    #[command(subcommand)]
    pub command: WalletSmartCommand,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SmartWalletMode {
    Ed25519,
    Passkey,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WalletSmartCommand {
    Create {
        name: String,
        #[arg(long, value_enum, default_value = "ed25519")]
        mode: SmartWalletMode,
    },
    Scaffold {
        name: String,
    },
    Info {
        name: String,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub command: ApiCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiCommand {
    Init,
    Generate(ApiGenerateArgs),
    Openapi(ApiOpenapiArgs),
    Events(ApiEventsArgs),
    Relayer(ApiRelayerArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ApiGenerateArgs {
    #[command(subcommand)]
    pub target: ApiGenerateTarget,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiGenerateTarget {
    Contract { name: String },
    Token { name: String },
}

#[derive(Debug, Args, Clone)]
pub struct ApiOpenapiArgs {
    #[command(subcommand)]
    pub command: ApiOpenapiCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiOpenapiCommand {
    Export,
}

#[derive(Debug, Args, Clone)]
pub struct ApiEventsArgs {
    #[command(subcommand)]
    pub command: ApiEventsCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiEventsCommand {
    Init,
}

#[derive(Debug, Args, Clone)]
pub struct ApiRelayerArgs {
    #[command(subcommand)]
    pub command: ApiRelayerCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiRelayerCommand {
    Init,
}

#[derive(Debug, Args, Clone)]
pub struct EventsArgs {
    #[command(subcommand)]
    pub command: EventsCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsCommand {
    Watch(EventsWatchArgs),
    Ingest(EventsIngestArgs),
    Cursor(EventsCursorArgs),
    Backfill(EventsBackfillArgs),
}

#[derive(Debug, Args, Clone)]
pub struct EventsWatchArgs {
    pub kind: String,
    pub resource: String,
    #[arg(long = "topic")]
    pub topics: Vec<String>,
    #[arg(long)]
    pub count: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long = "start-ledger")]
    pub start_ledger: Option<u64>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsBackfillArgs {
    pub resource: String,
    #[arg(long = "topic")]
    pub topics: Vec<String>,
    #[arg(long)]
    pub count: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long = "start-ledger")]
    pub start_ledger: Option<u64>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsIngestArgs {
    #[command(subcommand)]
    pub command: EventsIngestCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsIngestCommand {
    Init,
}

#[derive(Debug, Args, Clone)]
pub struct EventsCursorArgs {
    #[command(subcommand)]
    pub command: EventsCursorCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsCursorCommand {
    Ls,
    Reset { name: String },
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseArgs {
    #[command(subcommand)]
    pub command: ReleaseCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseCommand {
    Plan {
        env: String,
    },
    Deploy {
        env: String,
        #[arg(long)]
        confirm_mainnet: bool,
    },
    Verify {
        env: String,
    },
    Aliases(ReleaseAliasesArgs),
    Env(ReleaseEnvArgs),
    Registry(ReleaseRegistryArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseAliasesArgs {
    #[command(subcommand)]
    pub command: ReleaseAliasesCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseAliasesCommand {
    Sync { env: String },
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseEnvArgs {
    #[command(subcommand)]
    pub command: ReleaseEnvCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseEnvCommand {
    Export { env: String },
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseRegistryArgs {
    #[command(subcommand)]
    pub command: ReleaseRegistryCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseRegistryCommand {
    Publish { contract: String },
    Deploy { contract: String },
}

#[derive(Debug, Args, Clone)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub command: Option<DoctorCommand>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DoctorCommand {
    Env,
    Deps,
    Network { env: String },
    Project,
}

impl Command {
    pub fn action_hint(&self) -> &'static str {
        match self {
            Command::Init(_) => "init",
            Command::Project(args) => match &args.command {
                ProjectCommand::Info => "project.info",
                ProjectCommand::Sync => "project.sync",
                ProjectCommand::Validate => "project.validate",
                ProjectCommand::Add(args) => match &args.target {
                    ProjectAddTarget::Contract { .. } => "project.add.contract",
                    ProjectAddTarget::Api => "project.add.api",
                    ProjectAddTarget::Frontend { .. } => "project.add.frontend",
                },
                ProjectCommand::Adopt(args) => match &args.source {
                    ProjectAdoptSource::Scaffold => "project.adopt.scaffold",
                },
            },
            Command::Dev(args) => match &args.command {
                DevCommand::Up => "dev.up",
                DevCommand::Down => "dev.down",
                DevCommand::Status => "dev.status",
                DevCommand::Reset => "dev.reset",
                DevCommand::Reseed => "dev.reseed",
                DevCommand::Fund { .. } => "dev.fund",
                DevCommand::Watch { .. } => "dev.watch",
                DevCommand::Events { .. } => "dev.events",
                DevCommand::Logs => "dev.logs",
            },
            Command::Contract(args) => match &args.command {
                ContractCommand::New { .. } => "contract.new",
                ContractCommand::Build { .. } => "contract.build",
                ContractCommand::Deploy { .. } => "contract.deploy",
                ContractCommand::Call(_) => "contract.call",
                ContractCommand::Bind { .. } => "contract.bind",
                ContractCommand::Info { .. } => "contract.info",
                ContractCommand::Fetch { .. } => "contract.fetch",
                ContractCommand::Ttl(args) => match &args.command {
                    ContractTtlCommand::Extend(_) => "contract.ttl.extend",
                    ContractTtlCommand::Restore(_) => "contract.ttl.restore",
                },
                ContractCommand::Spec { .. } => "contract.spec",
            },
            Command::Token(args) => match &args.command {
                TokenCommand::Create(_) => "token.create",
                TokenCommand::Info { .. } => "token.info",
                TokenCommand::Mint(_) => "token.mint",
                TokenCommand::Burn(_) => "token.burn",
                TokenCommand::Transfer(_) => "token.transfer",
                TokenCommand::Trust { .. } => "token.trust",
                TokenCommand::Freeze { .. } => "token.freeze",
                TokenCommand::Unfreeze { .. } => "token.unfreeze",
                TokenCommand::Clawback { .. } => "token.clawback",
                TokenCommand::Sac(args) => match &args.command {
                    TokenSacCommand::Id { .. } => "token.sac.id",
                    TokenSacCommand::Deploy { .. } => "token.sac.deploy",
                },
                TokenCommand::Contract(args) => match &args.command {
                    TokenContractCommand::Init { .. } => "token.contract.init",
                },
                TokenCommand::Balance { .. } => "token.balance",
            },
            Command::Wallet(args) => match &args.command {
                WalletCommand::Create { .. } => "wallet.create",
                WalletCommand::Ls => "wallet.ls",
                WalletCommand::Address { .. } => "wallet.address",
                WalletCommand::Fund { .. } => "wallet.fund",
                WalletCommand::Balances { .. } => "wallet.balances",
                WalletCommand::Trust { .. } => "wallet.trust",
                WalletCommand::Pay(_) => "wallet.pay",
                WalletCommand::Receive { .. } => "wallet.receive",
                WalletCommand::Sep7(args) => match &args.command {
                    WalletSep7Command::Payment(_) => "wallet.sep7.payment",
                    WalletSep7Command::ContractCall(_) => "wallet.sep7.contract-call",
                },
                WalletCommand::Smart(args) => match &args.command {
                    WalletSmartCommand::Create { .. } => "wallet.smart.create",
                    WalletSmartCommand::Scaffold { .. } => "wallet.smart.scaffold",
                    WalletSmartCommand::Info { .. } => "wallet.smart.info",
                },
            },
            Command::Api(args) => match &args.command {
                ApiCommand::Init => "api.init",
                ApiCommand::Generate(args) => match &args.target {
                    ApiGenerateTarget::Contract { .. } => "api.generate.contract",
                    ApiGenerateTarget::Token { .. } => "api.generate.token",
                },
                ApiCommand::Openapi(args) => match &args.command {
                    ApiOpenapiCommand::Export => "api.openapi.export",
                },
                ApiCommand::Events(args) => match &args.command {
                    ApiEventsCommand::Init => "api.events.init",
                },
                ApiCommand::Relayer(args) => match &args.command {
                    ApiRelayerCommand::Init => "api.relayer.init",
                },
            },
            Command::Events(args) => match &args.command {
                EventsCommand::Watch(_) => "events.watch",
                EventsCommand::Ingest(args) => match &args.command {
                    EventsIngestCommand::Init => "events.ingest.init",
                },
                EventsCommand::Cursor(args) => match &args.command {
                    EventsCursorCommand::Ls => "events.cursor.ls",
                    EventsCursorCommand::Reset { .. } => "events.cursor.reset",
                },
                EventsCommand::Backfill(_) => "events.backfill",
            },
            Command::Release(args) => match &args.command {
                ReleaseCommand::Plan { .. } => "release.plan",
                ReleaseCommand::Deploy { .. } => "release.deploy",
                ReleaseCommand::Verify { .. } => "release.verify",
                ReleaseCommand::Aliases(args) => match &args.command {
                    ReleaseAliasesCommand::Sync { .. } => "release.aliases.sync",
                },
                ReleaseCommand::Env(args) => match &args.command {
                    ReleaseEnvCommand::Export { .. } => "release.env.export",
                },
                ReleaseCommand::Registry(args) => match &args.command {
                    ReleaseRegistryCommand::Publish { .. } => "release.registry.publish",
                    ReleaseRegistryCommand::Deploy { .. } => "release.registry.deploy",
                },
            },
            Command::Doctor(args) => match &args.command {
                None => "doctor",
                Some(DoctorCommand::Env) => "doctor.env",
                Some(DoctorCommand::Deps) => "doctor.deps",
                Some(DoctorCommand::Network { .. }) => "doctor.network",
                Some(DoctorCommand::Project) => "doctor.project",
            },
        }
    }
}
