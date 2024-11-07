# Fuel HD Wallet Asset Distributor


A CLI tool to manage Fuel HD wallets, created for the Matcher for HD wallets.

## Features

- **Initial Distribution (`--init-dist`)**: Distribute ETH to all HD wallets.
- **Continual Funding (`--cont-fund`)**: Monitor and fund wallets when balances are low.
- **Reclaim Funds (`--reclaim`)**: Collect funds back to the main wallet.


## Running 
```
cargo build --release
```

Initial Funding of HD paths:
```
./target/release/fund_distributor --cont-fund
```

Continuously Monitor if Balance falls below threshold:
```
./target/release/fund_distributor --cont-fund
```

Reclaim all assets back to wallet path 0
```
./target/release/fund_distributor --cont-fund
```