use clap::Parser;
use dotenv::dotenv;
use fuels::prelude::TxPolicies;
use fuels::types::bech32::Bech32Address;
use fuels::{
    accounts::{provider::Provider, wallet::WalletUnlocked, Account},
    types::AssetId,
};
use std::{env, error::Error, str::FromStr, time::Duration};
use tokio::time::sleep;

/// CLI tool for managing Fuel HD wallets.
#[derive(Parser)]
#[clap(name = "Fuel HD Wallet Manager")]
#[clap(author = "CompoLabs")]
#[clap(version = "1.0")]
#[clap(about = "Manage HD wallets using Fuel SDK", long_about = None)]
struct Cli {
    /// Send 0.005 ETH to all HD wallets from the main wallet.
    #[clap(long = "init-dist", conflicts_with_all = &["cont_fund", "reclaim"])]
    init_dist: bool,

    /// Monitor wallets every 20 seconds and fund if balance is below 0.005 ETH.
    #[clap(long = "cont-fund", conflicts_with_all = &["init_dist", "reclaim"])]
    cont_fund: bool,

    /// Reclaim all funds from HD wallets back to the main wallet.
    #[clap(long = "reclaim", conflicts_with_all = &["init_dist", "cont_fund"])]
    reclaim: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let cli = Cli::parse();

    // Environment variables
    let mnemonic =
        env::var("MNEMONIC").map_err(|_| "MNEMONIC not set in the environment".to_string())?;
    let provider_url =
        env::var("PROVIDER").map_err(|_| "PROVIDER not set in the environment".to_string())?;
    let eth_asset_id_str = env::var("ETH_ASSET_ID")
        .map_err(|_| "ETH_ASSET_ID not set in the environment".to_string())?;
    let number_of_wallets_str = env::var("NUMBER_OF_WALLETS")
        .map_err(|_| "NUMBER_OF_WALLETS not set in the environment".to_string())?;

    // Parse NUMBER_OF_WALLETS
    let number_of_wallets = number_of_wallets_str.parse::<usize>().map_err(|e| {
        format!(
            "Failed to parse NUMBER_OF_WALLETS ('{}') as a positive integer: {}",
            number_of_wallets_str, e
        )
    })?;

    if number_of_wallets == 0 {
        return Err("NUMBER_OF_WALLETS must be greater than 0".into());
    }

    // Parse the ETH_ASSET_ID from the environment variable
    let eth_asset_id = AssetId::from_str(&eth_asset_id_str)
        .map_err(|_| format!("Invalid ETH_ASSET_ID format: {}", eth_asset_id_str))?;

    // Connect to provider
    let provider = Provider::connect(&provider_url).await?;

    // Create the main wallet (wallet 0)
    let main_wallet = WalletUnlocked::new_from_mnemonic_phrase(&mnemonic, Some(provider.clone()))?;

    println!("Main Wallet address: {:?}", main_wallet.address());
    println!("Using AssetId: {:?}", eth_asset_id);
    println!("Number of HD Wallets: {}", number_of_wallets);

    if cli.init_dist {
        println!("Starting initial distribution...");
        initial_distribution(
            &main_wallet,
            &mnemonic,
            &provider,
            &eth_asset_id,
            number_of_wallets,
        )
        .await?;
    } else if cli.cont_fund {
        println!("Starting continual funding...");
        continual_funding(
            &main_wallet,
            &mnemonic,
            &provider,
            &eth_asset_id,
            number_of_wallets,
        )
        .await?;
    } else if cli.reclaim {
        println!("Starting fund reclamation...");
        reclaim_funds(
            &main_wallet,
            &mnemonic,
            &provider,
            &eth_asset_id,
            number_of_wallets,
        )
        .await?;
    } else {
        println!("No valid command provided. Use --init-dist, --cont-fund, or --reclaim.");
    }

    Ok(())
}

async fn initial_distribution(
    main_wallet: &WalletUnlocked,
    mnemonic: &str,
    provider: &Provider,
    asset_id: &AssetId,
    number_of_wallets: usize,
) -> Result<(), Box<dyn Error>> {
    // Define the amount to send (0.005 ETH in base units)
    let amount = 5_000_000u64; // Adjust based on your asset's base units

    for hd_wallet_number in 0..number_of_wallets {
        // Derive the HD wallet
        let path = format!("m/44'/1179993420'/{}'/0/0", hd_wallet_number);
        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            mnemonic,
            Some(provider.clone()),
            &path,
        )?;

        let wallet_address = wallet.address();
        println!(
            "HD Wallet {} address: {:?}",
            hd_wallet_number, wallet_address
        );

        // Send the specified amount to the wallet
        send_funds(main_wallet, &wallet_address, amount, provider, asset_id).await?;
    }

    println!("Initial distribution completed.");
    Ok(())
}

