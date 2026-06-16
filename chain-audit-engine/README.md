# ChainAudit Engine (CAE)
### *The Industrial-Grade Blockchain Audit & Compliance Infrastructure.*

**ChainAudit Engine (CAE)** is an enterprise-focused modular platform designed to ingest, decode, and transform raw blockchain data into GAAP-compliant financial records. The system is engineered for institutional transparency and is tax-reporting ready (IRS 1099-DA compliant).

---

## System Architecture

The project is built with **Rust** using a high-performance stack (**Alloy + Tokio + SQLx**). The architecture is divided into three independent components within a Cargo Workspace:

* **cae-core**: The primary engine. It manages multi-chain ingestion (ETH, Arbitrum, Base), historical backfilling, and dynamic protocol decoding.
* **cae-types**: A unified type library. It contains definitions for TransactionIntent (Swap, Bridge, Staking) and Ledger Entry models.
* **cae-api**: The analytical interface. It provides endpoints for balance calculation and data visualization on the dashboard.

---

## Key Features

* **Multi-Chain Native**: Parallel indexing of Ethereum, Arbitrum, and Base networks via independent asynchronous tasks.
* **Advanced Intent Discovery**: Deep classification of transactions: Swap, BridgeOut/In, LiquidityProvision, Staking, and SimpleTransfer.
* **Historical Backfiller**: Automated wallet history extraction for past periods before transitioning to real-time monitoring.
* **On-Chain Discovery**: Automated extraction of token metadata (Symbol, Decimals) directly from smart contracts.
* **DeFi & LP Support**: Built-in transformers for tracking positions in liquidity pools and staking contracts.
* **Audit-Ready Ledger**: A double-entry system to ensure 100% accuracy in financial reporting.

---

## Tech Stack

* **Language**: Rust (Alloy, Tokio, SQLx).
* **Database**: PostgreSQL (Metadata & Ledger Layer).
* **Infrastructure**: Docker for rapid database deployment.
* **Frontend**: Web dashboard powered by Chart.js + Tailwind CSS.

---

## Quick Start

### 1. Database Preparation
Run PostgreSQL and apply the schema from the file: docker/init/init.sql. You can use psql or any database manager by specifying the file path.

### 2. Environment Setup
Create a .env file in the project root (use .env.example as a template) and provide your RPC URLs and API keys for each network.

### 3. Running the System
Start the indexing engine and the API server. In your terminal, use the cargo run command with the --bin flag for specific modules:
- To collect data: cargo run --bin cae-core
- To start the analytical server: cargo run --bin cae-api

Once the API is running, open the file cae-api/public/index.html in your browser to access the visual dashboard.

---

## Mission Roadmap

CAE is a long-term R&D initiative to transform blockchain data into verifiable institutional assets.
* **Phase 1-2**: Foundation, fault-tolerant ingestion, and data integrity (Current Stage).
* **Phase 3-4**: AI-driven data enrichment and deep analytics of complex DeFi strategies.
* **Phase 5**: Full tax-reporting module according to IRS standards.

---

## Contact & Partnerships

**Kazbek Dzarasov**
*Founder & Senior Data Engineer*
[LinkedIn Profile](https://linkedin.com/in/kazbek-dzarasov-6769a83a)