# BibliZap 

## Description

BibliZap is a free and open-source project that aims to catalog articles similar to a source article based on both upward and downward citations.

*   **Downward citations:** Correspond to the references (bibliography) of the articles.
*   **Upward citations:** Correspond to the articles that cite the source article.

The process involves exploring citations iteratively. At each level of exploration (e.g., references of the source, articles citing the references, etc.), the number of citation paths reaching each article (identified internally by Lens ID) is recorded. The final score is the sum of these path counts across all levels. Results are ranked by descending score, with ascending Lens ID resolving ties deterministically.

For a published evaluation, see Bentegeac R, Le Guellec B, Leblanc V, et al. *BibliZap: An Exploratory Evaluation of an Automated Multi-Level Citation Searching Tool for Systematic and Rapid Reviews*. Research Synthesis Methods. 2026;17(4):816–829. [https://doi.org/10.1017/rsm.2026.10079](https://doi.org/10.1017/rsm.2026.10079). For reporting guidance, see [How to report a BibliZap search](https://biblizap.org/how-it-works#how-to-report-a-biblizap-search).

Data for BibliZap is provided by The Lens, a not-for-profit service from Cambia. The Lens aggregates and harmonizes bibliographic data from various sources like Crossref, PubMed, and Microsoft Academic.

The BibliZap web-app utilizes an API access generously provided by The Lens to all its users. Users of the R package, however, require a specific individual token obtainable from The Lens for 14 days.

BibliZap operates independently and does not receive financial support from The Lens, Cambia, or any other enterprise or journal.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

*   Rust and Cargo: Ensure you have Rust and Cargo installed. You can install them via [rustup](https://rustup.rs/).

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

### Building

To build the project, navigate to the project root directory and run:

```bash
cargo build
```

This will compile the project and place the executable in the `target/debug/` directory. For a release build, use:

```bash
cargo build --release
```

The release executable will be in `target/release/`.

### Running

To run the project directly using Cargo:

```bash
cargo run
```

If you built a release version and want to run the compiled executable:

```bash
./target/release/biblizap-rs
```

[Add any command-line arguments or configuration steps required to run the application.]

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
