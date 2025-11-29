use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex, Once,
    },
    thread::{self, available_parallelism},
    time::Duration,
};

use bitcoin::{BlockHash, Network, TestnetVersion};
use bitcoinkernel::{
    core::BlockHashExt, prelude::BlockValidationStateExt, ChainType, ChainstateManagerBuilder,
    Context, ContextBuilder, Log, Logger, SynchronizationState, ValidationMode,
};
use kernel_node::kernel_util::{ChainExt, DirnameExt};
use kernel_node::peer::{BitcoinPeer, NodeState, TipState};
use log::{debug, error, info, warn};
use p2p::{
    dns::{BITCOIN_SEEDS, SIGNET_SEEDS, TESTNET3_SEEDS, TESTNET4_SEEDS},
    p2p_message_types::{address::AddrV2, message::AddrV2Payload, NetworkExt, ServiceFlags},
};

const TABLE_WIDTH: usize = 16;
const TABLE_SLOT: usize = 16;
const MAX_BUCKETS: usize = 4;

const DNS_RESOLVER: IpAddr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

configure_me::include_config!();

//! The Context holds the kernel library's logically global state and is
//! passed to operations that need access to this state. It should be kept in
//! scope for the duration of all operations that depend on it.
fn create_context(
    chain_type: ChainType,
    shutdown_tx: mpsc::Sender<()>,
    tip_state: &Arc<Mutex<TipState>>,
) -> Arc<Context> {
    let shutdown_triggered = Arc::new(AtomicBool::new(false));
    let shutdown_triggered_clone = Arc::clone(&shutdown_triggered);
    let shutdown_tx_clone = shutdown_tx.clone();
    let tip_state_clone = tip_state.clone();
    Arc::new(ContextBuilder::new()
        .chain_type(chain_type)
        .with_block_tip_notification(|state, hash: bitcoinkernel::BlockHash, _| {
                let hash = BlockHash::from_byte_array(hash.into());
                match state {
                    SynchronizationState::InitDownload => debug!("Received new block tip {} during IBD.", hash),
                    SynchronizationState::PostInit => info!("Received new block {}", hash),
                    SynchronizationState::InitReindex => debug!("Moved new block tip {} during reindex.", hash),
                };
        })
        // TODO 3: Handle block header notification
        .with_progress_notification(|title, progress, resume_possible| {
                warn!("Made progress {}: {}. Can resume: {}", title, progress, resume_possible)
        })
        .with_warning_set_notification(|_warning, _message| {})
        .with_warning_unset_notification(|_warning| {})
        .with_flush_error_notification(move |message| {
                if !shutdown_triggered.swap(true, Ordering::SeqCst) {
                    shutdown_tx.send(()).expect("failed to send shutdown signal");
                }
                error!("Fatal flush error encountered: {}", message);
        })
        .with_fatal_error_notification(move |message| {
                error!("Fatal error encountered: {}", message);
                if !shutdown_triggered_clone.swap(true, Ordering::SeqCst) {
                    shutdown_tx_clone.send(()).expect("failed to send shutdown signal");
                }
        })
        // .with_block_checked_validation(setup_validation_interface(tip_state))
        .with_block_checked_validation(move |block: bitcoinkernel::Block, state: bitcoinkernel::BlockValidationStateRef<'_>| {
            match state.mode() {
                ValidationMode::Valid => {
                    let hash = bitcoin::BlockHash::from_byte_array(block.hash().into());
                    log::debug!("Validation interface: Successfully checked block: {}", hash);
                    tip_state_clone.lock().unwrap().block_hash = hash;
                }
                _ => error!("Received an invalid block!"),
            }
        })
        // extra handle, tricky notification that is not handled 
        .build()
        .unwrap())
}

struct KernelLog {}

impl Log for KernelLog {
    fn log(&self, message: &str) {
        log::info!(
            target: "bitcoinkernel", 
            "{}", message.strip_suffix("\r\n").or_else(|| message.strip_suffix('\n')).unwrap_or(message));
    }
}

