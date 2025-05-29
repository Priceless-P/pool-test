## Setup
Clone the Repository:
```bash
    git clone https://github.com/Priceless-P/pool-test.git
    cd pool-test
```

### Configuration:
The script expects a pool address and an optional public key as command-line arguments, if public key is not specified a default one will be used.

### Running the Script
- Option 1: Run the Binary
After building with `cargo build --release`, run the binary directly from the target/release directory:
    ```bash
    ./target/release/pool-test --pool <pool-address> --pub-key <public-key>
    ```

- Option 2: Run with Cargo
Run the program directly using Cargo without manually building and using the default pubkey:
    ```bash
    cargo run -- --pool <pool-address>
    ```
### Expected Output:
The script will run a series of tests (e.g., sending random bytes, unexpected messages, invalid messages).

Console output will display test progress and results, such as:

[TEST 1] Sending random bytes before Noise handshake...

[TEST 2] Sending random bytes after Noise handshake...

Errors will be printed to the console. The script uses a 10-second timeout for connections.

