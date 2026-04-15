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
    Scenario(ScenarioArgs),
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
    #[arg(long)]
    pub git: bool,
    #[arg(long)]
    pub no_git: bool,
    #[arg(long)]
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
    Info(ProjectReportArgs),
    Sync(ProjectReportArgs),
    Validate(ProjectReportArgs),
    Smoke(ProjectSmokeArgs),
    Add(ProjectAddArgs),
    Adopt(ProjectAdoptArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ProjectReportArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Api {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Frontend {
        #[arg(long, default_value = "react-vite")]
        framework: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ProjectAdoptArgs {
    #[command(subcommand)]
    pub source: ProjectAdoptSource,
}

#[derive(Debug, Args, Clone)]
pub struct ProjectSmokeArgs {
    #[arg(long)]
    pub install: bool,
    #[arg(long)]
    pub browser: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ProjectAdoptSource {
    Scaffold(ProjectReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct DevArgs {
    #[command(subcommand)]
    pub command: DevCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DevCommand {
    Up(DevReportArgs),
    Down(DevReportArgs),
    Status(DevReportArgs),
    Reset(DevReportArgs),
    Reseed(DevReportArgs),
    Snapshot(DevSnapshotArgs),
    Fund(DevFundArgs),
    Watch(DevWatchArgs),
    Events(DevEventsArgs),
    Logs(DevReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct DevReportArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DevFundArgs {
    pub target: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DevWatchArgs {
    #[arg(long, default_value_t = 1500)]
    pub interval_ms: u64,
    #[arg(long)]
    pub once: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DevEventsArgs {
    pub resource: Option<String>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DevSnapshotArgs {
    #[command(subcommand)]
    pub command: DevSnapshotCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DevSnapshotCommand {
    Save(DevSnapshotSaveArgs),
    Load(DevSnapshotLoadArgs),
}

#[derive(Debug, Args, Clone)]
pub struct DevSnapshotSaveArgs {
    pub name: Option<String>,
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DevSnapshotLoadArgs {
    pub name: Option<String>,
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct ScenarioArgs {
    #[command(subcommand)]
    pub command: ScenarioCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ScenarioCommand {
    Run(ScenarioRunArgs),
    Test(ScenarioRunArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ScenarioRunArgs {
    pub name: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Build {
        name: Option<String>,
        #[arg(long)]
        optimize: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Format(ContractFormatArgs),
    Lint(ContractLintArgs),
    Deploy {
        name: String,
        #[arg(long)]
        env: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Call(ContractCallArgs),
    Bind {
        contract: String,
        #[arg(long = "lang", value_delimiter = ',')]
        langs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Info {
        contract: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Fetch {
        contract: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Ttl(ContractTtlArgs),
    Spec(ContractSpecArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ContractFormatArgs {
    pub name: Option<String>,
    #[arg(long)]
    pub check: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct ContractLintArgs {
    pub name: Option<String>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct ContractSpecArgs {
    pub contract: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Airdrop(TokenAirdropArgs),
    AirdropReconcile(TokenAirdropReconcileArgs),
    AirdropResume(TokenAirdropResumeArgs),
    AirdropReport(TokenAirdropArgs),
    AirdropValidate(TokenAirdropArgs),
    AirdropPreview(TokenAirdropArgs),
    AirdropSummary(TokenAirdropArgs),
    Mint(TokenMoveArgs),
    Burn(TokenBurnArgs),
    Transfer(TokenMoveArgs),
    Trust {
        name: String,
        wallet: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Freeze {
        name: String,
        holder: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Unfreeze {
        name: String,
        holder: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Clawback {
        name: String,
        from: String,
        amount: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Sac(TokenSacArgs),
    Contract(TokenContractArgs),
    Balance(TokenBalanceArgs),
}

#[derive(Debug, Args, Clone)]
pub struct TokenBalanceArgs {
    pub name: String,
    #[arg(long)]
    pub holder: Option<String>,
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenAirdropArgs {
    pub name: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub sep7: bool,
    #[arg(long)]
    pub build_only: bool,
    #[arg(long)]
    pub relayer: bool,
}

#[derive(Debug, Args, Clone)]
pub struct TokenAirdropReconcileArgs {
    pub name: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenAirdropResumeArgs {
    pub name: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub report: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub start_at: Option<usize>,
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<usize>,
    #[arg(long)]
    pub sep7: bool,
    #[arg(long)]
    pub build_only: bool,
    #[arg(long)]
    pub relayer: bool,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenBurnArgs {
    pub name: String,
    #[arg(long)]
    pub amount: String,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct TokenSacArgs {
    #[command(subcommand)]
    pub command: TokenSacCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenSacCommand {
    Id {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Deploy {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct TokenContractArgs {
    #[command(subcommand)]
    pub command: TokenContractCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenContractCommand {
    Init {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Ls(WalletLsArgs),
    Address(WalletAddressArgs),
    Fund {
        name_or_address: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Balances(WalletBalancesArgs),
    Trust {
        wallet: String,
        token: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Pay(WalletPayArgs),
    BatchPay(WalletBatchPayArgs),
    BatchReconcile(WalletBatchReconcileArgs),
    BatchResume(WalletBatchResumeArgs),
    BatchReport(WalletBatchPayArgs),
    BatchValidate(WalletBatchPayArgs),
    BatchPreview(WalletBatchPayArgs),
    BatchSummary(WalletBatchPayArgs),
    Receive(WalletReceiveArgs),
    Sep7(WalletSep7Args),
    Smart(WalletSmartArgs),
}

#[derive(Debug, Args, Clone)]
pub struct WalletLsArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletAddressArgs {
    pub name: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletBalancesArgs {
    pub name_or_address: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletReceiveArgs {
    pub wallet: String,
    #[arg(long)]
    pub sep7: bool,
    #[arg(long)]
    pub qr: bool,
    #[arg(long)]
    pub asset: Option<String>,
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletBatchPayArgs {
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub asset: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub sep7: bool,
    #[arg(long)]
    pub build_only: bool,
    #[arg(long)]
    pub relayer: bool,
}

#[derive(Debug, Args, Clone)]
pub struct WalletBatchReconcileArgs {
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub asset: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletBatchResumeArgs {
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub report: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub asset: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub start_at: Option<usize>,
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<usize>,
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Scaffold {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Info {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Onboard(WalletSmartOnboardArgs),
    Provision {
        name: String,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        fund: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Materialize {
        name: String,
        #[arg(long)]
        fund: bool,
        #[arg(long)]
        no_policy_deploy: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Controller(WalletSmartControllerArgs),
    Policy(WalletSmartPolicyArgs),
}

#[derive(Debug, Args, Clone)]
pub struct WalletSmartControllerArgs {
    #[command(subcommand)]
    pub command: WalletSmartControllerCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WalletSmartControllerCommand {
    Rotate {
        name: String,
        identity: String,
        #[arg(long)]
        fund: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct WalletSmartPolicyArgs {
    #[command(subcommand)]
    pub command: WalletSmartPolicyCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WalletSmartPolicyCommand {
    Info(WalletSmartPolicyNamedArgs),
    Diff(WalletSmartPolicyNamedArgs),
    Sync(WalletSmartPolicyNamedArgs),
    Simulate {
        name: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Apply {
        name: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        build_only: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    SetDailyLimit {
        name: String,
        amount: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        build_only: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Allow {
        name: String,
        address: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        build_only: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Revoke {
        name: String,
        address: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        build_only: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct WalletSmartOnboardArgs {
    pub name: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WalletSmartPolicyNamedArgs {
    pub name: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub command: ApiCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiCommand {
    Init(ApiReportArgs),
    Generate(ApiGenerateArgs),
    Openapi(ApiOpenapiArgs),
    Events(ApiEventsArgs),
    Relayer(ApiRelayerArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ApiReportArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct ApiGenerateArgs {
    #[command(subcommand)]
    pub target: ApiGenerateTarget,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiGenerateTarget {
    Contract {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Token {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ApiOpenapiArgs {
    #[command(subcommand)]
    pub command: ApiOpenapiCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiOpenapiCommand {
    Export(ApiReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ApiEventsArgs {
    #[command(subcommand)]
    pub command: ApiEventsCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiEventsCommand {
    Init(ApiReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ApiRelayerArgs {
    #[command(subcommand)]
    pub command: ApiRelayerCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ApiRelayerCommand {
    Init(ApiReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct EventsArgs {
    #[command(subcommand)]
    pub command: EventsCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsCommand {
    Status(EventsStatusArgs),
    Export(EventsExportArgs),
    Replay(EventsReplayArgs),
    Watch(EventsWatchArgs),
    Ingest(EventsIngestArgs),
    Cursor(EventsCursorArgs),
    Backfill(EventsBackfillArgs),
}

#[derive(Debug, Args, Clone)]
pub struct EventsStatusArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsExportArgs {
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsReplayArgs {
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
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
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsIngestArgs {
    #[command(subcommand)]
    pub command: EventsIngestCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsIngestCommand {
    Init(EventsReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct EventsCursorArgs {
    #[command(subcommand)]
    pub command: EventsCursorCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventsCursorCommand {
    Ls(EventsCursorLsArgs),
    Reset {
        name: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct EventsCursorLsArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct EventsReportArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
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
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Deploy {
        env: String,
        #[arg(long)]
        confirm_mainnet: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Verify {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Status {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Drift {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Diff {
        env: String,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    History {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Inspect {
        env: String,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Rollback {
        env: String,
        #[arg(long)]
        to: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Prune(ReleasePruneArgs),
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
    Sync {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseEnvArgs {
    #[command(subcommand)]
    pub command: ReleaseEnvCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseEnvCommand {
    Export {
        env: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ReleaseRegistryArgs {
    #[command(subcommand)]
    pub command: ReleaseRegistryCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReleaseRegistryCommand {
    Publish {
        contract: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Deploy {
        contract: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args, Clone)]
pub struct ReleasePruneArgs {
    pub env: String,
    #[arg(long, default_value_t = 10)]
    pub keep: usize,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DoctorArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<DoctorCommand>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DoctorCommand {
    Env(DoctorReportArgs),
    Deps(DoctorReportArgs),
    Audit(DoctorAuditArgs),
    Fix(DoctorFixArgs),
    Network(DoctorNetworkArgs),
    Project(DoctorReportArgs),
}

#[derive(Debug, Args, Clone)]
pub struct DoctorAuditArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DoctorReportArgs {
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DoctorNetworkArgs {
    pub env: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct DoctorFixArgs {
    #[arg(long, value_enum)]
    pub scope: Option<DoctorFixScope>,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DoctorFixScope {
    All,
    Scripts,
    Events,
    Api,
    Frontend,
    Release,
    Lockfile,
}

impl Command {
    pub fn action_hint(&self) -> &'static str {
        match self {
            Command::Init(_) => "init",
            Command::Project(args) => match &args.command {
                ProjectCommand::Info(_) => "project.info",
                ProjectCommand::Sync(_) => "project.sync",
                ProjectCommand::Validate(_) => "project.validate",
                ProjectCommand::Smoke(_) => "project.smoke",
                ProjectCommand::Add(args) => match &args.target {
                    ProjectAddTarget::Contract { .. } => "project.add.contract",
                    ProjectAddTarget::Api { .. } => "project.add.api",
                    ProjectAddTarget::Frontend { .. } => "project.add.frontend",
                },
                ProjectCommand::Adopt(args) => match &args.source {
                    ProjectAdoptSource::Scaffold(_) => "project.adopt.scaffold",
                },
            },
            Command::Dev(args) => match &args.command {
                DevCommand::Up(_) => "dev.up",
                DevCommand::Down(_) => "dev.down",
                DevCommand::Status(_) => "dev.status",
                DevCommand::Reset(_) => "dev.reset",
                DevCommand::Reseed(_) => "dev.reseed",
                DevCommand::Snapshot(args) => match &args.command {
                    DevSnapshotCommand::Save(_) => "dev.snapshot.save",
                    DevSnapshotCommand::Load(_) => "dev.snapshot.load",
                },
                DevCommand::Fund(_) => "dev.fund",
                DevCommand::Watch(_) => "dev.watch",
                DevCommand::Events(_) => "dev.events",
                DevCommand::Logs(_) => "dev.logs",
            },
            Command::Scenario(args) => match &args.command {
                ScenarioCommand::Run(_) => "scenario.run",
                ScenarioCommand::Test(_) => "scenario.test",
            },
            Command::Contract(args) => match &args.command {
                ContractCommand::New { .. } => "contract.new",
                ContractCommand::Build { .. } => "contract.build",
                ContractCommand::Format(_) => "contract.format",
                ContractCommand::Lint(_) => "contract.lint",
                ContractCommand::Deploy { .. } => "contract.deploy",
                ContractCommand::Call(_) => "contract.call",
                ContractCommand::Bind { .. } => "contract.bind",
                ContractCommand::Info { .. } => "contract.info",
                ContractCommand::Fetch { .. } => "contract.fetch",
                ContractCommand::Ttl(args) => match &args.command {
                    ContractTtlCommand::Extend(_) => "contract.ttl.extend",
                    ContractTtlCommand::Restore(_) => "contract.ttl.restore",
                },
                ContractCommand::Spec(_) => "contract.spec",
            },
            Command::Token(args) => match &args.command {
                TokenCommand::Create(_) => "token.create",
                TokenCommand::Info { .. } => "token.info",
                TokenCommand::Airdrop(_) => "token.airdrop",
                TokenCommand::AirdropReconcile(_) => "token.airdrop-reconcile",
                TokenCommand::AirdropResume(_) => "token.airdrop-resume",
                TokenCommand::AirdropReport(_) => "token.airdrop-report",
                TokenCommand::AirdropValidate(_) => "token.airdrop-validate",
                TokenCommand::AirdropPreview(_) => "token.airdrop-preview",
                TokenCommand::AirdropSummary(_) => "token.airdrop-summary",
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
                TokenCommand::Balance(_) => "token.balance",
            },
            Command::Wallet(args) => match &args.command {
                WalletCommand::Create { .. } => "wallet.create",
                WalletCommand::Ls(_) => "wallet.ls",
                WalletCommand::Address(_) => "wallet.address",
                WalletCommand::Fund { .. } => "wallet.fund",
                WalletCommand::Balances(_) => "wallet.balances",
                WalletCommand::Trust { .. } => "wallet.trust",
                WalletCommand::Pay(_) => "wallet.pay",
                WalletCommand::BatchPay(_) => "wallet.batch-pay",
                WalletCommand::BatchReconcile(_) => "wallet.batch-reconcile",
                WalletCommand::BatchResume(_) => "wallet.batch-resume",
                WalletCommand::BatchReport(_) => "wallet.batch-report",
                WalletCommand::BatchValidate(_) => "wallet.batch-validate",
                WalletCommand::BatchPreview(_) => "wallet.batch-preview",
                WalletCommand::BatchSummary(_) => "wallet.batch-summary",
                WalletCommand::Receive(_) => "wallet.receive",
                WalletCommand::Sep7(args) => match &args.command {
                    WalletSep7Command::Payment(_) => "wallet.sep7.payment",
                    WalletSep7Command::ContractCall(_) => "wallet.sep7.contract-call",
                },
                WalletCommand::Smart(args) => match &args.command {
                    WalletSmartCommand::Create { .. } => "wallet.smart.create",
                    WalletSmartCommand::Scaffold { .. } => "wallet.smart.scaffold",
                    WalletSmartCommand::Info { .. } => "wallet.smart.info",
                    WalletSmartCommand::Onboard(_) => "wallet.smart.onboard",
                    WalletSmartCommand::Provision { .. } => "wallet.smart.provision",
                    WalletSmartCommand::Materialize { .. } => "wallet.smart.materialize",
                    WalletSmartCommand::Controller(args) => match &args.command {
                        WalletSmartControllerCommand::Rotate { .. } => {
                            "wallet.smart.controller.rotate"
                        }
                    },
                    WalletSmartCommand::Policy(args) => match &args.command {
                        WalletSmartPolicyCommand::Info(_) => "wallet.smart.policy.info",
                        WalletSmartPolicyCommand::Diff(_) => "wallet.smart.policy.diff",
                        WalletSmartPolicyCommand::Sync(_) => "wallet.smart.policy.sync",
                        WalletSmartPolicyCommand::Simulate { .. } => "wallet.smart.policy.simulate",
                        WalletSmartPolicyCommand::Apply { .. } => "wallet.smart.policy.apply",
                        WalletSmartPolicyCommand::SetDailyLimit { .. } => {
                            "wallet.smart.policy.set-daily-limit"
                        }
                        WalletSmartPolicyCommand::Allow { .. } => "wallet.smart.policy.allow",
                        WalletSmartPolicyCommand::Revoke { .. } => "wallet.smart.policy.revoke",
                    },
                },
            },
            Command::Api(args) => match &args.command {
                ApiCommand::Init(_) => "api.init",
                ApiCommand::Generate(args) => match &args.target {
                    ApiGenerateTarget::Contract { .. } => "api.generate.contract",
                    ApiGenerateTarget::Token { .. } => "api.generate.token",
                },
                ApiCommand::Openapi(args) => match &args.command {
                    ApiOpenapiCommand::Export(_) => "api.openapi.export",
                },
                ApiCommand::Events(args) => match &args.command {
                    ApiEventsCommand::Init(_) => "api.events.init",
                },
                ApiCommand::Relayer(args) => match &args.command {
                    ApiRelayerCommand::Init(_) => "api.relayer.init",
                },
            },
            Command::Events(args) => match &args.command {
                EventsCommand::Status(_) => "events.status",
                EventsCommand::Export(_) => "events.export",
                EventsCommand::Replay(_) => "events.replay",
                EventsCommand::Watch(_) => "events.watch",
                EventsCommand::Ingest(args) => match &args.command {
                    EventsIngestCommand::Init(_) => "events.ingest.init",
                },
                EventsCommand::Cursor(args) => match &args.command {
                    EventsCursorCommand::Ls(_) => "events.cursor.ls",
                    EventsCursorCommand::Reset { .. } => "events.cursor.reset",
                },
                EventsCommand::Backfill(_) => "events.backfill",
            },
            Command::Release(args) => match &args.command {
                ReleaseCommand::Plan { .. } => "release.plan",
                ReleaseCommand::Deploy { .. } => "release.deploy",
                ReleaseCommand::Verify { .. } => "release.verify",
                ReleaseCommand::Status { .. } => "release.status",
                ReleaseCommand::Drift { .. } => "release.drift",
                ReleaseCommand::Diff { .. } => "release.diff",
                ReleaseCommand::History { .. } => "release.history",
                ReleaseCommand::Inspect { .. } => "release.inspect",
                ReleaseCommand::Rollback { .. } => "release.rollback",
                ReleaseCommand::Prune(_) => "release.prune",
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
                Some(DoctorCommand::Env(_)) => "doctor.env",
                Some(DoctorCommand::Deps(_)) => "doctor.deps",
                Some(DoctorCommand::Audit(_)) => "doctor.audit",
                Some(DoctorCommand::Fix(_)) => "doctor.fix",
                Some(DoctorCommand::Network(_)) => "doctor.network",
                Some(DoctorCommand::Project(_)) => "doctor.project",
            },
        }
    }
}
