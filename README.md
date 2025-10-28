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

# Build and submit swap transaction
cargo run -- swap -p FLckHLGMJy5gEoXWwcE68Nprde1D4araK4TGLw4pQq2n -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000
cargo run --  --protocol goonfi swap -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46 -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000
cargo run --  --protocol obric swap -p 3csCx7o3KCvpXeRZYjhCGrgXdKyjSgaqcirc5pZJUJCf -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000

# Simulate swap transaction (builds and simulates actual transaction)
cargo run -- simulate -p FLckHLGMJy5gEoXWwcE68Nprde1D4araK4TGLw4pQq2n -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000
cargo run --  --protocol goonfi simulate -p 4ynTYgJK5ruYx3AZMRjCHrJk1qkm61fePF7dkbvRQD46 -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000
cargo run --  --protocol obric simulate -p 3csCx7o3KCvpXeRZYjhCGrgXdKyjSgaqcirc5pZJUJCf -i EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v -a 1000000

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