static START: Once = Once::new();
static mut GLOBAL_LOG_CALLBACK_HOLDER: Option<Logger> = None;

fn setup_logging() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter(None, log::LevelFilter::Info).init();

    unsafe { GLOBAL_LOG_CALLBACK_HOLDER = Some(Logger::new(KernelLog {}).unwrap()) };
}

fn resolve_seeds(network: Network) -> Vec<IpAddr> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let format_hostname = |host: &str| format!("{host}:53");
    let seeds: Vec<String> = match network {
        Network::Bitcoin => BITCOIN_SEEDS.into_iter().map(format_hostname).collect(),
        Network::Signet => SIGNET_SEEDS.into_iter().map(format_hostname).collect(),
        Network::Testnet(TestnetVersion::V3) => {
            TESTNET3_SEEDS.into_iter().map(format_hostname).collect()
        }
        Network::Testnet(TestnetVersion::V4) => {
            TESTNET4_SEEDS.into_iter().map(format_hostname).collect()
        }
        Network::Regtest => Vec::new(),
        _ => panic!("unknown network."),
    };
    let mut results = Vec::new();
    for host in seeds {
        let peers = rt.block_on(async move {
            tokio::net::lookup_host(host)
                .await
                .map(|sockets| sockets.map(|socket| socket.ip()).collect())
                .unwrap_or(Vec::new())
        });
        results.extend(peers);
    }
    results
}

