//! The sole bounded-I/O XSH evaluator entrypoint.

use std::{ffi::OsString, fs::File, io::Read, path::PathBuf};

use society_xsh_circuit::{
    Vs001DirectEvaluatorInputManifestV1, MAX_DIRECT_CURATION_MANIFEST_BYTES,
};

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(64);
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let [flag, input_manifest_path] = arguments.as_slice() else {
        return Err(usage());
    };
    if flag != "--input-manifest" {
        return Err(usage());
    }
    let input_manifest_path = PathBuf::from(input_manifest_path);
    if !input_manifest_path.is_absolute() {
        return Err(usage());
    }
    let input_file = File::open(input_manifest_path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(MAX_DIRECT_CURATION_MANIFEST_BYTES + 1);
    input_file
        .take((MAX_DIRECT_CURATION_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_DIRECT_CURATION_MANIFEST_BYTES {
        return Err("direct curation input manifest exceeds 131072 bytes".to_owned());
    }
    let input =
        Vs001DirectEvaluatorInputManifestV1::parse(&bytes).map_err(|error| error.to_string())?;
    print!(
        "{}",
        input
            .evaluate()
            .map_err(|error| error.to_string())?
            .canonical_tsv()
    );
    Ok(())
}

fn usage() -> String {
    "usage: vs001-direct-evaluator-adapter --input-manifest VERIFIED_ABSOLUTE_PATH".to_owned()
}
