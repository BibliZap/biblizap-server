# Biblizap Server

A server and frontend application for performing snowball searches on academic literature using the Lens.org API.

## Project Description

Biblizap is a tool designed to help researchers find relevant academic papers by performing "snowball" searches. Starting from a list of initial papers (identified by PMIDs, DOIs, or Lens IDs), it recursively explores their references (downward citations) and the papers that cite them (upward citations) up to a specified depth. The results are then scored based on how many times they appear in the search path.

This repository contains the backend server (built with Rust and Actix-web) and the frontend web application (built with Rust and Yew) that provides a user interface for the snowball search functionality.

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
    - Then compile the Actix backend with `cargo build --release`

### Configuration

The server requires a Lens.org API key. This is provided as a command-line argument when running the server.

### Running the Server

Run the compiled executable, providing your Lens.org API key and optionally a port:

```bash
cargo run --release -- --lens-api-key YOUR_LENS_API_KEY --port 8080
```

Replace `YOUR_LENS_API_KEY` with your actual key. The server will listen on the specified port (default is 8080).

Once the server is running, you can access the frontend web application by navigating to `http://127.0.0.1:8080` in your web browser.

## API Documentation

The server exposes a single POST endpoint at `/api`.
It expects a JSON body with the following structure:

```json
{
  "output_max_size": 100,
  "depth": 2,
  "input_id_list": ["10.1016/j.cell.2020.01.040", "32109876"],
  "search_for": "Both" // or "References", "Citations"
}
```

The response is a JSON array of article objects.

## Contributing

Contributions are welcome! Please check the [GitHub repository](https://github.com/BibliZap/BibliZap) for guidelines on how to contribute, report issues, or suggest features.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

For questions or support, please refer to the contact information provided in the web application's "Contact" page or open an issue on the GitHub repository.