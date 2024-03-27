use ark_serialize::SerializationError;
use clap::{Parser, ValueEnum};
use solidity_verifiers::{Groth16Data, KzgData, NovaCyclefoldData, ProtocolData};
use std::{env, fmt::Display, path::PathBuf};

fn get_default_out_path() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("verifier.sol");
    path
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub(crate) enum Protocol {
    Groth16,
    Kzg,
    NovaCyclefold,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Would be nice to link this to the `Template` or `ProtocolData` traits.
// Sadly, this requires Boxing with `dyn` or similar which would complicate the code more than is actually required.
impl Protocol {
    pub(crate) fn render(
        &self,
        data: &[u8],
        pragma: Option<String>,
    ) -> Result<Vec<u8>, SerializationError> {
        match self {
            Self::Groth16 => {
                Ok(Groth16Data::deserialize_protocol_data(data)?.render_as_template(pragma))
            }

            Self::Kzg => Ok(KzgData::deserialize_protocol_data(data)?.render_as_template(pragma)),
            Self::NovaCyclefold => {
                Ok(NovaCyclefoldData::deserialize_protocol_data(data)?.render_as_template(pragma))
            }
        }
    }
}

const ABOUT: &str = "A Command-Line Interface (CLI) tool designed to simplify the generation of Solidity smart contracts that verify proofs of Zero Knowledge cryptographic protocols.
";

const LONG_ABOUT: &str = "
 _____  ______  ______  ______  ______  ______  ______ 
| |__| || |__| || |__| || |__| || |__| || |__| || |__| |
|  ()  ||  ()  ||  ()  ||  ()  ||  ()  ||  ()  ||  ()  |
|______||______||______||______||______||______||______|
 ______                                          ______ 
| |__| |   ____        _ _     _ _ _            | |__| |
|  ()  |  / ___|  ___ | (_) __| (_) |_ _   _    |  ()  |
|______|  \\___ \\ / _ \\| | |/ _` | | __| | | |   |______|
 ______    ___) | (_) | | | (_| | | |_| |_| |    ______ 
| |__| |  |____/ \\___/|_|_|\\__,_|_|\\__|\\__, |   | |__| |
|  ()  |  __     __        _  __ _     |___/    |  ()  |
|______|  \\ \\   / /__ _ __(_)/ _(_) ___ _ __    |______|
 ______    \\ \\ / / _ \\ '__| | |_| |/ _ \\ '__|    ______ 
| |__| |    \\ V /  __/ |  | |  _| |  __/ |      | |__| |
|  ()  |     \\_/ \\___|_|  |_|_| |_|\\___|_|      |  ()  |
|______|                                        |______|
 ______  ______  ______  ______  ______  ______  ______ 
| |__| || |__| || |__| || |__| || |__| || |__| || |__| |
|  ()  ||  ()  ||  ()  ||  ()  ||  ()  ||  ()  ||  ()  |
|______||______||______||______||______||______||______|

Welcome to Solidity Verifiers CLI, a Command-Line Interface (CLI) tool designed to simplify the generation of Solidity smart contracts that verify proofs of Zero Knowledge cryptographic protocols. This tool is developed by the collaborative efforts of the PSE (Privacy & Scaling Explorations) and 0XPARC teams.

Solidity Verifiers CLI is released under the MIT license, but notice that the Solidity template for the Groth16 verification has GPL-3.0 license, hence the generated Solidity verifiers that use the Groth16 template will have that license too.

Solidity Verifier currently supports the generation of Solidity smart contracts for the verification of proofs in the following Zero Knowledge protocols:

    Groth16:
        Efficient and succinct zero-knowledge proof system.

    KZG:
        Uses the Kate-Zaverucha-Goldberg polynomial commitment scheme.

    Nova + CycleFold Decider:
        Implements the decider circuit verification for the Nova proof system in conjunction with the CycleFold protocol optimization.
";
#[derive(Debug, Parser)]
#[command(author = "0XPARC & PSE", version, about = ABOUT, long_about = Some(LONG_ABOUT))]
#[command(propagate_version = true)]
/// A tool to create Solidity Contracts which act as verifiers for the major Folding Schemes implemented
/// within the `sonobe` repo.
pub(crate) struct Cli {
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,

    /// Selects the protocol for which we want to generate the Solidity Verifier contract.
    #[arg(short = 'p', long, value_enum, rename_all = "lower")]
    pub protocol: Protocol,

    #[arg(short = 'o', long, default_value=get_default_out_path().into_os_string())]
    /// Sets the output path for all the artifacts generated by the command.
    pub out: PathBuf,

    #[arg(short = 'd', long)]
    /// Sets the input path for the file containing all the data required by the protocol chosen such that the verification contract can be generated.
    pub protocol_data: PathBuf,

    /// Selects the Solidity compiler version to be set in the Solidity Verifier contract artifact.
    #[arg(long, default_value=None)]
    pub pragma: Option<String>,
}
