# Redis Analyzer

Redis Analyzer is a Rust-based tool for analyzing Redis keyspace information. It connects to a Redis instance, retrieves keyspace data, and provides detailed statistics about memory usage and key counts. The results can be outputted in CSV format or printed to the console.

✨ It can be used for big redis instances to get a quick overview of it's usage. It does not require getting all keys from the Redis instance, it only collects statistics from a sample of keys and estimates the total usage.

## Features

- Connects to a Redis instance and retrieves keyspace information.
- Provides detailed statistics about memory usage and key counts.
- Outputs results in CSV format or prints them to the console.

## Installation

To install Redis Analyzer, you need to have Rust and Cargo installed on your system. You can install Rust and Cargo by following the instructions on the [official Rust website](https://www.rust-lang.org/tools/install).

Clone the repository and build the project:

```sh
git clone <repository-url>
cd redis-analyzer
cargo build --release
```

## Usage

To run Redis Analyzer, use the following command:

```sh
cargo run --release -- [OPTIONS]
```

### Options

- `<HOST>`: Redis host (default: `127.0.0.1`)
- `<PORT>`: Redis port (default: `6379`)
- `--csv`: Output results in CSV format

### Example

```sh
cargo run --release -- 192.168.1.100 6379 --csv
```

This command connects to the Redis instance at `192.168.1.100:6379` and outputs the results in CSV format.

### Understanding the results

- First column of the output is the prefix of the key. It uses spaces for grouping keys with the same prefix.
- Most columns contain information collected from the sample.
- Important are two last columns: `estimated_total_count` and `estimated_total_memory`. They are the estimated number of keys with given prefix and estimated memory usage of all keys with given prefix.

## Code Structure

- `src/main.rs`: The main entry point of the application. It handles command-line arguments, connects to Redis, retrieves keyspace information, and outputs the results.
- `src/keyspace_info.rs`: Contains definitions and implementations related to keyspace information.
- `src/prefix_map.rs`: Contains definitions and implementations related to prefix mapping.
- `src/redis.rs`: Contains definitions and implementations related to Redis connection and commands.
- `src/results.rs`: Contains definitions and implementations related to result formatting and output.
- `src/results2.rs`: Additional result-related implementations.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

For any questions or suggestions, please open an issue on GitHub.

---

Happy analyzing!
