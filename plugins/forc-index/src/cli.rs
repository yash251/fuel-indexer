pub(crate) use crate::commands::{
    build::Command as BuildCommand, check::Command as CheckCommand,
    deploy::Command as DeployCommand, init::Command as InitCommand,
    new::Command as NewCommand, remove::Command as RemoveCommand,
    start::Command as StartCommand,
};
use clap::{Parser, Subcommand};
use forc_tracing::{init_tracing_subscriber, TracingSubscriberOptions};

#[derive(Debug, Parser)]
#[clap(name = "forc index", about = "Fuel Index Orchestrator", version)]
struct Opt {
    /// The command to run
    #[clap(subcommand)]
    command: ForcIndex,
}

#[derive(Subcommand, Debug)]
enum ForcIndex {
    Init(InitCommand),
    New(NewCommand),
    Deploy(DeployCommand),
    Start(Box<StartCommand>),
    Check(CheckCommand),
    Remove(RemoveCommand),
    Build(BuildCommand),
}

pub async fn run_cli() -> Result<(), anyhow::Error> {
    let opt = Opt::parse();
    let tracing_options = TracingSubscriberOptions {
        ..Default::default()
    };

    init_tracing_subscriber(tracing_options);

    match opt.command {
        ForcIndex::Init(command) => crate::commands::init::exec(command),
        ForcIndex::New(command) => crate::commands::new::exec(command),
        ForcIndex::Deploy(command) => crate::commands::deploy::exec(command),
        ForcIndex::Start(command) => crate::commands::start::exec(command),
        ForcIndex::Check(command) => crate::commands::check::exec(command),
        ForcIndex::Remove(command) => crate::commands::remove::exec(command),
        ForcIndex::Build(command) => crate::commands::build::exec(command),
    }
}
