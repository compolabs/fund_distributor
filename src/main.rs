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
    /// Send 0.005 ETH to all 10 HD wallets from wallet 0.
    #[clap(long = "init-dist", conflicts_with = "cont_fund")]
    init_dist: bool,

    /// Monitor wallets every 20 seconds and fund if balance is below 0.005 ETH.
    #[clap(long = "cont-fund", conflicts_with = "init_dist")]
    cont_fund: bool,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Environment variables
    let mnemonic = env::var("MNEMONIC")?;
    let provider_url = env::var("PROVIDER")?;
    let eth_asset_id_str = env::var("ETH_ASSET_ID")?;

    // Parse the ETH_ASSET_ID from the environment variable
    let eth_asset_id = AssetId::from_str(&eth_asset_id_str)
        .map_err(|_| format!("Invalid ETH_ASSET_ID format: {}", eth_asset_id_str))?;

    // Connect to provider
    let provider = Provider::connect(provider_url).await?;

    // Create the main wallet (wallet 0)
    let main_wallet = WalletUnlocked::new_from_mnemonic_phrase(&mnemonic, Some(provider.clone()))?;

    println!("Wallet 0 address: {:?}", main_wallet.address());
    println!("Using AssetId: {:?}", eth_asset_id);

    if cli.init_dist {
        println!("Starting initial distribution...");
        initial_distribution(&main_wallet, &mnemonic, &provider, &eth_asset_id).await?;
    } else if cli.cont_fund {
        println!("Starting continual funding...");
        continual_funding(&main_wallet, &mnemonic, &provider, &eth_asset_id).await?;
    } else {
        println!("No valid command provided. Use --init-dist or --cont-fund.");
    }

    Ok(())
}

async fn initial_distribution(
    main_wallet: &WalletUnlocked,
    mnemonic: &str,
    provider: &Provider,
    asset_id: &AssetId,
) -> std::result::Result<(), Box<dyn Error>> {
    // Define the threshold amount (0.005 ETH in units used by Fuel)
    let threshold = 5_000_000u64; // 0.005 * 1_000_000_000 = 5,000,000

    for hd_wallet_number in 0..10 {
        // Derive the HD wallet
        let path = format!("m/44'/1179993420'/{}'/0/0", hd_wallet_number);
        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            &mnemonic,
            Some(provider.clone()),
            &path,
        )?;

        let wallet_address = wallet.address();
        println!("Wallet {} address: {:?}", hd_wallet_number, wallet_address);

        // Send threshold amount to the wallet
        send_funds(main_wallet, &wallet_address, threshold, provider, asset_id).await?;
    }

    println!("Initial distribution completed.");
    Ok(())
}

async fn continual_funding(
    main_wallet: &WalletUnlocked,
    mnemonic: &str,
    provider: &Provider,
    asset_id: &AssetId,
) -> std::result::Result<(), Box<dyn Error>> {
    // Define the threshold amount (0.005 ETH in units used by Fuel)
    let threshold = 5_000_000u64; // 0.005 * 1_000_000_000 = 5,000,000

    loop {
        for hd_wallet_number in 0..10 {
            // Derive the HD wallet
            let path = format!("m/44'/1179993420'/{}'/0/0", hd_wallet_number);
            let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
                &mnemonic,
                Some(provider.clone()),
                &path,
            )?;

            let wallet_address = wallet.address();

            // Get the balance of the wallet for the specified AssetId
            let balance = provider
                .get_asset_balance(&wallet_address, *asset_id)
                .await?;

            println!(
                "Wallet {} balance: {} (in base units)",
                hd_wallet_number, balance
            );

            // Check if balance is less than threshold
            if balance < threshold {
                println!(
                    "Wallet {} balance is less than threshold, sending funds...",
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

async fn send_funds(
    from_wallet: &WalletUnlocked,
    to_address: &Bech32Address,
    amount: u64,
    provider: &Provider,
    asset_id: &AssetId,
) -> Result<(), Box<dyn Error>> {
    // Retrieve the from_wallet's address
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
