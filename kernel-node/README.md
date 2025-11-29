# Kernel-Node-Workshop

- A slimmed down version of [kernel-node](https://github.com/sedited/kernel-node) for
  a workshop at the African Bitcoin Conference Mauritius ABC.

- The aim is to build a toy bitcoin node using the libbitcoinkernel
  library. The node just validates blocks, and does not serve blocks to the
  network. It is meant to showcase the limited initial API of the kernel library.

- For now, IBD is done from a single peer, selected from the DNS seed nodes. If the
  connection to this peer happens to fail for some reason, a new peer will be selected.
  A direct connection can also be selected from the command line. See `--help` for
  this.

- To run on e.g. signet:

```
cargo run --bin node --release -- --network signet
```

=======
By default it will put data in the `$HOME/.kernel-node` directory.

Options: 

```
Arguments:
        --network    Which Bitcoin network to use.
        --datadir    Data directory for blockchain and configuration.
        --connect    Connect only to this node (format: ip:port or hostname:port)
```
