use std::error::Error;

use clap::{Args, Parser, Subcommand};
use multirepl_runtime_cli::{
    ApiClient, EnvironmentSpec, Runtime, RuntimeCreate, RuntimeStatus, RuntimeUpdate,
    SnapshotPolicy,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "multirepl",
    version,
    about = "Manage Multirepl runtimes over HTTP"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "MULTIREPL_API_URL",
        default_value = "http://127.0.0.1:8000"
    )]
    api_url: String,

    #[arg(long, global = true, help = "Print JSON instead of a table")]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RuntimeCommand {
    Create(CreateArgs),
    List(ListArgs),
    Get { runtime_id: String },
    Update(UpdateArgs),
    Delete { runtime_id: String },
}

#[derive(Debug, Args)]
struct CreateArgs {
    name: String,

    #[arg(long, default_value = "python")]
    language: String,

    #[arg(long, default_value = "python")]
    environment_kind: String,

    #[arg(long, default_value = "python3")]
    executable: String,

    #[arg(long)]
    environment_digest: Option<String>,

    #[arg(long, default_value_t = 25)]
    snapshot_interval: u32,

    #[arg(long, default_value = "logical")]
    snapshot_mode: String,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(1..=100))]
    limit: u32,

    #[arg(long, default_value_t = 0)]
    offset: u64,

    #[arg(long)]
    status: Option<RuntimeStatus>,
}

#[derive(Debug, Args)]
struct UpdateArgs {
    runtime_id: String,

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    language: Option<String>,

    #[arg(long)]
    status: Option<RuntimeStatus>,

    #[arg(long)]
    environment_kind: Option<String>,

    #[arg(long)]
    executable: Option<String>,

    #[arg(long)]
    environment_digest: Option<String>,

    #[arg(long)]
    snapshot_interval: Option<u32>,

    #[arg(long)]
    snapshot_mode: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = ApiClient::new(&cli.api_url)?;

    match cli.command {
        Command::Runtime { command } => run_runtime(&client, command, cli.json),
    }
}

fn run_runtime(
    client: &ApiClient,
    command: RuntimeCommand,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    match command {
        RuntimeCommand::Create(args) => {
            let mut request = RuntimeCreate::new(args.name);
            request.language = args.language;
            request.environment = EnvironmentSpec {
                kind: args.environment_kind,
                executable: args.executable,
                digest: args.environment_digest,
            };
            request.snapshot_policy = SnapshotPolicy {
                interval_executions: args.snapshot_interval,
                mode: args.snapshot_mode,
            };
            print_runtime(&client.create_runtime(&request)?, json_output)?;
        }
        RuntimeCommand::List(args) => {
            let runtimes = client.list_runtimes(args.limit, args.offset, args.status)?;
            if json_output {
                print_json(&runtimes)?;
            } else {
                println!(
                    "{:<35}  {:<24}  {:<11}  {:<10}  REV",
                    "ID", "NAME", "STATUS", "LANGUAGE"
                );
                for runtime in runtimes.items {
                    println!(
                        "{:<35}  {:<24}  {:<11}  {:<10}  {}",
                        runtime.id,
                        runtime.name,
                        runtime.status,
                        runtime.language,
                        runtime.revision
                    );
                }
                println!("\n{} runtime(s)", runtimes.total);
            }
        }
        RuntimeCommand::Get { runtime_id } => {
            print_runtime(&client.get_runtime(&runtime_id)?, json_output)?;
        }
        RuntimeCommand::Update(args) => {
            let needs_current = args.environment_kind.is_some()
                || args.executable.is_some()
                || args.environment_digest.is_some()
                || args.snapshot_interval.is_some()
                || args.snapshot_mode.is_some();
            let current = if needs_current {
                Some(client.get_runtime(&args.runtime_id)?)
            } else {
                None
            };
            let environment = current.as_ref().and_then(|runtime| {
                if args.environment_kind.is_none()
                    && args.executable.is_none()
                    && args.environment_digest.is_none()
                {
                    return None;
                }
                let mut value = runtime.environment.clone();
                if let Some(kind) = args.environment_kind {
                    value.kind = kind;
                }
                if let Some(executable) = args.executable {
                    value.executable = executable;
                }
                if let Some(digest) = args.environment_digest {
                    value.digest = Some(digest);
                }
                Some(value)
            });
            let snapshot_policy = current.as_ref().and_then(|runtime| {
                if args.snapshot_interval.is_none() && args.snapshot_mode.is_none() {
                    return None;
                }
                let mut value = runtime.snapshot_policy.clone();
                if let Some(interval) = args.snapshot_interval {
                    value.interval_executions = interval;
                }
                if let Some(mode) = args.snapshot_mode {
                    value.mode = mode;
                }
                Some(value)
            });
            let request = RuntimeUpdate {
                name: args.name,
                language: args.language,
                environment,
                snapshot_policy,
                status: args.status,
            };
            if request.is_empty() {
                return Err("update requires at least one field".into());
            }
            print_runtime(
                &client.update_runtime(&args.runtime_id, &request)?,
                json_output,
            )?;
        }
        RuntimeCommand::Delete { runtime_id } => {
            client.delete_runtime(&runtime_id)?;
            if json_output {
                print_json(&json!({"deleted": runtime_id}))?;
            } else {
                println!("Deleted {runtime_id}");
            }
        }
    }
    Ok(())
}

fn print_runtime(runtime: &Runtime, json_output: bool) -> Result<(), serde_json::Error> {
    if json_output {
        return print_json(runtime);
    }
    println!("id:                {}", runtime.id);
    println!("name:              {}", runtime.name);
    println!("status:            {}", runtime.status);
    println!("language:          {}", runtime.language);
    println!("executable:        {}", runtime.environment.executable);
    println!("worker generation: {}", runtime.worker_generation);
    println!("revision:          {}", runtime.revision);
    println!("created:           {}", runtime.created_at);
    println!("updated:           {}", runtime.updated_at);
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
