# Multi-Protocol DEX Tools (Rust)

A generic Rust command-line tool for reading pool data and building swap instructions across multiple DEX protocols on Solana.

## Features

- **Multi-protocol support**: Currently supports Tessera, easily extensible for other protocols
- **Pool data fetching**: Reads live pool account data from Solana blockchain
- **Swap transaction building**: Creates and submits swap transactions for different protocols
- **Transaction simulation**: Builds and simulates actual swap transactions via RPC
- **Wallet integration**: Uses private key from .env for transaction signing
- **Protocol-agnostic design**: Common interfaces for easy addition of new protocols
- **Configuration management**: .env support with sensible defaults
- **Clean CLI interface**: Simple command-line interface for all operations

## Installation

Make sure you have Rust installed, then:

```bash
cargo build --release
```

## Usage

```bash
# Read pool data (default protocol: tessera)
cargo run -- read -p FLckHLGMJy5gEoXWwcE68Nprde1D4araK4TGLw4pQq2n

# Read pool data with specific protocol
cargo run -- --protocol tessera read -p FLckHLGMJy5gEoXWwcE68Nprde1D4araK4TGLw4pQq2n
cargo run -- --protocol goonfi read -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46
cargo run -- --protocol obric read -p 3csCx7o3KCvpXeRZYjhCGrgXdKyjSgaqcirc5pZJUJCf
cargo run -- --protocol saros read -p 7EFmig3Jb9j1kJ7ppaUs5iY8P5pBnRdQXUR4q9vSCY37
cargo run -- --protocol alphaq read -p Pi9nzTjPxD8DsRfRBGfKYzmefJoJM8TcXu2jyaQjSHm
cargo run -- --protocol humidifi read -p 3QYYvFWgSuGK8bbxMSAYkCqE8QfSuFtByagnZAuekia2
cargo run -- --protocol aquifer read -p Pi9nzTjPxD8DsRfRBGfKYzmefJoJM8TcXu2jyaQjSHm

# Build and submit swap transaction
cargo run -- swap -p BDqQBspbipXxnTX2kw4FPM9pzfcf9kwieGCy4yUZ9tCC -i 1000000 -a 1
cargo run --  --protocol goonfi swap -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46 -i 1000000 -a 1
cargo run --  --protocol obric swap -p 3csCx7o3KCvpXeRZYjhCGrgXdKyjSgaqcirc5pZJUJCf -i 1000000 -a 1
cargo run --  --protocol saros swap -p 2wUvdZA8ZsY714Y5wUL9fkFmupJGGwzui2N74zqJWgty -i 1000000 -a 1
cargo run --  --protocol alphaq swap -p Pi9nzTjPxD8DsRfRBGfKYzmefJoJM8TcXu2jyaQjSHm -i 1000000 -a 1
cargo run --  --protocol aquifer swap -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46 -i 1000000 -a 1

# Simulate swap transaction (builds and simulates actual transaction)
cargo run -- simulate -p FLckHLGMJy5gEoXWwcE68Nprde1D4araK4TGLw4pQq2n -i 1000000 -a 1
cargo run --  --protocol goonfi simulate -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46 -i 1000000 -a 1
cargo run --  --protocol obric simulate -p 3csCx7o3KCvpXeRZYjhCGrgXdKyjSgaqcirc5pZJUJCf -i 1000000 -a 1
cargo run --  --protocol saros simulate -p 2wUvdZA8ZsY714Y5wUL9fkFmupJGGwzui2N74zqJWgty -i 1000000 -a 1
cargo run --  --protocol aquifer simulate -i So11111111111111111111111111111111111111112 -o 7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs -a 1000000

# List all pool accounts for a protocol
cargo run -- list --protocol tessera
cargo run -- list --protocol goonfi
cargo run -- list --protocol obric
cargo run -- list --protocol saros
cargo run -- list --protocol alphaq
cargo run -- list --protocol aquifer
cargo run -- list --protocol humidifi


# Dump accounts for protocol
cargo run -- -p tessera dump -p BDqQBspbipXxnTX2kw4FPM9pzfcf9kwieGCy4yUZ9tCC

cargo run -- -p tessera curve-simulate -p BDqQBspbipXxnTX2kw4FPM9pzfcf9kwieGCy4yUZ9tCC -a 1

```

## Project Structure

```
src/
├── main.rs                 # CLI interface and main logic
├── config.rs              # Configuration management with .env support
├── constants.rs           # Common program IDs and constants
├── fetch/                 # Pool data fetching
│   ├── mod.rs            # Common interfaces for pool fetching
│   └── tessera/          # Tessera-specific implementation
│       └── mod.rs
└── swap/                  # Swap instruction building
    ├── mod.rs            # Common interfaces for swap building
    └── tessera/          # Tessera-specific implementation
        └── mod.rs
```

## Adding New Protocols

To add support for a new protocol (e.g., Orca, Raydium):

1. **Create protocol-specific modules**:

   - `src/fetch/orca/mod.rs` - Implement `PoolFetcher` and `PoolData` traits
   - `src/swap/orca/mod.rs` - Implement `SwapBuilder` trait

2. **Update main.rs** to handle the new protocol in the match statements

3. **Add protocol-specific constants** to `constants.rs` if needed

4. **Update configuration** in `config.rs` for protocol-specific settings

## Dependencies

- `solana-client`: Solana RPC client
- `solana-sdk`: Solana SDK for types
- `spl-token`: SPL Token program support
- `tokio`: Async runtime
- `clap`: Command-line argument parsing
- `anyhow`: Error handling
- `borsh`: Binary serialization
- `hex`: Hexadecimal encoding/decoding
- `dotenv`: Environment variable support
