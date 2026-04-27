# GasPilot ⛽

> Find the cheapest fuel in Davao, pay on-chain, and earn XLM loyalty points — built on Stellar Soroban.

# Links

🔗 https://stellar.expert/explorer/testnet/tx/7e0f31c2a32c4b1e4319f5be1de0f4a6c7f47daa0d40f23b7d6be20bd3cd76b5
🔗 https://lab.stellar.org/r/testnet/contract/CCRC4LOZ75UW7DVZMQH6OH3LDEMPDMXTLHEEGMARMOMYW4ZLACJCFS4B

---

## Problem

Mark drives a jeepney in Davao City. Every peso per litre matters — but the only way to know which station is cheapest today is to drive around, burning the very fuel he is trying to save. Farmers transporting harvest to market and PUV operators face the same invisible tax of time and wasted fuel. There is no reliable, real-time, city-wide view of pump prices.

## Solution

GasPilot is a mobile-first Stellar dApp where participating gas stations publish their live price per litre on-chain via a Soroban smart contract. Drivers open the app, see a sorted price list of stations near them, tap to pay directly from their Stellar wallet (XLM), and automatically earn loyalty points redeemable for future discounts. Every payment is peer-to-peer: user wallet → station wallet, settled in seconds at near-zero cost — no bank required.

---

## Suggested MVP Timeline

| Week | Milestone |
|------|-----------|
| 1 | Smart contract: `register_station`, `pay_for_fuel`, `get_all_stations` |
| 2 | React Native mobile UI: price list + wallet connect (Freighter / Albedo) |
| 3 | Testnet demo: 3 mock stations, live pay flow, points display |
| 4 | Polish, README, demo video, hackathon submission |

---

## Stellar Features Used

| Feature | Purpose |
|---------|---------|
| **Soroban smart contracts** | Station registry, price storage, payment routing, loyalty points |
| **XLM transfers** | Native token used for direct fuel payments (driver → station) |
| **On-chain events** | `purchase` event emitted per transaction for off-chain indexing |
| **Custom tokens** *(optional extension)* | Station-specific reward tokens issued via trustlines |
| **Trustlines** *(optional extension)* | User wallets opt in to receive custom reward tokens |

---

## Vision & Purpose

GasPilot turns every fuel purchase into a transparent, verifiable on-chain event. For Davao's unbanked jeepney drivers and farmers, it provides:

- **Price transparency** — see who is cheapest before you drive there
- **Financial inclusion** — pay and earn without a bank account, just a Stellar wallet
- **Loyalty without gatekeeping** — points are on-chain assets, not locked in a corporate app
- **Path to DeFi** — accumulated points or custom tokens can eventually be swapped on Stellar's built-in DEX

Long term, GasPilot can integrate with local anchors (e.g. GCash ramps) so unbanked users can top up their wallets with cash at any 7-Eleven, then pay for fuel on-chain.

---

## Prerequisites

```bash
# Rust toolchain (stable + wasm32 target)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Soroban CLI (v22 or later)
cargo install --locked soroban-cli --version 22.0.0

# Verify
soroban --version   # soroban 22.x.x
cargo --version     # cargo 1.7x+
```

---

## Build

```bash
git clone https://github.com/<your-handle>/gas-pilot
cd gas-pilot

# Compile to WASM
soroban contract build
# Output: target/wasm32-unknown-unknown/release/gas_pilot.wasm
```

---

## Test

```bash
cargo test
# Runs all 5 tests in src/test.rs
# Expected output: test result: ok. 5 passed; 0 failed
```

---

## Deploy to Testnet

```bash
# 1. Configure testnet identity (one-time)
soroban keys generate --global alice --network testnet
soroban keys fund alice --network testnet

# 2. Deploy the contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/gas_pilot.wasm \
  --source alice \
  --network testnet
# Output: CONTRACT_ID (save this)

# 3. Initialise the contract
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS>
```

---

## Sample CLI Invocations

### Register a gas station (admin only)
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- register_station \
  --station_wallet GSTATION...XYZ \
  --name PetronMatina \
  --price_per_litre 500
```

### Pay for fuel as a driver
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- pay_for_fuel \
  --user GDRIVER...ABC \
  --xlm_token CABLH...TOKEN \
  --station GSTATION...XYZ \
  --litres 10
# Returns: { station, litres: 10, total_stroops: 5000, points_earned: 500, cumulative_points: 500 }
```

### View all station prices
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_all_stations
# Returns: { <station_addr>: { name, price_per_litre, wallet, active }, ... }
```

### Check a user's loyalty points
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_points \
  --user GDRIVER...ABC
# Returns: 500
```

---

## Optional Enhancement: AI Price Prediction

Integrate a lightweight AI model (on-device or via API) that:
- Fetches the last 30 days of on-chain price events per station
- Predicts tomorrow's likely price using a simple time-series model
- Shows a **"Predicted cheapest tomorrow"** banner in the app

This turns GasPilot from a price directory into a smart fuel advisor — especially valuable for farmers planning market-day logistics.

---

## Reference Repositories

- Bootcamp deployment guide: https://github.com/armlynobinguar/Stellar-Bootcamp-2026
- Full-stack example (community treasury): https://github.com/armlynobinguar/community-treasury

---

## License

MIT License — Copyright (c) 2026 GasPilot Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
