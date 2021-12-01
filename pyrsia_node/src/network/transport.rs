use noise::AuthenticKeypair;
use noise::X25519Spec;

use libp2p::{
    core::{muxing, transport, upgrade},
    mplex,
    noise,
    // `TokioTcpConfig` is available through the `tcp-tokio` feature.
    tcp::TokioTcpConfig,
    PeerId,
    Transport,
};

pub type TcpTokioTransport = transport::Boxed<(PeerId, muxing::StreamMuxerBox)>;

// Create a tokio-based TCP transport use noise for authenticated
// encryption and Mplex for multiplexing of substreams on a TCP stream.
pub fn new_tokio_tcp_transport(id_keys: &libp2p::identity::Keypair) -> TcpTokioTransport {
    // Create a keypair for authenticated encryption of the transport.
    let noise_keys: AuthenticKeypair<X25519Spec> = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
}
