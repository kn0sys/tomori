use libp2p::{gossipsub, mdns, swarm::NetworkBehaviour, PeerId};

/// The custom network behaviour for the L2 Validator.
/// It combines mDNS for local peer discovery and Gossipsub for transaction broadcasting.
#[derive(NetworkBehaviour)]
pub struct ValidatorBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

impl ValidatorBehaviour {
    pub fn new(
        local_peer_id: PeerId,
        local_key: libp2p::identity::Keypair,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Configure Gossipsub
        let message_authenticity = gossipsub::MessageAuthenticity::Signed(local_key);
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .max_transmit_size(1024 * 1024 * 5) // 5MB to handle massive Curve Tree proofs
            .build()
            .expect("Valid config");

        let mut gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .expect("Correct configuration");

        // Subscribe to our L2 mempool topic
        let topic = gossipsub::IdentTopic::new("monero-l2-mempool");
        gossipsub.subscribe(&topic)?;

        // 2. Configure mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        Ok(Self { gossipsub, mdns })
    }
}
