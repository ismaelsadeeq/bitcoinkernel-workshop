### BitcoinKernel Workshop

The aim of this workshop is to introduce the experimental bitcoinkernel stateful library, which will be released in Bitcoin Core v31.

**Workshop Overview:**
- Introduction to the library API
- Overview of available bitcoinkernel bindings
- Hands-on work with a slimmed version of the Rust toy node by Sebastian Kung, lead developer of the kernel project

This workshop is going to be done using the following crates:

1. **bitcoinkernel** - Stateful consensus engine
2. [**bitcoin-p2p**](https://docs.rs/bitcoin-p2p/latest/bitcoin_p2p/) - Encapsulates the P2P protocol layer, Bitcoin network primitives, peer management, and net processing
3. [**addrman**](https://docs.rs/bitcoin-addrman/latest/bitcoin_addrman/) - Manages gossiped Bitcoin peer addresses in the network
4. [**rust-bitcoin**](https://docs.rs/bitcoin/latest/bitcoin/index.html) - Handles consensus encoding, decoding, and Bitcoin types and primitives

Attendees will use the bitcoinkernel Rust crate API documentation to fill in missing lines of code in the toy node (based on the [kernel-node repository](https://github.com/sedited/kernel-node/tree/master)), then compile the node and perform an initial block download (IBD) on the Bitcoin signet network.

### Requirements
- An operating system with Docker installed

### Workshop Procedure

1. **Clone this repository:**
   ```
   git clone https://github.com/ismaelsadeeq/bitcoinkernel-workshop.git
   ```

2. **Make the build script executable:**
   ```
   chmod +x docker-dev.sh
   ```

3. **Build and enter the container:**
   ```
   ./docker-dev.sh
   ```

4. **Complete the exercises:**
   ```
   git grep "TODO:"
   ```
   vim filename, edit, save and exit all the files.

5. **Run the IBD script:**
   ```
   cd kernel-node
   cargo run --bin node --release -- --network signet
   ```

