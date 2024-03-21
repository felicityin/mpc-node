use clap::Parser;
use std::fs::File;
use std::io::Write;

use tss_cli::opts::tss::{Opts, Subcommands};
use tss_lib::keygen;

fn main() -> eyre::Result<()> {
    let opts = Opts::parse();

    match opts.sub {
        // Keygen
        Subcommands::Keygen {
            server_url,
            room,
            index,
            threshold,
            number_of_parties,
            output,
        } => {
            let mut output_file = File::create(output)?;

            let data =
                keygen::run(&server_url, &room, index, threshold, number_of_parties).unwrap();

            _ = output_file.write_all(&data.as_slice());
        }
    }

    Ok(())
}