async fn continual_funding(
    main_wallet: &WalletUnlocked,
    mnemonic: &str,
    provider: &Provider,
    asset_id: &AssetId,
    number_of_wallets: usize,
) -> Result<(), Box<dyn Error>> {
    // Define the threshold amount (0.005 ETH in base units)
    let threshold = 5_000_000u64; // Adjust based on your asset's base units

    loop {
        for hd_wallet_number in 0..number_of_wallets {
            // Derive the HD wallet
            let path = format!("m/44'/1179993420'/{}'/0/0", hd_wallet_number);
            let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
                mnemonic,
                Some(provider.clone()),
                &path,
            )?;

            let wallet_address = wallet.address();

            // Get the balance of the wallet for the specified AssetId
            let balance = provider
                .get_asset_balance(&wallet_address, *asset_id)
                .await?;

            println!(
                "HD Wallet {} balance: {} (in base units)",
                hd_wallet_number, balance
            );

            // Check if balance is less than threshold
            if balance < threshold {
                println!(
                    "HD Wallet {} balance is below threshold, sending funds...",
                    hd_wallet_number
                );

                // Send threshold amount to the wallet
                send_funds(main_wallet, &wallet_address, threshold, provider, asset_id).await?;
            }
        }

        // Wait for 20 seconds before the next check
        println!("Waiting for 20 seconds before next check...");
        sleep(Duration::from_secs(20)).await;
    }
}

async fn reclaim_funds(
    main_wallet: &WalletUnlocked,
    mnemonic: &str,
    provider: &Provider,
    asset_id: &AssetId,
    number_of_wallets: usize,
) -> Result<(), Box<dyn Error>> {
    // Define the percentage of funds to reclaim (e.g., 99.9%)
    const RECLAIM_PERCENTAGE: f64 = 99.9;

    // Iterate through all HD wallets
    for hd_wallet_number in 0..number_of_wallets {
        // Derive the HD wallet
        let path = format!("m/44'/1179993420'/{}'/0/0", hd_wallet_number);
        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            mnemonic,
            Some(provider.clone()),
            &path,
        )?;

        let wallet_address = wallet.address();
        println!(
            "Reclaiming funds from HD Wallet {}: {:?}",
            hd_wallet_number, wallet_address
        );

        // Get the balance of the wallet for the specified AssetId
        let balance = provider
            .get_asset_balance(&wallet_address, *asset_id)
            .await?;

        println!(
            "HD Wallet {} balance: {} (in base units)",
            hd_wallet_number, balance
        );

        if balance > 0 {
            // Calculate the amount to reclaim (e.g., 99.9% of the balance)
            let reclaim_amount = ((balance as f64) * (RECLAIM_PERCENTAGE / 100.0)).round() as u64;

            // Ensure that reclaim_amount is greater than zero
            if reclaim_amount == 0 {
                println!(
                    "Reclaim amount for HD Wallet {} is too small to send.",
                    hd_wallet_number
                );
                continue;
            }

            println!(
                "Reclaiming {} units from HD Wallet {} to main wallet.",
                reclaim_amount, hd_wallet_number
            );

            // Send the reclaim amount back to the main wallet
            send_funds(
                &wallet,
                &main_wallet.address().into(),
                reclaim_amount,
                provider,
                asset_id,
            )
            .await?;
            println!(
                "Successfully reclaimed {} units from HD Wallet {}.",
                reclaim_amount, hd_wallet_number
            );
        } else {
            println!("HD Wallet {} has no funds to reclaim.", hd_wallet_number);
        }
    }

    println!("Fund reclamation completed.");
    Ok(())
}

async fn send_funds(
    from_wallet: &WalletUnlocked,
    to_address: &Bech32Address,
    amount: u64,
    provider: &Provider,
    asset_id: &AssetId,
) -> Result<(), Box<dyn Error>> {
    let from_address = from_wallet.address();

    // Query the balance of the specified AssetId for the from_wallet
    let balance = provider.get_asset_balance(&from_address, *asset_id).await?;

    println!(
        "Balance of AssetId {:?} for {}: {}",
        asset_id, from_address, balance
    );

    // Ensure there are sufficient funds before attempting the transfer
    if balance < amount {
        return Err(format!(
            "Insufficient funds: attempted to send {}, but balance is {}",
            amount, balance
        )
        .into());
    }

    // Perform the transfer
    let (tx_id, _receipts) = from_wallet
        .transfer(to_address, amount, *asset_id, TxPolicies::default())
        .await?;

    println!("Sent transaction: {:?}", tx_id);

    Ok(())
}
