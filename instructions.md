Stellar RWA Launchpad – No-Code Tokenization Platform

Overview:


Asset class targeted: music, digital artworks and royalties ( not NFT, ex)
This project implements a no-code Real-World Asset (RWA) Launchpad on the Stellar blockchain. It enables asset issuers to tokenize real-world assets through a guided web interface, handling KYC/AML, legal compliance metadata, and on-chain tokenization automatically. The platform uses Soroban smart contracts for the tokenized asset and a bonding-curve sale contract, gating token transfers to KYC-approved investors and integrating with Stellar’s native AMM for liquidity post-sale.

Key features include:

Smart Contracts (Soroban): A permissioned token contract (for the RWA) implementing Stellar’s Token Interface, with KYC allowlist gating transfers, pause/unpause, and admin-controlled clawback; and a bonding curve sale contract allowing compliant token sales (primary issuance) where token price increases linearly with each token sold. Both are deployed on Stellar Futurenet (test network).

Asset Controls: The RWA token contract enforces KYC by requiring both sender and receiver to be on an allowlist for any transfers (mirroring ERC-3643-style permissioned tokens on Ethereum). It supports Protocol 17-style clawback by admin and can pause transfers globally in case of compliance issues.

On/Off-Ramps & KYC: The demo integrates a simulated KYC flow. In a real app, this would use Stellar SEP-10 (WebAuth for user authentication) and SEP-12 (KYC info exchange) to verify investor identity. Here, a simple backend endpoint simulates KYC approval and adds the investor’s Stellar address to the on-chain allowlist. Once verified, the investor is minted a test balance of stablecoin (USDC) for demo purposes. (In production, off-chain funding via SEP-24/31 anchors would be used
dev.to
docs.freighter.app
.)

Frontend & Wallet Integration: A simple Node.js Express web UI is provided. Issuers use a “Launch an RWA” wizard to input asset details (name, symbol, etc.) and deploy the contracts. Investors use an “Invest” portal to connect their Freighter wallet (Stellar browser extension), complete KYC, and purchase tokens. The front-end integrates Freighter for key management – investors sign the buy transaction with Freighter, ensuring non-custodial control of funds.

Liquidity via AMMs: Although not fully automated in this demo, the design anticipates seeding a liquidity pool once the sale concludes (using Stellar’s native AMM pools or a Soroban AMM contract). After the bonding curve sale reaches its cap or ends, the contract could automatically deposit a portion of proceeds and tokens into a pool to provide instant secondary market liquidity
bitbond.com
. (In this code, the final step is manual/omitted for simplicity.)

Compliance by Design:
This project embeds compliance rules into the smart contracts (“compliance on-chain, by design”). The RWA token is identity-gated – only addresses approved via KYC (and thus on the allowlist) can receive or send tokens. This addresses regulatory requirements such as ensuring only accredited or eligible investors hold the security (e.g. for a U.S. Reg D 506(c) offering, only verified accredited investors would be allowlisted). The token contract allows the issuer to freeze (pause all transfers) or claw back tokens if required by court order or regulatory action
dev.to
. We include an admin multi-sig pattern placeholder (e.g. set_minter to allow a sale contract to mint tokens) – in a production setting, the issuer’s functions could be secured by a multi-signature scheme for added security. The sale contract enforces jurisdictional and investor-type restrictions indirectly via the KYC allowlist: only users who have completed the required verification can participate. (For example, U.S. investors might be verified under Reg D rules, whereas non-U.S. under Reg S – the backend could tag allowlist entries with investor type and the token contract could store jurisdiction tags in an extended version; due to time, we stick to a simple allowlist boolean.)

Proof of RWA on Stellar:
Stellar is already being used in production for tokenized assets. A notable example is Franklin Templeton’s tokenized money market fund, which has been running on Stellar since 2021
dev.to
. This provides credibility to using Stellar for real-world assets. Our platform leverages the same network capabilities that enabled Franklin Templeton’s fund to ensure compliance and interoperability.

Project Structure
stellar-rwa-launchpad/
├── contracts/
│   ├── token/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs        (RWA Token contract source – Soroban smart contract in Rust)
│   └── sale/
│       ├── Cargo.toml
│       └── src/lib.rs        (Bonding Curve Sale contract source – Soroban smart contract in Rust)
├── index.js                  (Node.js Express server for backend and web UI)
├── investor.html             (Frontend HTML/JS for the investor portal)
├── .env.example              (Example environment variables file)
└── README.md                 (Project documentation – you are here)


Token Contract (Rust – contracts/token/src/lib.rs): Implements a fungible token with compliance controls. It uses the Soroban SDK to define storage keys and methods for initialize, mint, transfer, transfer_from, etc. The initialize method sets token metadata (name, symbol, decimals) and records the issuer (admin). It also has an allowlist_flag and stores an allowlist of approved addresses. Transfers are overridden to require that both sender and receiver are allowlisted (if the allowlist feature is enabled). The contract includes set_allowlist(addr, bool) (admin-only) to update allowed investors, pause/unpause functions, and an admin-only burn (clawback) function to revoke tokens from a holder if needed. This contract is compatible with Soroban’s token interface standards, enabling wallet interoperability
dev.to
dev.to
.
Technical note: The token contract uses an enum DataKey to segregate storage for different data (balances, allowances, etc.)
jamesbachini.com
jamesbachini.com
. This pattern (using an enum as a storage key) avoids needing to load large maps – each balance or allowance is a separate entry in contract storage, similar to an Ethereum mapping
jamesbachini.com
.

