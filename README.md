# Escrow Program

A Solana escrow program built with Anchor and Rust. The program allows a maker to deposit token A into a program-controlled vault and request a specific amount of token B from a taker.

When a taker accepts the offer:

1. The taker sends token B to the maker.
2. The vault sends token A to the taker.
3. The escrow account and vault are closed.
4. Account rent is returned to the maker.

The program ID is:

```text
5Y6HMSgNYbkcBiQCukYvTK56aQarSpq1Nk9aiSsjws2o
```

## Requirements

Install the following tools before building the project:

- Rust toolchain 1.89.0 or a compatible toolchain
- Solana CLI
- Anchor CLI
- Cargo

Check the installed versions:

```bash
rustc --version
cargo --version
solana --version
anchor --version
```

The Rust version used by this workspace is declared in `Cargo.toml` and `rust-toolchain.toml`.

## Project Structure

```text
.
├── Anchor.toml
├── Cargo.toml
├── programs/
│   └── escrowq32026/
│       ├── src/
│       │   ├── instructions/
│       │   ├── state.rs
│       │   ├── error.rs
│       │   └── lib.rs
│       └── tests/
│           └── mod.rs
└── rust-toolchain.toml
```

Important files:

- `programs/escrowq32026/src/lib.rs`: Program entrypoints and instruction dispatch.
- `programs/escrowq32026/src/state.rs`: Escrow account data.
- `programs/escrowq32026/src/instructions/make.rs`: Creates an escrow and deposits token A.
- `programs/escrowq32026/src/instructions/take.rs`: Completes the swap and closes the escrow.
- `programs/escrowq32026/src/instructions/refund.rs`: Returns token A to the maker.
- `programs/escrowq32026/src/instructions/update.rs`: Updates the maker's requested amount and expiration.
- `programs/escrowq32026/tests/mod.rs`: LiteSVM integration test for all instructions.

## Build the Program

From the repository root, run:

```bash
anchor build
```

This compiles the program and creates the deployable shared object under `target/deploy/`.

For a faster Rust-only validation, run:

```bash
cargo check -p escrowq32026
```

## Run the Tests

The integration test uses LiteSVM, creates test mints and token accounts, and runs the complete escrow lifecycle.

Run the test with transaction logs enabled:

```bash
cargo test -p escrowq32026 --test mod -- --nocapture
```

The test performs these operations:

1. Creates token A and token B.
2. Funds the maker with token A.
3. Funds the taker with token B.
4. Creates an escrow with `make`.
5. Changes the requested token B amount with `update`.
6. Completes the swap with `take`.
7. Creates a second escrow.
8. Returns its token A deposit with `refund`.
9. Verifies token balances and escrow/vault account closure.

The `--nocapture` option displays signatures, compute-unit usage, and progress logs.

You can also run all package tests with:

```bash
cargo test -p escrowq32026
```

If the test reports that `target/deploy/escrowq32026.so` is missing, build the program first:

```bash
anchor build
```

## Escrow Instructions

### `make`

Creates an escrow PDA and deposits token A into its associated token account.

Parameters:

- `seed`: Unique maker-selected value used in the escrow PDA.
- `deposit`: Amount of token A deposited into the vault, in the token's base units.
- `receive`: Amount of token B requested from the taker, in base units.
- `expiration`: Expiration timestamp stored in the escrow account.

The escrow PDA is derived from:

```text
["escrow", maker_public_key, seed.to_le_bytes()]
```

The vault is the associated token account for token A owned by the escrow PDA.

### `update`

Allows the maker to update:

- The requested token B amount (`receive`)
- The stored expiration timestamp (`expiration`)

The maker must sign the instruction, and the escrow PDA must match the supplied seed.

### `take`

Allows a taker to complete the exchange. The taker must provide token B and sign the transaction.

The instruction:

1. Creates the taker's token A account if needed.
2. Creates the maker's token B account if needed, using the taker as payer.
3. Transfers the requested token B amount to the maker.
4. Transfers the vault's token A balance to the taker.
5. Closes the vault and escrow accounts.

### `refund`

Allows the maker to reclaim the vault's token A balance. The vault and escrow accounts are closed after the transfer.

## Account State

Each escrow stores:

- `seed`: PDA seed.
- `maker`: Maker public key.
- `mint_a`: Token deposited by the maker.
- `mint_b`: Token requested from the taker.
- `receive`: Requested token B amount.
- `bump`: PDA bump seed.
- `expiration`: Stored expiration timestamp.

Amounts are expressed in base units. For a mint with six decimals, `10_000_000` represents `10` whole tokens.

## Local Development

To use a local Solana wallet, configure the wallet path in `Anchor.toml` or set it with:

```bash
solana config set --url localhost
solana config set --keypair ~/.config/solana/id.json
```

Check the active configuration:

```bash
solana config get
```

The included integration test does not require a running validator because it uses LiteSVM. The project configuration has `skip_local_validator = true`.