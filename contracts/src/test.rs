#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
        symbol_short, token, Address, Env, IntoVal, Symbol,
    };
    use crate::{GasPilot, GasPilotClient};

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Deploy the contract and a mock XLM token; return (env, client, admin, xlm_address)
    fn setup() -> (Env, GasPilotClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GasPilot);
        let client = GasPilotClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        // Deploy a mock native token so we can test XLM transfers
        let xlm_id = env.register_stellar_asset_contract(admin.clone());

        (env, client, admin, xlm_id)
    }

    // ── Test 1: Happy Path ────────────────────────────────────────────────────
    // A driver pays for fuel successfully and receives loyalty points.
    #[test]
    fn test_happy_path_pay_for_fuel() {
        let (env, client, admin, xlm_id) = setup();

        // Register a station
        let station_wallet = Address::generate(&env);
        let station_name = Symbol::new(&env, "PetronMatina");
        client.register_station(&station_wallet, &station_name, &500_u64); // 500 stroops/litre

        // Mint XLM to the driver
        let driver = Address::generate(&env);
        let token_admin = token::AdminClient::new(&env, &xlm_id);
        token_admin.mint(&driver, &100_000_i128);

        // Driver buys 10 litres → should cost 5_000 stroops
        let receipt = client.pay_for_fuel(&driver, &xlm_id, &station_wallet, &10_u64);

        assert_eq!(receipt.litres, 10);
        assert_eq!(receipt.total_stroops, 5_000);
        assert!(receipt.points_earned > 0, "should earn points");
        assert_eq!(receipt.cumulative_points, receipt.points_earned);

        // Station wallet should have received the payment
        let token_client = token::Client::new(&env, &xlm_id);
        assert_eq!(token_client.balance(&station_wallet), 5_000_i128);
    }

    // ── Test 2: Edge Case – inactive station is rejected ─────────────────────
    // Paying at a station that has been deactivated must panic.
    #[test]
    #[should_panic(expected = "station not registered")]
    fn test_edge_case_unregistered_station_rejected() {
        let (env, client, _admin, xlm_id) = setup();

        let random_station = Address::generate(&env);
        let driver = Address::generate(&env);

        // No register_station call — station does not exist in storage
        // This must panic with "station not registered"
        client.pay_for_fuel(&driver, &xlm_id, &random_station, &5_u64);
    }

    // ── Test 3: State Verification ────────────────────────────────────────────
    // After a successful purchase, contract storage must reflect the correct
    // point balance and station price.
    #[test]
    fn test_state_verification_after_purchase() {
        let (env, client, admin, xlm_id) = setup();

        let station_wallet = Address::generate(&env);
        let station_name = Symbol::new(&env, "SeaoilDavao");
        client.register_station(&station_wallet, &station_name, &600_u64); // 600 stroops/litre

        let driver = Address::generate(&env);
        let token_admin = token::AdminClient::new(&env, &xlm_id);
        token_admin.mint(&driver, &200_000_i128);

        // First purchase: 5 litres = 3_000 stroops → 300 points
        client.pay_for_fuel(&driver, &xlm_id, &station_wallet, &5_u64);
        let points_after_first = client.get_points(&driver);
        assert_eq!(points_after_first, 300_u64); // 3_000 / 10 = 300

        // Second purchase: 10 litres = 6_000 stroops → 600 more points
        client.pay_for_fuel(&driver, &xlm_id, &station_wallet, &10_u64);
        let points_after_second = client.get_points(&driver);
        assert_eq!(points_after_second, 900_u64); // 300 + 600

        // Verify station info is still intact
        let info = client.get_station(&station_wallet);
        assert_eq!(info.price_per_litre, 600_u64);
        assert!(info.active);
    }

    // ── Test 4: Price Update Reflected Immediately ────────────────────────────
    // When admin updates a station price, the next purchase uses the new price.
    #[test]
    fn test_price_update_applies_on_next_purchase() {
        let (env, client, admin, xlm_id) = setup();

        let station_wallet = Address::generate(&env);
        client.register_station(&station_wallet, &Symbol::new(&env, "CleanFuel"), &400_u64);

        // Admin updates price to 800 stroops/litre
        client.update_price(&station_wallet, &800_u64);

        let driver = Address::generate(&env);
        let token_admin = token::AdminClient::new(&env, &xlm_id);
        token_admin.mint(&driver, &200_000_i128);

        // Buy 1 litre — must cost 800 stroops (new price), not 400
        let receipt = client.pay_for_fuel(&driver, &xlm_id, &station_wallet, &1_u64);
        assert_eq!(receipt.total_stroops, 800_u64);
    }

    // ── Test 5: Multiple Stations Listed ─────────────────────────────────────
    // get_all_stations returns every registered station so the app can
    // render the cheapest-station leaderboard.
    #[test]
    fn test_all_stations_returned_in_list() {
        let (env, client, admin, _xlm_id) = setup();

        let s1 = Address::generate(&env);
        let s2 = Address::generate(&env);
        let s3 = Address::generate(&env);

        client.register_station(&s1, &Symbol::new(&env, "Petron"), &500_u64);
        client.register_station(&s2, &Symbol::new(&env, "Shell"), &520_u64);
        client.register_station(&s3, &Symbol::new(&env, "Seaoil"), &490_u64);

        let all = client.get_all_stations();
        assert_eq!(all.len(), 3);

        // Verify individual prices are stored correctly
        assert_eq!(all.get(s3.clone()).unwrap().price_per_litre, 490_u64); // cheapest
        assert_eq!(all.get(s2.clone()).unwrap().price_per_litre, 520_u64); // priciest
    }
}