Sale Contract (Rust – contracts/sale/src/lib.rs): Implements the token sale logic using a bonding curve pricing model. The initialize method links to the previously deployed token contract and the USDC contract (for payments), stores the sale parameters (base price, price slope), and sets the issuer as admin. The buy(amount) function is the core – it requires the caller (investor) to be KYC-approved (checked via the token’s is_allowed function) and not paused, then computes the cost for the requested token amount using the bonding curve formula. We implemented a linear bonding curve: price increases by a fixed increment (slope) for each token sold. The cost calculation integrates the price over the range of new tokens (current_supply -> current_supply + amount), charging slightly above the average price to account for the incremental increase (essentially summing an arithmetic series)
github.com
. We round up fractional cents to ensure sufficient payment. Next, the contract performs an atomic payment and mint: it calls the USDC contract to transfer cost USDC from the buyer to the issuer’s treasury address, and then mints the purchased amount of RWA tokens to the buyer. These cross-contract calls use Soroban’s authorized invocation framework – the buyer’s signature (collected via Freighter) authorizes the contract to transfer USDC on their behalf
dev.to
dev.to
. Finally, it updates the total_sold counter. The contract also provides a read-only get_price(amount) method so the frontend can quote the current price for a desired purchase size, and an admin function set_sale_closed() to halt the sale (prevent further buys) if needed.

Node Backend (index.js): The Express server orchestrates the contract deployment and user interactions:

Deployment Flow (Issuer): When the issuer submits the form, the backend uses the Stellar SDK to generate or load an issuer keypair (using a testnet friendbot to fund it). It then compiles (if not already) and uploads the WASM smart contracts to Stellar Futurenet. We deploy two instances of the token contract: one for the RWA asset (with allowlist enforced) and one for a USDC token (with allowlist disabled) to simulate a stablecoin. Then we deploy the sale contract, passing it the contract IDs of the RWA token and USDC token. This is done via Stellar’s native InvokeHostFunction operations and the Soroban RPC. The backend automatically calls token.set_minter(saleContract) (if needed) to authorize the sale contract to mint new tokens on the RWA token contract. On success, the contract IDs are stored and displayed to the issuer. (Under the hood, deploying a Soroban contract involves uploading the WASM code to the ledger and creating a contract instance with a unique ID derived from a hash of the WASM, the deployer’s address, and a salt
stellar.github.io
stellar.github.io
. The Node code calculates these IDs so they can be output immediately.)

KYC Flow (Investor): When an investor connects their Freighter wallet and clicks “Complete KYC”, the backend simulates KYC verification. In this demo, it immediately adds the investor’s public key to the allowlist by pretending to call the token contract’s set_allowlist(address, true). (For simplicity, the code does not actually submit this transaction to the ledger – in a real app, the issuer or an automated compliance oracle would invoke the contract method on-chain.) The backend then mints a test amount of USDC to the investor’s address (simulating that the investor deposited fiat and received USDC via an anchor). In production, this minting would be replaced by an actual anchor transfer or the user providing USDC from their wallet. After KYC, the investor’s address is now eligible to receive RWA tokens, and they have a USDC balance to invest.

Purchase Flow (Investor): The investor enters an amount of tokens to buy. The frontend queries the current price (the demo currently uses a placeholder formula) and then requests the backend to create a transaction for calling sale.buy(amount). The backend uses the Soroban RPC to simulate the transaction on the network (to obtain the necessary fees and ledger footprint)
dev.to
, then returns the unsigned transaction XDR. The investor’s Freighter wallet prompts them to sign this transaction (which includes their authorization for the contract to transfer USDC on their behalf). Once signed, the frontend sends the signed transaction back to the backend, which submits it to the network via the Soroban RPC. On success, the smart contracts execute: USDC is transferred from the investor to the issuer, and new RWA tokens are minted to the investor. The backend updates its records of the investor’s balances accordingly for display. All of this happens in a single atomic transaction – if any step fails (e.g., insufficient USDC, or the investor wasn’t actually allowlisted), the entire transaction is rolled back.

Frontend (Investor Portal – investor.html): This is a simple HTML/JavaScript page. It loads the Freighter API script and provides buttons for connecting the wallet, performing KYC, and buying tokens. Once the user connects, their Stellar address is shown. After KYC completion, their USDC balance is displayed (in this demo, an arbitrary amount is minted). The investor can then input how many tokens to buy. When they click “Buy”, the frontend calls our backend to get a transaction XDR for the purchase, then uses freighterApi.signTransaction(xdr, networkPassphrase) to sign it
docs.freighter.app
docs.freighter.app
. The signed XDR is sent back to the server for submission. Status messages (or any errors) are displayed to the user.