use futures::stream::StreamExt;
use libp2p::{
    Multiaddr, PeerId, SwarmBuilder, core::upgrade, gossipsub, identity, mdns, noise,
    swarm::SwarmEvent, tcp, yamux,
};
use std::error::Error;

mod network;
use network::ValidatorBehaviour;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Generate a random PeerId and Keypair for this validator
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local validator PeerId: {}", local_peer_id);

    // Standard Monero stagenet ZMQ port is usually 38081
    let daemon_zmq_addr = "tcp://127.0.0.1:38081";

    tokio::spawn(async move {
        if let Err(e) = tomori::l1_bridge::start_zmq_listener(daemon_zmq_addr).await {
            eprintln!("ZMQ Listener crashed: {}", e);
        }
    });

    // Setup the encrypted transport layer (TCP + Noise + Yamux)
    let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| ValidatorBehaviour::new(local_peer_id, key.clone()).unwrap())?
        .build();

    // Listen on all interfaces, random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // The core event loop
    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Validator listening on {:?}", address);
                }
                SwarmEvent::Behaviour(network::ValidatorBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        println!("mDNS discovered new validator: {} at {}", peer_id, multiaddr);
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(network::ValidatorBehaviourEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message, .. })) => {
                    println!("Received L2 payload from {}: '{:?}'", propagation_source, message.data);
                    // TODO: Pass byte array to Curve Tree verification logic
                }
                _ => {}
            }
        }
    }
}
