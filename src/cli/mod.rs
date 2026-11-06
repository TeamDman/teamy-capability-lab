pub mod disclosure;
pub mod facet_shape;
pub mod global_args;
pub mod output;
pub mod plan;

use crate::cli::disclosure::DisclosureArgs;
use crate::cli::global_args::GlobalArgs;
use crate::cli::output::CliOutput;
use crate::cli::plan::PlanArgs;
use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};
use teamy_cancellation::CancellationScope;

/// Offline capability planning using fixed synthetic data.
///
/// Planning does not transfer files or authorize execution.
#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    /// Global logging, cancellation and output options.
    #[facet(flatten)]
    pub global_args: GlobalArgs,
    /// Standard CLI options (help, version, completions).
    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,
    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl Cli {
    /// # Errors
    /// Returns an error if runtime construction, cancellation or planning fails.
    pub fn invoke(self, cancellation: &CancellationScope) -> eyre::Result<CliOutput> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to build tokio runtime")?;
        runtime.block_on(
            cancellation.cancellable_scope(|cancellation| async move {
                self.command.invoke(&cancellation)
            }),
        )
    }
}

/// Available offline operations.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Assess synthetic candidate routes without execution.
    Plan(PlanArgs),
    /// Assess one fictional identity and VM disclosure scenario.
    Disclosure(DisclosureArgs),
}

impl Command {
    /// # Errors
    /// Returns planning, cancellation or local diagram output errors.
    pub fn invoke(self, cancellation: &CancellationScope) -> eyre::Result<CliOutput> {
        match self {
            Self::Plan(args) => args.invoke(cancellation),
            Self::Disclosure(args) => {
                cancellation.bail_if_cancelled()?;
                args.invoke()
            }
        }
    }
}
