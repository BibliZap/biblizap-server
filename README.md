# Biblizap Server

A server and frontend application for performing snowball searches on academic literature using the Lens.org API.

## Project Description

Biblizap is a tool designed to help researchers find relevant academic papers by performing "snowball" searches. Starting from seed papers identified by DOIs or PMIDs, it recursively explores their references (downward citations) and the papers that cite them (upward citations) up to a specified depth. It ranks candidate records by descending citation-path score, with ascending Lens ID as a deterministic tie-breaker.

This repository contains the backend server (built with Rust and Actix-web) and the frontend web application (built with Rust and Yew) that provides a user interface for the snowball search functionality.

## Published evaluation and reporting

Bentegeac R, Le Guellec B, Leblanc V, et al. BibliZap: An Exploratory Evaluation of an Automated Multi-Level Citation Searching Tool for Systematic and Rapid Reviews. *Research Synthesis Methods*. 2026;17(4):816–829. [https://doi.org/10.1017/rsm.2026.10079](https://doi.org/10.1017/rsm.2026.10079).

Using BibliZap in a review? See [How to report a BibliZap search](https://biblizap.org/how-it-works#how-to-report-a-biblizap-search) for copy-ready Methods and Results examples and the parameters to document.

<!-- Add the JEPH article here only after it has been published. -->

## Getting Started

### Prerequisites

- **Rust and Cargo** – Install via [rustup](https://rustup.rs):
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
    After installation, ensure Cargo is available:
    ```bash
    cargo --version
    ```

- **WASM target for Rust** – Required to compile the frontend to WebAssembly:
    ```bash
    rustup target add wasm32-unknown-unknown
    ```

- **Trunk** – Used to build the Yew frontend:
    ```bash
    cargo install trunk
    ```

- **Lens.org API key** – Required for accessing citation data:  
    [Get your API key here](https://www.lens.org/lens/user/api-key)

### Installation

1. Clone the repository:
    ```bash
    git clone https://github.com/BibliZap/biblizap-server
    ```

2. Navigate to the project directory:
    ```bash
    cd biblizap-server
    ```

3. Build both frontend and backend:
    ```bash
    ./build.sh --release
    ```

    This script will:
    - Build the Yew frontend with `trunk build --release`
    - Then compile the Actix backend with `cargo build --release` using the
      versioned SQLx offline query metadata; no build-time database is required

    When a `sqlx::query!` invocation or the database schema changes, regenerate
    the `.sqlx` metadata against an up-to-date development database with
    `cargo sqlx prepare` and commit the resulting files.

4. Optional: install system service

If you want the server installed as a systemd service and run without privileges, use the provided `install.sh` script.

Build and run the interactive installer:

```bash
./build.sh --release
sudo ./install.sh
```

The installer:

- Creates a system group and user `biblizap` (no login).
- Installs the binary at `/usr/bin/biblizap-server`.
- Prompts for the API key and both PostgreSQL URLs without echoing secrets.
- Writes `/etc/biblizap/biblizap.toml` with mode `600` and owner `biblizap`.
- Installs the checked-in systemd unit, reloads systemd, enables the service, and
  starts it.

Existing configuration is preserved during upgrades. Use
`sudo ./install.sh --reconfigure` to replace it; the installer creates a backup
first. Use `--binary PATH` if the release binary is in a different location.

To check service status after install:

```bash
systemctl status biblizap.service
journalctl -u biblizap.service -f
```

To make configuration changes edit `/etc/biblizap/biblizap.toml` (it is created with mode 600) and then restart the service:

```bash
sudo systemctl daemon-reload
sudo systemctl restart biblizap.service
```

### Configuration

The server configuration precedence is highest to lowest:

1. Command-line flags (CLI)
2. `BIBLIZAP_` environment variables
3. `./biblizap.toml`
4. `$XDG_CONFIG_HOME/biblizap/biblizap.toml`
5. `$HOME/.config/biblizap/biblizap.toml`
6. `/etc/biblizap/biblizap.toml`

Passing `--config PATH` disables file discovery and loads that explicit TOML file.
Environment variables and CLI options still override its values. Keep a TOML file
containing API keys or database URLs at mode `600`.

Configuration keys available in the TOML file:

- `bind_address` (string) — address to bind the HTTP server, e.g. `"127.0.0.1"`
- `port` (integer) — port to listen on, e.g. `35642`
- `lens_api_key` (string) — your Lens.org API key (keep file mode 600 if populated)
- `cache_backend_url` (string) — PostgreSQL URL for the Lens cache backend
- `database_url` (string) — PostgreSQL URL for tracking and corpus data
- `openalex_dump_path` (string) — optional path to an OpenAlex gzipped JSON/JSONL dump file or dump directory

Examples:

`/etc/biblizap/biblizap.toml` (generated by installer):

```toml
bind_address = "127.0.0.1"
port = 35642
lens_api_key = "REPLACE_WITH_YOUR_LENS_KEY"
cache_backend_url = "postgres://biblizap:password@localhost/biblizap_cache"
database_url = "postgres://biblizap:password@localhost/biblizap"
openalex_dump_path = "/data/openalex/works/part_000.gz"
```

The legacy `DATABASE_URL` environment variable remains supported as a fallback.
New deployments should use `database_url` in TOML or
`BIBLIZAP_DATABASE_URL`.

### Running the Server

You can run the compiled executable directly, or install it as a systemd service using `install.sh` (recommended on systems with systemd).

Run the binary directly with an explicit configuration file:

```bash
./target/release/biblizap-server --config ./biblizap.toml
```

Defaults: bind_address=127.0.0.1, port=35642. The server will listen on the
configured address and port; if you omit flags, values are discovered using the
precedence shown above and in `--help`.

### Building the OpenAlex Database

The OpenAlex database is built with a separate CLI so the web server stays focused on serving the app:

```bash
./target/release/biblizap-openalex import \
  --dump-path /data/openalex/works/part_000.gz \
  --database-url postgres://biblizap:password@localhost/biblizap
```

The importer reads `openalex_dump_path` from `biblizap.toml`. It writes to the app database from `DATABASE_URL`, unless `--database-url` is provided.

To see the help text (includes config-file locations and precedence):

```bash
./target/release/biblizap-server --help
```

Once the server is running, open the frontend in your browser at the configured address and port, for example:

```
http://127.0.0.1:35642
```

## API Documentation

The main search API is exposed as a POST endpoint at `/api`.
It expects a JSON body with the following structure:

```json
{
  "output_max_size": "100",
  "depth": 2,
  "input_id_list": ["10.1016/j.cell.2020.01.040", "32109876"],
  "search_for": "Both",
  "exclude_corpus_hashes": ["<64-character corpus hash>"]
}
```

`search_for` can also be `"References"` or `"Citations"`. The optional
`exclude_corpus_hashes` array contains hashes returned by
`POST /api/corpus/upload`; omit it or use `[]` when no exclusion lists are
needed. Use `"All"` for an unlimited `output_max_size`. The server ranks
candidate records by descending citation-path score, with ascending Lens ID
breaking ties. It then removes records whose DOIs match the exclusion lists
before applying `output_max_size`, filling from lower-ranked candidates when
needed. Exclusions do not change citation traversal or scores; records without
a matching DOI remain eligible. Fewer than the requested number may be returned
if no more candidates are available. The response is a JSON array of article
objects.

### Health check

`GET /health` checks that the web server can query both the tracking and Lens cache
PostgreSQL databases. It returns:

- HTTP `200` with `{"status":"ok"}` when both databases respond.
- HTTP `503` with `{"status":"unavailable"}` when either database fails or the
  checks take longer than three seconds.

The response deliberately excludes internal error details. Check the server logs
for the failed database and error.

## External health monitor

The `biblizap-monitor` binary is intended to run on a different server. By default,
it requests the public health URL every 30 seconds with a 10-second timeout. It
sends an SMTP outage email after two consecutive failures and a recovery email
after the endpoint becomes healthy again. Failed email deliveries are retried on
the next check; repeated health failures do not produce repeated emails once an
outage alert has been delivered.

Build it with:

```bash
cargo build --release -p biblizap-monitor
```

On the monitoring server, run the interactive installer as root:

```bash
sudo ./install-monitor.sh
```

It installs the binary at `/usr/bin/biblizap-monitor`, prompts for the health URL
and SMTP settings, writes the configuration with mode `0600`, installs the
systemd unit, enables it, and starts the service. Existing configuration is kept
during upgrades; use `sudo ./install-monitor.sh --reconfigure` to replace it.

Use `--binary PATH` when the release binary is located somewhere other than
`./target/release/biblizap-monitor`. The configuration example remains available
at `biblizap-monitor.example.toml` for manual installations.

The default SMTP mode is required STARTTLS, normally used on port 587. Set
`smtp.tls_mode = "implicit"` for implicit TLS, normally on port 465. Multiple alert
recipients can be listed in the `alerts.to` TOML array. Plaintext SMTP is not
supported.

The last successfully announced service state is stored in
`/var/lib/biblizap-monitor/state`. While the service remains down, no additional
outage emails are sent—even if the monitor restarts. A successful check after an
announced outage sends one recovery email and records the service as up again.

The monitor accepts `--config PATH`. Without it, the monitor searches the current
directory, the XDG configuration directory, and `/etc/biblizap-monitor` using the
same precedence as the server. Configuration values can be overridden with
`BIBLIZAP_MONITOR_` environment variables using `__` for nested keys—for example,
`BIBLIZAP_MONITOR_SMTP__PASSWORD`.

Monitor logs are available with:

```bash
journalctl -u biblizap-monitor.service -f
```

## Contributing

Contributions are welcome! Please check the [GitHub repository](https://github.com/BibliZap/BibliZap) for guidelines on how to contribute, report issues, or suggest features.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

For questions or support, please refer to the contact information provided in the web application's "Contact" page or open an issue on the GitHub repository.
