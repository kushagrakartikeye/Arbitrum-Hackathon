# 🏷 RFID Voting System with Face Verification (Arbitrum Stylus)

A fully on-chain RFID voting system with **face verification**, powered by a **Rust smart contract on Arbitrum Stylus**, a Node.js + Python backend, a React frontend, and an ESP32 + RFID hardware setup.

This project was built for the **Arbitrum Stylus Hackathon** to showcase how **real‑world IoT devices**, **biometric verification**, and **high‑performance WASM contracts** can come together on Arbitrum.

---

## 📚 Table of Contents

- [Overview](#overview)
- [Why Arbitrum Stylus](#why-arbitrum-stylus)
- [Architecture](#architecture)
- [Smart Contract (Rust + Stylus)](#smart-contract-rust--stylus)
  - [Contract Design](#contract-design)
  - [Storage Layout](#storage-layout)
  - [Key Functions](#key-functions)
  - [Errors & Events](#errors--events)
  - [Stylus‑Specific Considerations](#stylus-specific-considerations)
- [Backend (Node.js + Python)](#backend-nodejs--python)
  - [Responsibilities](#responsibilities)
  - [Face Verification Pipeline](#face-verification-pipeline)
  - [Endpoints](#endpoints)
- [Frontend (React)](#frontend-react)
  - [Features](#features)
- [Hardware (ESP32 + MFRC522)](#hardware-esp32--mfrc522)
- [Repository Structure](#repository-structure)
- [Local Development](#local-development)
  - [1. Smart Contract Setup](#1-smart-contract-setup)
  - [2. Backend Setup](#2-backend-setup)
  - [3. Frontend Setup](#3-frontend-setup)
  - [4. ESP32 / Arduino Setup](#4-esp32--arduino-setup)
- [Deployment](#deployment)
  - [Backend Deployment](#backend-deployment)
  - [Frontend Deployment](#frontend-deployment)
- [Security Considerations](#security-considerations)
- [Future Improvements](#future-improvements)
- [License](#license)

---

## Overview

This system lets a voter:

1. Tap an **RFID tag** on an **ESP32 + MFRC522** reader.
2. Undergo **face verification** via a Python OpenCV window.
3. If verified and not previously used, their vote is **cast on‑chain** to an Arbitrum Stylus **Rust smart contract**.
4. A React dashboard displays:
   - Total votes
   - Votes per candidate (button)
   - Full vote history (tag, button, timestamp, date)
   - Winner selection and admin tools (for the contract owner)

The core innovation is that the **voting logic and state live entirely on Arbitrum** while using **Rust/WASM (Stylus)** instead of Solidity, giving much better performance and memory efficiency for more complex applications. The off‑chain components (backend, hardware, face recognition) act as secure oracles around that contract.

---

## Why Arbitrum Stylus

Arbitrum Stylus extends Arbitrum Nitro with a **WASM‑based smart contract environment** that runs alongside the EVM. Key advantages for this project:

- **Rust Smart Contracts**  
  Stylus lets contracts be written in **Rust**, compiled to **WASM**, and executed on Arbitrum while remaining fully interoperable with standard Solidity/EVM contracts. This unlocks the entire Rust ecosystem for on‑chain logic.

- **Massive Performance Gains**  
  Stylus WASM contracts are typically **10× more compute‑efficient** and offer **100–500× cheaper memory** compared to standard EVM execution. This is ideal for more complex voting logic, audit trails, and future features like on‑chain cryptography or analytics.

- **Multi‑VM Interoperability**  
  Stylus contracts can interact seamlessly with Solidity contracts on the same chain. If needed, this project could later integrate with existing DeFi/NFT infrastructure on Arbitrum using normal Solidity interfaces, while keeping the heavy logic in Rust.

- **Developer Accessibility**  
  Instead of learning Solidity from scratch, the core on‑chain logic leverages **idiomatic Rust** and the Stylus SDK. This is a more natural stack for systems and embedded developers (which fits well with the ESP32/IoT background of the project).

- **Fully Ethereum‑Secured**  
  Stylus runs inside Arbitrum’s Nitro architecture: execution disputes are verified via WASM one‑step proofs, with final settlement and security anchored to Ethereum.

This project demonstrates how Stylus can be used for **IoT + biometric + voting** workloads that would otherwise be too complex or expensive purely in Solidity.

---

## Architecture

High‑level architecture:

text
    ┌───────────────────────────────┐
    │           Frontend            │
    │         (React + Web3)        │
    │  - User dashboard             │
    │  - Owner controls             │
    │  - Connects via MetaMask      │
    └─────────────▲─────────────────┘
                  │ REST + RPC
                  │
    ┌─────────────┴─────────────────┐
    │            Backend             │
    │       (Node.js + Python)       │
    │  - Express REST API            │
    │  - ethers.js → Stylus SC       │
    │  - Face verification (Python)  │
    │  - Query & aggregate votes     │
    └─────────────▲─────────────────┘
                  │ Serial/WiFi/HTTP
                  │
    ┌─────────────┴─────────────────┐
    │            ESP32               │
    │    + MFRC522 RFID Reader      │
    │  - Reads tag ID               │
    │  - Sends tag + button input   │
    │  - UI buttons for candidates  │
    └─────────────▲─────────────────┘
                  │
    ┌─────────────┴─────────────────┐
    │      Arbitrum Stylus SC       │
    │          (Rust/WASM)          │
    │  - Stores all votes           │
    │  - One vote per tag           │
    │  - Button tallies             │
    │  - Winner computation         │
    └───────────────────────────────┘
text

---

## Smart Contract (Rust + Stylus)

### Contract Design

The contract is implemented in Rust using the **Stylus SDK** and compiled to WebAssembly. It is deployed on **Arbitrum Sepolia** at:

- **Network**: Arbitrum Sepolia
- **Address**: `0x16f7b54cb4002b5ca98a07ee44d81802e1009977`  
  (replace here if redeployed)

The main goals:

- Store each vote **on‑chain** as `(tag_id, button_number, timestamp)`.
- Enforce **one vote per RFID tag**.
- Maintain per‑button vote counts.
- Allow the owner to:
  - Reset a tag’s vote status if needed.
  - Query the winning button and its votes.

### Storage Layout

sol_storage! {
#[entrypoint]
pub struct RFIDVoting {
address owner;
StorageVec<VoteData> votes;
mapping(string => bool) has_voted;
mapping(uint256 => uint256) button_votes;
bool locked; // reentrancy guard
}

text
pub struct VoteData {
    StorageString tag_id;
    uint256 button_number;
    uint256 timestamp;
}
}

text

- `owner`: Address of the contract owner (can reset votes, transfer ownership).
- `votes`: List of all votes ever cast.
- `has_voted[tag_id]`: Prevents a tag from voting more than once.
- `button_votes[button_number]`: Total votes per candidate/button.
- `locked`: Simple reentrancy guard flag.

The use of `StorageVec` and `StorageString` is specific to Stylus’ storage SDK for Rust.

### Key Functions

**Initialize**

pub fn initialize(&mut self) -> Result<(), RFIDVotingError>

text

- Sets `owner` to `msg::sender()`.
- Unlocks the contract.
- Should be called once after deployment.

**Cast Vote**

pub fn cast_vote(&mut self, tag_id: String, button_number: U256) -> Result<(), RFIDVotingError>

text

- Checks `locked` to prevent reentrancy.
- Ensures `has_voted[tag_id] == false`, otherwise reverts with `AlreadyVoted`.
- Gets current block timestamp via `block::timestamp()`.
- Appends a new `VoteData` entry to `votes`.
- Sets `has_voted[tag_id] = true`.
- Increments `button_votes[button_number]`.
- Emits a `VoteCast` event.
- Unlocks the contract.

**Get Vote Count**

pub fn get_vote_count(&self) -> U256

text

- Returns `votes.len()` as a `U256`.

**Get Vote by Index**

pub fn get_vote(&self, index: U256) -> Result<(String, U256, U256), RFIDVotingError>

text

- Bounds checks the index.
- Returns `(tag_id, button_number, timestamp)` for the given vote.

**Pick Winner**

pub fn pick_winner(&self) -> Result<(U256, U256), RFIDVotingError>

text

- Requires at least one vote (`NoVotes` error otherwise).
- Iterates over `votes` and looks up `button_votes` to find the button with maximum votes.
- Returns `(winning_button, votes_for_that_button)`.

**Reset Vote**

pub fn reset_vote(&mut self, tag_id: String) -> Result<(), RFIDVotingError>

text

- Only callable by `owner()`.
- Sets `has_voted[tag_id] = false`.
- (Does not retroactively remove the old vote from `votes`; this is more of an “unlock” for re‑voting.)

**Owner & Ownership Transfer**

pub fn owner(&self) -> Address
pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), RFIDVotingError>

text

- Standard ownership pattern.
- Emits `OwnershipTransferred` event.

**Button Votes & Tag Check**

pub fn get_button_votes(&self, button_number: U256) -> U256
pub fn check_has_voted(&self, tag_id: String) -> bool

text

- Read‑only helper queries used heavily by the backend and frontend.

### Errors & Events

Defined via `sol!` macro for ABI compatibility:

sol! {
event VoteCast(string tag_id, uint256 button_number, uint256 timestamp);
event WinnerDeclared(uint256 winning_button, uint256 votes);
event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);

text
error AlreadyVoted(string message);
error NoVotes(string message);
error InvalidIndex(string message);
error NotOwner(string message);
error ReentrancyGuard(string message);
}

text

These provide clean, human‑readable revert reasons and indexable events for off‑chain indexing.

### Stylus‑Specific Considerations

- Uses the Stylus **`evm`, `msg`, and `block` modules** for:
  - `msg::sender()` for caller.
  - `block::timestamp()` for time.
  - `evm::log` for emitting events.
- `StorageString` and `StorageVec` are part of the Stylus SDK, optimized for WASM storage layout.
- Designed to be **EVM‑compatible at the ABI level**, so the Node.js backend can use standard `ethers.js` with a normal JSON ABI.
- Takes advantage of Stylus’ **cheap memory and fast CPU** to safely store and iterate over a growing vote history.

---

## Backend (Node.js + Python)

### Responsibilities

- Provides an **HTTP REST API** consumed by:
  - The React frontend (for dashboard/data).
  - The ESP32 (for casting votes).
- Handles:
  - MetaMask / Web3 is not used directly here; instead, backend holds a **server wallet** (private key) that signs all vote transactions.
  - ABI calls to the Arbitrum Stylus contract using **ethers.js v6**.
  - Aggregation and formatting of vote data for the frontend.

### Face Verification Pipeline

- Written in **Python** (`face_verify.py`).
- Uses:
  - `face_recognition` for embedding + comparison.
  - `opencv-python` for webcam streaming.
- Flow:
  1. Backend receives `/vote` request with `{ tagId, buttonId }`.
  2. Backend checks `/checkHasVoted(tagId)` via contract.
  3. If not voted:
     - Spawns Python process with `face_verify.py` and `tagId`.
     - Python loads reference image from `backend/faces/{tagId}.jpg`.
     - Captures live webcam frames, computes face embeddings.
     - Compares distance; if below threshold → verified.
  4. Node reads Python’s exit code / stdout to decide success.
  5. Only then calls `castVote(tagId, buttonId)` on the Stylus contract.

This design treats the Python pipeline as a **local biometric oracle**.

### Endpoints

All under `http://localhost:3000` (or your deployed URL):

- `GET /health`  
  Returns basic status and contract address.

- `POST /initialize`  
  Calls `initialize()` on the contract.

- `POST /vote`  
  Body: `{ tagId, buttonId }`  
  Runs face verification, checks tag status, and casts vote.

- `GET /votes/count`  
  Returns total vote count from `getVoteCount()`.

- `GET /votes/all`  
  Returns full vote history with decoded `(tagId, buttonNumber, timestamp, date)`.

- `GET /check/:tagId`  
  Returns whether the tag has already voted (`checkHasVoted`).

- `GET /button/:buttonNumber`  
  Returns vote count for a specific button (`getButtonVotes`).

- `GET /winner`  
  Calls `pickWinner()` on the contract and returns the winning button and votes.

- `POST /reset`  
  Body: `{ tagId }`  
  Owner‑only endpoint; calls `resetVote(tagId)`.

- `GET /owner`  
  Returns current owner address from `owner()`.

---

## Frontend (React)

### Features

- **Wallet Connection**  
  - Uses `ethers` and `window.ethereum` (MetaMask) to:
    - Display connected address.
    - Check network (Arbitrum Sepolia chain ID).
    - Show contract owner badge for the deployer.

- **Contract Initialization Flow**
  - If `owner == 0x000...0`, shows an “Initialize Contract” section.
  - First initializer becomes the contract owner.

- **Voting UI**
  - Inputs:
    - RFID Tag ID (string, e.g. `9158283` or `AC6955D3`).
    - Button number (integer, candidate ID).
  - On submit:
    - Calls backend `/check/:tagId`.
    - Initiates face verification via backend `/vote`.
    - Shows rich status messages throughout the flow.

- **Results Dashboard**
  - Total votes, active buttons, contract owner, current leader.
  - Per‑button vote breakdown.
  - Full vote history table:
    - Index
    - Tag ID
    - Button
    - Raw timestamp
    - Human‑readable date/time

- **Query Tools**
  - Check if a tag has voted.
  - Query vote count for a specific button.
  - Owner‑only: pick winner and show official winning button.

---

## Hardware (ESP32 + MFRC522)

- **ESP32** runs Arduino sketch `decentralised_elections.ino`.
- **MFRC522** RFID reader:
  - Reads card/tag UID.
  - Sends tag + button input to backend (typically via WiFi HTTP or serial bridge).
- **Buttons**:
  - Mapped to candidate IDs (1..N).
- ESP32 acts as a **trusted voting terminal** that triggers the backend `/vote` endpoint.

---

## Repository Structure

Adapt this to your actual repo layout:

Arbitrum-Hackathon/
├── decentralised_elections/
│ └── decentralised_elections.ino # ESP32 + RFID Arduino code
├── RFID-voting/
│ ├── rfid-voting-backend/
│ │ ├── index.js # Express + ethers + Stylus integration
│ │ ├── faceAuth.js # Node wrapper around Python
│ │ ├── face_verify.py # OpenCV + face_recognition
│ │ ├── abi.json # Stylus contract ABI
│ │ ├── package.json
│ │ ├── package-lock.json
│ │ ├── faces/ # Reference images (tagId.jpg)
│ │ │ └── .gitkeep
│ │ └── .env.example
│ └── rfid-voting-frontend/
│ └── researchproject/
│ ├── src/
│ │ ├── App.js # React UI
│ │ └── App.css
│ ├── public/
│ ├── package.json
│ └── README.md
└── smart-contract/ # (teammate added; path may differ)
├── src/
│ └── lib.rs # Rust Stylus contract
├── Cargo.toml
└── README.md

text

---

## Local Development

### 1. Smart Contract Setup

Requirements:
- Rust (stable)
- `cargo stylus`
- Arbitrum Stylus toolchain installed
- Wallet/private key with Arbitrum Sepolia test ETH

Typical workflow:

cd smart-contract

Optional: run checks
cargo stylus check

Build & deploy to Arbitrum Sepolia
cargo stylus deploy
--private-key YOUR_PRIVATE_KEY
--rpc-url https://arb-sepolia.g.alchemy.com/v2/YOUR_KEY

text

After deployment:
- Update `CONTRACT_ADDRESS` in `backend/.env`.
- Export ABI (`abi.json`) and copy it to backend.

### 2. Backend Setup

cd RFID-voting/rfid-voting-backend

Install Node dependencies
npm install

(Optional) Create virtual env for Python
python -m venv .venv310
.venv310\Scripts\activate
Install Python dependencies
pip install face_recognition opencv-python numpy

Create env file
cp .env.example .env

Edit .env with your values
- ALCHEMY_RPC_URL
- PRIVATE_KEY
- CONTRACT_ADDRESS
- PORT
Run backend
node index.js

text

Backend will start on `http://localhost:3000` by default.

### 3. Frontend Setup

cd RFID-voting/rfid-voting-frontend/researchproject

npm install

Optionally, set backend URL
echo REACT_APP_BACKEND_URL=http://localhost:3000 > .env
npm start

text

Visit `http://localhost:3000` in your browser (create‑react‑app dev server).

### 4. ESP32 / Arduino Setup

1. Open **Arduino IDE**.
2. Install required libraries:
   - `ESP32` board package
   - `MFRC522` for RFID
   - `WiFi` / `HTTPClient` as needed
3. Open `decentralised_elections/decentralised_elections.ino`.
4. Update:
   - WiFi SSID / password.
   - Backend URL (e.g. `http://192.168.1.X:3000/vote`).
5. Select **ESP32 Dev Module**, correct COM port.
6. Upload.

---

## Deployment

### Backend Deployment

You can deploy the backend to any Node‑friendly host (Railway, Render, etc.). At a high level:

1. Push this repo to GitHub (already done).
2. On your chosen platform:
   - Select `RFID-voting/rfid-voting-backend` as the root.
   - Set `start` script: `node index.js`.
   - Configure environment variables:
     - `ALCHEMY_RPC_URL`
     - `PRIVATE_KEY`
     - `CONTRACT_ADDRESS`
     - `PORT` (usually 3000 or as required by the host).

3. Update frontend’s `REACT_APP_BACKEND_URL` to this new URL.

> Note: Cloud environments generally cannot access a local webcam, so the **Python face verification** component is best run on a local server or a dedicated machine with camera access. For a pure cloud demo, you can optionally bypass or mock the face check.

### Frontend Deployment

**Vercel** is a great fit for the React app:

1. Import the repo `kushagrakartikeye/Arbitrum-Hackathon` into Vercel.
2. Set project root to `RFID-voting/rfid-voting-frontend/researchproject`.
3. Add environment variable:
   - `REACT_APP_BACKEND_URL=https://your-backend-host.com`
4. Deploy.

---

## Security Considerations

- **Biometric Data**  
  Face images are stored locally as static `.jpg` files under `backend/faces/`. They are **not** uploaded to GitHub (.gitignore). For production, consider encrypted storage and proper consent flows.

- **Private Keys**  
  The backend uses a server wallet to send votes. The private key is read from `.env` and must **never** be committed.

- **One Vote Per Tag Enforcement**  
  Enforced **on‑chain** via `has_voted` mapping in the Stylus contract. Even if the backend is compromised, the contract will reject duplicate votes for the same tag.

- **Reentrancy Guard**  
  `locked` boolean prevents reentrant calls to `cast_vote`, inspired by common Solidity patterns.

- **Owner‑Only Admin**  
  Only the contract owner can reset votes or transfer ownership; validated via `msg::sender()` checks.

---

## Future Improvements

- **Full On‑Chain Biometrics**  
  Explore on‑chain verification of ZK‑proofs of face embeddings using Stylus’ WASM performance.

- **More Advanced Analytics**  
  Use Stylus to run richer on‑chain tallying, statistical checks, or anomaly detection thanks to cheaper compute and memory.

- **Multi‑Election Support**  
  Extend the Rust contract to support multiple parallel elections with separate candidate sets.

- **Event Indexer**  
  Build a subgraph or custom indexer for `VoteCast` events to power more advanced dashboards.

- **Hardware Security**  
  Add secure attestation from ESP32 or integrate secure elements for tamper resistance.

---

## License

This project is licensed under the **MIT License**. See `LICENSE` for details.

---

Built with 🦀 **Rust**, ⚡ **Arbitrum Stylus**, 🧠 **Python**, 📡 **ESP32**, and ❤️ by the team for the **Arbitrum Stylus Hackathon**.