fn run(
    network: Network,
    connect: Option<SocketAddr>,
    mut node_state: NodeState,
    shutdown_rx: mpsc::Receiver<()>,
    addr_rx: mpsc::Receiver<AddrV2Payload>,
    block_rx: mpsc::Receiver<bitcoinkernel::Block>,
) -> std::io::Result<()> {
    let mut table = addrman::Table::<TABLE_WIDTH, TABLE_SLOT, MAX_BUCKETS>::new();
    match connect {
        Some(connect) => {
            let record = match connect.ip() {
                IpAddr::V4(ipv4) => addrman::Record::new(
                    AddrV2::Ipv4(ipv4),
                    connect.port(),
                    ServiceFlags::NETWORK,
                    &DNS_RESOLVER,
                ),
                IpAddr::V6(ipv6) => addrman::Record::new(
                    AddrV2::Ipv6(ipv6),
                    connect.port(),
                    ServiceFlags::NETWORK,
                    &DNS_RESOLVER,
                ),
            };
            table.add(&record);
        }
        None => {
            let addresses = resolve_seeds(network);
            info!("{} addresses resolved from the dns seeds", addresses.len());
            for addr in &addresses {
                let record = match addr {
                    IpAddr::V4(ipv4) => addrman::Record::new(
                        AddrV2::Ipv4(*ipv4),
                        network.default_p2p_port(),
                        ServiceFlags::NETWORK,
                        &DNS_RESOLVER,
                    ),
                    IpAddr::V6(ipv6) => addrman::Record::new(
                        AddrV2::Ipv6(*ipv6),
                        network.default_p2p_port(),
                        ServiceFlags::NETWORK,
                        &DNS_RESOLVER,
                    ),
                };
                table.add(&record);
            }
        }
    };

    let chainman = Arc::clone(&node_state.chainman);
    let context = Arc::clone(&node_state.context);
    let addrman = Arc::new(Mutex::new(table));

    let running = Arc::new(AtomicBool::new(true));
    let running_addr = running.clone();
    let running_peer = running.clone();
    let running_block = running.clone();

    let peer_source = Arc::clone(&addrman);
    let kill = Arc::new(Mutex::new(None));
    let writer = Arc::clone(&kill);

    let peer_processing_handler = thread::spawn(move || {
        info!("Starting net processing thread.");
        while running_peer.load(Ordering::SeqCst) {
            let addr_lock = peer_source.lock().unwrap();
            let (address, port) = addr_lock.select().unwrap().network_addr();
            let peer = match address {
                AddrV2::Ipv4(ipv4) => BitcoinPeer::new(
                    SocketAddr::V4(SocketAddrV4::new(ipv4, port)),
                    network,
                    &mut node_state,
                ),
                AddrV2::Ipv6(ipv6) => {
                    let socket_adrr = (ipv6, port).into();
                    BitcoinPeer::new(socket_adrr, network, &mut node_state)
                }
                _ => continue,
            };
            let mut peer = match peer {
                Ok(connection) => {
                    let mut writer_lock = writer.lock().unwrap();
                    *writer_lock = Some(connection.writer());
                    connection
                }
                Err(e) => {
                    error!("Could not connect: {e}");
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            };
            loop {
                if let Err(e) = peer.receive_and_process_message(&mut node_state) {
                    match e {
                        p2p::net::Error::Io(io) => {
                            if io.kind() != std::io::ErrorKind::UnexpectedEof {
                                error!("Unexpected I/O error: {}", io);
                            }
                        }
                        e => error!("Error processing message: {e}"),
                    }
                    break;
                }
            }
        }
        info!("Stopping net processing thread.");
    });

    let addr_processing_handler = thread::spawn(move || {
        info!("Starting addr processing thread.");
        while running_addr.load(Ordering::SeqCst) {
            match addr_rx.recv() {
                Ok(payload) => {
                    let mut addr_lock = addrman.lock().unwrap();
                    for address in payload.0 {
                        let record = addrman::Record::new(
                            address.addr,
                            address.port,
                            address.services,
                            &DNS_RESOLVER,
                        );
                        addr_lock.add(&record);
                    }
                }
                Err(_) => break,
            }
        }
        info!("Stopping addr processing thread.");
    });

    let block_processing_handler = thread::spawn(move || {
        info!("Starting block processing thread.");
        while running_block.load(Ordering::SeqCst) {
            match block_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(block) => {
                    debug!("Validating block.");
                    // TODO:4 pass the received block to chainmanager for validation
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        info!("Stopping block processing thread.");
    });

    if let Ok(()) = shutdown_rx.recv() {
        context.interrupt().unwrap();
        let mut peer_lock = kill.lock().unwrap();
        if let Some(conn) = peer_lock.deref_mut() {
            conn.shutdown().unwrap()
        }
        info!("Received shutdown signal, shutting down...");
        running.store(false, Ordering::SeqCst);
    }

    addr_processing_handler.join().unwrap();
    peer_processing_handler.join().unwrap();
    block_processing_handler.join().unwrap();

    info!("exiting.");
    Ok(())
}

fn main() {
    let (config, _) = Config::including_optional_config_files::<&[&str]>(&[]).unwrap_or_exit();
    START.call_once(|| {
        setup_logging();
    });
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let tip_state = Arc::new(Mutex::new(TipState::default()));

    let network = config.network.parse::<Network>().expect("invalid network");
    let context = create_context(network.chain_type(), shutdown_tx.clone(), &tip_state);

    ctrlc::set_handler(move || shutdown_tx.send(()).unwrap()).unwrap();
    let data_dir = config.datadir.data_dir();
    let blocks_dir = data_dir.clone() + "/blocks";
    // TODO 5: Create a new chainmanager builder and chainman
    let (block_tx, block_rx) = mpsc::sync_channel(1);
    let (addr_tx, addr_rx) = mpsc::channel();

    let node_state = NodeState {
        addr_tx,
        block_tx,
        tip_state,
        chainman,
        context: Arc::clone(&context),
    };

    // TODO 6: Import blocks
    if let Err(err) =  // line of code goes here {
        error!("Error importing blocks: {}", err);
        return;
    }

    // TODO 7: Get and set the chain tip
    // let tip_index = node_state.chainman. 
    let hash = tip_index.block_hash();
    node_state.set_tip_state(BlockHash::from_byte_array(hash.to_bytes()));

    info!("Bitcoin kernel initialized");

    let connect = config
        .connect
        .map(|sock| sock.parse::<SocketAddr>().unwrap());

    if shutdown_rx.try_recv().is_ok() {
        info!("Shutting down!");
        return;
    }

    run(network, connect, node_state, shutdown_rx, addr_rx, block_rx).unwrap();
}
