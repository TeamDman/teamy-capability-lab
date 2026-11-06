use crate::capability::planner::SearchBudget;
use crate::capability::planner::plan;
use crate::capability::synthetic_fixture;
use crate::capability::visualization::diagram_json;
use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;
use figue as args;
use std::fs::OpenOptions;
use std::io::Write;
use teamy_cancellation::CancellationScope;

/// Enumerate and assess fixed synthetic transfer alternatives, without executing them.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct PlanArgs {
    /// Optionally create a new editable Excalidraw file. Existing files are not overwritten.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub excalidraw: Option<String>,
}

impl PlanArgs {
    /// # Errors
    /// Returns errors from planning, cancellation or explicit local diagram output.
    pub fn invoke(self, cancellation: &CancellationScope) -> Result<CliOutput> {
        let fixture = synthetic_fixture();
        let mut report = plan(&fixture, SearchBudget::default(), || {
            cancellation.bail_if_cancelled().is_err()
        })?;
        cancellation.bail_if_cancelled()?;
        if let Some(path) = self.excalidraw {
            let json = diagram_json(&report)?;
            cancellation.bail_if_cancelled()?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .wrap_err_with(|| format!("creating Excalidraw file {path}"))?;
            output
                .write_all(json.as_bytes())
                .wrap_err("writing Excalidraw scene")?;
            output.flush().wrap_err("flushing Excalidraw scene")?;
            report.diagram = Some(path);
        }
        Ok(CliOutput::facet(report))
    }
}
