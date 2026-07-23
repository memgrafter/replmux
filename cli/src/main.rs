use std::error::Error;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use multirepl_runtime_cli::{
    ApiClient, EnvironmentSpec, KernelManager, ReplResponse, Runtime, RuntimeCreate, RuntimeStatus,
    RuntimeUpdate, SnapshotPolicy,
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

    #[arg(long, global = true, env = "MULTIREPL_KERNEL_DIR")]
    kernel_dir: Option<PathBuf>,

    #[arg(long, global = true, env = "MULTIREPL_PYTHON")]
    python: Option<PathBuf>,

    #[arg(long, global = true, env = "MULTIREPL_KERNEL_SCRIPT")]
    kernel_script: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
    Kernel {
        #[command(subcommand)]
        command: KernelCommand,
    },
    #[command(hide = true)]
    Create { name: String },
    #[command(hide = true)]
    List,
    #[command(hide = true)]
    Connect { name: String },
    #[command(hide = true)]
    Delete { name: String },
    #[command(hide = true)]
    Exec { name: String, code: String },
}

#[derive(Debug, Subcommand)]
enum KernelCommand {
    Create { name: String },
    List,
    Connect { name: String },
    Delete { name: String },
    Exec { name: String, code: String },
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
    match cli.command {
        Command::Runtime { command } => {
            let client = ApiClient::new(&cli.api_url)?;
            run_runtime(&client, command, cli.json)
        }
        Command::Kernel { command } => run_kernel(
            command,
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
        Command::Create { name } => run_kernel(
            KernelCommand::Create { name },
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
        Command::List => run_kernel(
            KernelCommand::List,
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
        Command::Connect { name } => run_kernel(
            KernelCommand::Connect { name },
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
        Command::Delete { name } => run_kernel(
            KernelCommand::Delete { name },
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
        Command::Exec { name, code } => run_kernel(
            KernelCommand::Exec { name, code },
            cli.json,
            cli.kernel_dir,
            cli.python,
            cli.kernel_script,
        ),
    }
}

fn run_kernel(
    command: KernelCommand,
    json_output: bool,
    kernel_dir: Option<PathBuf>,
    python: Option<PathBuf>,
    kernel_script: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let manager = KernelManager::from_options(kernel_dir, python, kernel_script)?;
    match command {
        KernelCommand::Create { name } => {
            let pid = manager.create(&name)?;
            if json_output {
                print_json(&json!({"name": name, "pid": pid, "status": "running"}))?;
            } else {
                println!("Kernel '{name}' started (pid {pid})");
            }
        }
        KernelCommand::List => {
            let kernels = manager.list()?;
            if json_output {
                print_json(&kernels)?;
            } else if kernels.is_empty() {
                println!("No kernels found.");
            } else {
                println!("{:<24} {:<10} STATUS", "NAME", "PID");
                for kernel in kernels {
                    let pid = kernel
                        .pid
                        .map_or_else(|| "?".to_owned(), |pid| pid.to_string());
                    println!("{:<24} {:<10} {}", kernel.name, pid, kernel.status);
                }
            }
        }
        KernelCommand::Connect { name } => print_json(&manager.connection(&name)?)?,
        KernelCommand::Delete { name } => {
            manager.delete(&name)?;
            if json_output {
                print_json(&json!({"name": name, "deleted": true}))?;
            } else {
                println!("Kernel '{name}' shut down.");
            }
        }
        KernelCommand::Exec { name, code } => {
            let response = manager.execute(&name, &code)?;
            if json_output {
                print_json(&response)?;
            } else {
                print_repl_response(&response)?;
            }
        }
    }
    Ok(())
}

fn print_repl_response(response: &ReplResponse) -> Result<(), Box<dyn Error>> {
    if !response.ok {
        return Err(response
            .error
            .clone()
            .unwrap_or_else(|| "kernel execution failed".to_owned())
            .into());
    }
    if response.mode.as_deref() == Some("eval") {
        if let Some(result) = &response.result {
            match result {
                serde_json::Value::String(value) => println!("{value}"),
                value => println!("{value}"),
            }
        }
    }
    if !response.stdout.is_empty() {
        print!("{}", response.stdout);
    }
    if !response.stderr.is_empty() {
        eprint!("{}", response.stderr);
    }
    Ok(())
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
