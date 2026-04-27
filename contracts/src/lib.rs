#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, Map, Symbol, Vec, token,
};

// ─────────────────────────────────────────────
// Storage key types
// ─────────────────────────────────────────────

/// Key variants used in persistent contract storage
#[contracttype]
pub enum DataKey {
    /// Maps a station Address → StationInfo
    Station(Address),
    /// Maps a user Address → accumulated loyalty points (u64)
    Points(Address),
    /// Stores the contract admin address
    Admin,
    /// Running list of registered station addresses
    StationList,
}

// ─────────────────────────────────────────────
// Data structures
// ─────────────────────────────────────────────

/// On-chain record for a participating gas station
#[contracttype]
#[derive(Clone)]
pub struct StationInfo {
    /// Human-readable station name (e.g. "Petron Matina")
    pub name: Symbol,
    /// Price per litre in stroops (1 XLM = 10_000_000 stroops)
    pub price_per_litre: u64,
    /// Station's wallet address for direct payment
    pub wallet: Address,
    /// Whether the station is currently active / accepting payments
    pub active: bool,
}

/// Result returned to the caller after a fuel purchase
#[contracttype]
#[derive(Clone)]
pub struct PurchaseReceipt {
    pub station: Address,
    pub litres: u64,
    pub total_stroops: u64,
    pub points_earned: u64,
    pub cumulative_points: u64,
}

// ─────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────

#[contract]
pub struct GasPilot;

#[contractimpl]
impl GasPilot {
    // ── Admin / Setup ────────────────────────

    /// Initialise the contract and set the administrator.
    /// Must be called once immediately after deployment.
    pub fn initialize(env: Env, admin: Address) {
        // Prevent re-initialisation
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialised");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        // Bootstrap an empty station list
        let empty: Vec<Address> = Vec::new(&env);
        env.storage().persistent().set(&DataKey::StationList, &empty);
    }

    // ── Station management ───────────────────

    /// Register a new gas station or update its fuel price.
    /// Only the admin may call this — in a real deployment the
    /// admin would be a multisig or governance contract.
    pub fn register_station(
        env: Env,
        station_wallet: Address,
        name: Symbol,
        price_per_litre: u64, // in stroops per litre
    ) {
        // Require admin authorisation
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let info = StationInfo {
            name,
            price_per_litre,
            wallet: station_wallet.clone(),
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Station(station_wallet.clone()), &info);

        // Append to the global station list if not already present
        let mut list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::StationList)
            .unwrap_or(Vec::new(&env));

        let mut found = false;
        for addr in list.iter() {
            if addr == station_wallet {
                found = true;
                break;
            }
        }
        if !found {
            list.push_back(station_wallet);
            env.storage().persistent().set(&DataKey::StationList, &list);
        }
    }

    /// Update the price of an existing station.
    /// Useful when daily pump prices change.
    pub fn update_price(env: Env, station_wallet: Address, new_price: u64) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut info: StationInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Station(station_wallet.clone()))
            .expect("station not found");

        info.price_per_litre = new_price;
        env.storage()
            .persistent()
            .set(&DataKey::Station(station_wallet), &info);
    }

    // ── Core MVP: Pay & Earn ─────────────────

    /// A driver pays for fuel at a specific station.
    ///
    /// Flow:
    ///   1. User approves the contract to spend XLM (done client-side via token::approve).
    ///   2. User calls pay_for_fuel().
    ///   3. Contract pulls `total_cost` XLM from user → station wallet.
    ///   4. Contract awards loyalty points (1 point per 10 stroops spent).
    ///   5. Returns a PurchaseReceipt so the dApp can display confirmation.
    ///
    /// `xlm_token`  – address of the native XLM token contract on testnet/mainnet.
    /// `station`    – address of the gas station being paid.
    /// `litres`     – number of litres being purchased (integer, no decimals).
    pub fn pay_for_fuel(
        env: Env,
        user: Address,
        xlm_token: Address,
        station: Address,
        litres: u64,
    ) -> PurchaseReceipt {
        // Require the user's signature so nobody can pay on their behalf
        user.require_auth();

        // Load station info and ensure it is active
        let info: StationInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Station(station.clone()))
            .expect("station not registered");

        if !info.active {
            panic!("station is not active");
        }

        // Calculate total cost in stroops
        let total_cost: u64 = info.price_per_litre * litres;

        // Transfer XLM from user wallet → station wallet
        let token_client = token::Client::new(&env, &xlm_token);
        token_client.transfer(&user, &info.wallet, &(total_cost as i128));

        // Award loyalty points: 1 point per 10 stroops (tune as needed)
        let points_earned: u64 = total_cost / 10;

        let prev_points: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Points(user.clone()))
            .unwrap_or(0u64);

        let new_points = prev_points + points_earned;
        env.storage()
            .persistent()
            .set(&DataKey::Points(user.clone()), &new_points);

        // Emit a purchase event so off-chain indexers / the mobile app can react
        env.events().publish(
            (symbol_short!("purchase"), user.clone()),
            (station.clone(), litres, total_cost, points_earned),
        );

        PurchaseReceipt {
            station,
            litres,
            total_stroops: total_cost,
            points_earned,
            cumulative_points: new_points,
        }
    }

    // ── Read-only queries ────────────────────

    /// Return all registered stations so the mobile app can display a
    /// sorted price list — this is the "cheapest station near me" feed.
    pub fn get_all_stations(env: Env) -> Map<Address, StationInfo> {
        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::StationList)
            .unwrap_or(Vec::new(&env));

        let mut result: Map<Address, StationInfo> = Map::new(&env);
        for addr in list.iter() {
            if let Some(info) = env
                .storage()
                .persistent()
                .get(&DataKey::Station(addr.clone()))
            {
                result.set(addr, info);
            }
        }
        result
    }

    /// Get a single station's current price and metadata
    pub fn get_station(env: Env, station: Address) -> StationInfo {
        env.storage()
            .persistent()
            .get(&DataKey::Station(station))
            .expect("station not found")
    }

    /// Return the accumulated loyalty points for a user wallet
    pub fn get_points(env: Env, user: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::Points(user))
            .unwrap_or(0u64)
    }
}

#[cfg(test)]
mod test;
