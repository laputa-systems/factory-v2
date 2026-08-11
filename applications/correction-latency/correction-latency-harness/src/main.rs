//! Run the provider-free CL-001 matched experiment.

fn main() -> Result<(), correction_latency_harness::HarnessError> {
    let report = correction_latency_harness::run_provider_free_pair()?;
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--analysis-tsv")
    {
        let artifact = report.persisted_analysis_artifact().map_err(|_| {
            correction_latency_harness::HarnessError::UnexpectedEvent("analysis artifact")
        })?;
        print!("{}", artifact.render_tsv());
    } else {
        println!("{}", report.world_simulation_summary());
    }
    Ok(())
}
