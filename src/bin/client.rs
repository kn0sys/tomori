use futures::stream::StreamExt;
use libp2p::{
    PeerId, SwarmBuilder, gossipsub, identity, mdns, noise, swarm::SwarmEvent, tcp, yamux,
};
use std::error::Error;
use std::time::Duration;
use tokio::time;

use tomori::network::{ValidatorBehaviour, ValidatorBehaviourEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Generate a random identity for the test client
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Test Client PeerId: {}", local_peer_id);

    // Setup the same encrypted transport layer as the main node
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

    // The topic we want to broadcast to
    let topic = gossipsub::IdentTopic::new("monero-l2-mempool");

    // Broadcast a test payload every 5 seconds
    let mut interval = time::interval(Duration::from_secs(5));

    // Manually construct the full multiaddr from Terminal 1's output
    let main_node_addr: libp2p::Multiaddr =
        "/ip4/127.0.0.1/tcp/35255/p2p/12D3KooWDuUdtBRUBCGbxPQjcxtJ2KZ8bs376YJbu84vgNLpLwpT"
            .parse()?;
    println!("Bypassing mDNS and dialing main node directly...");
    if let Err(e) = swarm.dial(main_node_addr) {
        eprintln!("Failed to dial main node: {:?}", e);
    }
    // ----------------------
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let dummy_payload = b"Dummy Curve Tree Proof: L2 Tx -> L1 Anchor";

                // Attempt to publish. This will fail gracefully until mDNS finds a peer.
                match swarm.behaviour_mut().gossipsub.publish(topic.clone(), dummy_payload) {
                    Ok(message_id) => println!("Published dummy L2 payload! Message ID: {}", message_id),
                    Err(gossipsub::PublishError::InsufficientPeers) => println!("Waiting for peers..."),
                    Err(e) => eprintln!("Publish error: {:?}", e),
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Test Client listening on {:?}", address);
                }
                SwarmEvent::Behaviour(ValidatorBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        println!("mDNS discovered main node: {} at {}", peer_id, multiaddr);
                        // Add the peer to Gossipsub so we can send messages to it
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                _ => {}
            }
        }
    }
}
