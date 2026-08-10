//! Run the provider-free CL-001 matched experiment.

fn main() -> Result<(), correction_latency_harness::HarnessError> {
    let report = correction_latency_harness::run_provider_free_pair()?;
    println!("{report:#?}");
    Ok(())
}
