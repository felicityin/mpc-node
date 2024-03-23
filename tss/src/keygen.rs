use anyhow::{Context, Ok, Result};

use cggmp21::{progress::PerfProfiler, supported_curves::Secp256k1};
use cggmp21_keygen::ExecutionId;
use key_share::DirtyCoreKeyShare;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_dev::DevRng;
use round_based::MpcParty;

#[cfg(feature = "curve-secp256k1")]
pub use generic_ec::curves::Secp256k1;

use crate::network::join_computation;

#[tokio::main]
pub async fn run(
    server_url: &str,
    room: &str,
    index: u16,
    threshold: u16,
    number_of_parties: u16,
) -> Result<DirtyCoreKeyShare<Secp256k1>> {
    let (delivery, listening, sending) = join_computation(server_url, room, index)
        .await
        .context("join computation")?;

    let party = MpcParty::connected(delivery);

    let mut rng = DevRng::new();
    let eid: [u8; 32] = [0u8; 32];
    let eid = ExecutionId::new(&eid);

    let mut profiler = PerfProfiler::new();

    let keygen = cggmp21::keygen::<Secp256k1>(eid, index, number_of_parties)
        .enforce_reliable_broadcast(false)
        .set_progress_tracer(&mut profiler);
        // .set_threshold(threshold);

    let mut party_rng = ChaCha20Rng::from_seed(rng.gen());
    let output = keygen
        .start(&mut party_rng, party)
        .await
        .expect("keygen failed");

    listening.abort();
    sending.abort();

    let report = profiler.get_report().context("get perf report")?;
    log::debug!("{}", report.display_io(false));
    log::info!("keygen done");

    Ok(output.into_inner())
}
