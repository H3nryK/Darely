# Darely Bot - On-Chain Challenge Bot for OpenChat

Darely Bot is an interactive challenge system designed to run as an on-chain application on the Internet Computer (ICP). It integrates with OpenChat (via a separate bot frontend) to provide users with dynamically generated dares, track their completion streaks, offer rewards, and maintain a leaderboard.

This repository contains the **backend canister code** written in Rust.

## Features

* **User Registration:** Users can register with the bot.
* **Dynamic Dare Generation:** Leverages external Large Language Models (LLMs) via HTTPS Outcalls (currently configured for OpenAI's API) to generate unique dares based on difficulty (Easy, Medium, Hard).
* **Dare Submission:** Users can submit proof of dare completion (basic submission tracking implemented).
* **Streak Tracking:** Tracks consecutive dare completions for each user.
* **Streak Rewards:** Users can redeem rewards upon reaching predefined streak milestones.
* **Leaderboard:** Displays top users based on their current streak.
* **On-Chain & Persistent:** All user data (profiles, streaks, redeemed rewards) is stored securely on-chain using ICP's stable memory structures.

## Technology Stack

* **Platform:** Internet Computer (ICP)
* **Language:** Rust
* **ICP SDKs/Libraries:**
    * `ic-cdk`: Canister Development Kit for Rust
    * `ic-stable-structures`: For persistent data storage across upgrades
    * `candid`: For data serialization between canister and external callers
    * `serde`, `serde_json`: For JSON serialization/deserialization (used for LLM API interaction)
* **External Services:**
    * OpenAI API (or other compatible LLM): For dynamic dare generation via HTTPS Outcalls.
* **Target Frontend:** OpenChat (Requires a separate OpenChat bot integration layer)

## Project Structure

The backend canister code is organized into several Rust modules within the `src/` directory:

* `lib.rs`: Main canister entry point, lifecycle hooks (`init`, `pre_upgrade`, `post_upgrade`), endpoint definitions (`#[update]`, `#[query]`), and module declarations.
* `types.rs`: Core data structure definitions (`UserProfile`, `Difficulty`, API structs, etc.) and `Storable` implementations.
* `state.rs`: Stable memory management and state variable definitions (`USER_PROFILES`, `MEMORY_MANAGER`, etc.).
* `llm.rs`: Logic for handling HTTPS Outcalls to the external LLM API (request building, API key handling, response parsing).

## Prerequisites

* **DFINITY Canister SDK (`dfx`):** [Install Guide](https://internetcomputer.org/docs/current/developer-docs/setup/install/)
* **Rust:** [Install Guide](https://www.rust-lang.org/tools/install)
* **Rust Wasm Target:** `rustup target add wasm32-unknown-unknown`
* **OpenAI API Key:** You need an API key from OpenAI to use the LLM dare generation feature.

## Configuration

### OpenAI API Key (CRITICAL)

The canister needs access to your OpenAI API key to generate dares.

**WARNING:** The current code in `src/llm.rs` contains a **placeholder function `get_openai_api_key()` which is INSECURE for production.**

* **DO NOT hardcode your real API key directly into the source code.**
* **DO NOT commit your real API key to version control (e.g., Git).**

**Before deployment (especially to mainnet), you MUST replace the placeholder function with a secure method for managing your API key.** Options include:
    * Using environment variables during local development/testing ONLY (e.g., via build scripts).
    * Implementing secure storage within the canister (e.g., encrypting the key and storing it in stable memory with strict access controls).
    * Utilizing future ICP platform features for secrets management if available.

### Cycles for HTTPS Outcalls

The constant `HTTP_REQUEST_CYCLES` in `src/llm.rs` defines the estimated cycles attached to each HTTPS Outcall. This value needs tuning based on:
* The complexity of the LLM request/response.
* The size of the ICP subnet the canister runs on.
* Current network conditions and cycle pricing.

Start with the provided value (e.g., 70 Billion) and monitor your canister's cycle balance during testing, adjusting as needed. Insufficient cycles will cause outcalls to fail.

## Running Locally

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/H3nryK/Darely
    cd Darely
    ```
2.  **Configure API Key (Locally - Insecure Example):** For local testing *only*, you might temporarily modify `src/llm.rs` to read from an environment variable or directly insert your key (remembering this is insecure).
    *Example (using env var, modify `get_openai_api_key`):*
    ```rust
    // Inside get_openai_api_key() in src/llm.rs - FOR LOCAL TESTING ONLY
    match std::env::var("OPENAI_API_KEY") {
        Ok(key) => Ok(key),
        Err(_) => Err("Error: OPENAI_API_KEY environment variable not set.".to_string()),
    }
    ```
    *Then run `dfx deploy` like:* `OPENAI_API_KEY="sk-..." dfx deploy darely_bot_backend`

3.  **Start the local replica:**
    ```bash
    dfx start --clean --background
    ```
4.  **Deploy the canister:**
    ```bash
    # Make sure to handle the API key securely or via env var for local test
    dfx deploy darely_bot_backend
    ```
    *(Note: Replace `darely_bot_backend` if your canister name in `dfx.json` is different)*

## Usage / Testing Locally

Use `dfx canister call` to interact with the deployed canister. Replace `darely_bot_backend` if needed.

* **Register a user (yourself):**
    ```bash
    dfx canister call darely_bot_backend register
    ```
* **Get your profile:**
    ```bash
    dfx canister call darely_bot_backend get_my_profile
    ```
* **Get an Easy Dare (requires configured API key):**
    ```bash
    dfx canister call darely_bot_backend get_dare '(variant { Easy })'
    ```
* **Get a Medium Dare:**
    ```bash
    dfx canister call darely_bot_backend get_dare '(variant { Medium })'
    ```
* **Get a Hard Dare:**
    ```bash
    dfx canister call darely_bot_backend get_dare '(variant { Hard })'
    ```
* **Submit Dare Completion:**
    ```bash
    dfx canister call darely_bot_backend submit_dare '("I finished the dare!")'
    ```
* **Redeem Reward (if streak milestone met):**
    ```bash
    dfx canister call darely_bot_backend redeem_reward
    ```
* **View Leaderboard:**
    ```bash
    dfx canister call darely_bot_backend get_leaderboard
    ```

## Deployment to ICP Mainnet

1.  **Ensure Secure API Key:** Implement a secure method for your API key in `src/llm.rs`.
2.  **Acquire Cycles:** Ensure your canister principal has sufficient cycles to cover deployment and runtime costs (especially HTTPS outcalls).
3.  **Deploy:**
    ```bash
    dfx deploy --network ic darely_bot_backend
    ```
4.  **Monitor:** Keep an eye on canister cycle balance and logs.

## OpenChat Integration

This canister provides the backend logic. To use it within OpenChat:

1.  You need to develop a separate **OpenChat bot frontend**.
2.  This frontend bot will run within OpenChat's infrastructure.
3.  Configure the frontend bot with the `canister_id` of your deployed `darely_bot_backend` canister (on mainnet).
4.  The frontend bot will parse user commands (e.g., `/dare easy`) in OpenChat.
5.  It will then make inter-canister calls to the corresponding methods on your backend canister (e.g., calling `get_dare(variant { Easy })`).
6.  It will receive the results from the backend and format them as messages back into the OpenChat channel.
7.  Refer to the **OpenChat developer documentation** for specifics on building and configuring bots on their platform.

## Contributing

First off, thank you for considering contributing to Darely Bot! We welcome contributions from the community. Whether it's reporting a bug, discussing improvements, or submitting code changes, your help is appreciated.

### Reporting Bugs

* If you find a bug, please ensure it hasn't already been reported by searching the project's issue tracker (e.g., GitHub Issues, if applicable).
* If it's a new bug, please open an issue providing a clear title and description, steps to reproduce the bug, and details about your environment (dfx version, OS, etc.).

### Suggesting Enhancements

* If you have an idea for a new feature or an improvement to an existing one, please open an issue first to discuss it. This helps ensure it aligns with the project's goals before significant work is done.
* Provide a clear description of the enhancement and why it would be beneficial.

### Pull Requests (Code Contributions)

We follow a standard workflow for code contributions:

1.  **Fork the Repository:** If working on a platform like GitHub, fork the repository to your own account.
2.  **Create a Branch:** Create a new branch from the `main` (or `develop`) branch for your changes (e.g., `git checkout -b feature/new-dare-logic` or `git checkout -b fix/streak-bug`). Use a descriptive branch name.
3.  **Make Changes:** Write your code, ensuring you adhere to the coding standards.
4.  **Test Your Changes:**
    * Add unit tests for any new logic where applicable.
    * Test the canister locally using `dfx deploy` and `dfx canister call` to ensure your changes work as expected and don't break existing functionality.
5.  **Format and Lint:**
    * Format your Rust code using `cargo fmt`.
    * Check your code for common issues using `cargo clippy`. Address any warnings or errors reported.
6.  **Commit Changes:** Use clear and descriptive commit messages.
7.  **Push Changes:** Push your branch to your fork.
8.  **Submit a Pull Request (PR):** Open a Pull Request from your branch to the main project repository's `main` (or `develop`) branch.
    * Provide a clear title and description for your PR, explaining the changes and linking to any relevant issues.
9.  **Code Review:** Project maintainers will review your PR. Address any feedback or requested changes.
10. **Merge:** Once approved, your PR will be merged.

## Development Setup

Please refer to the main `README.md` file for instructions on setting up the development environment (installing `dfx`, Rust, etc.).

## Coding Standards

* **Language:** Rust
* **Formatting:** Use `cargo fmt` to automatically format the code according to standard Rust conventions.
* **Linting:** Use `cargo clippy` to catch common mistakes and improve code quality. Address lints before submitting a PR.
* **Comments:** Write clear and concise comments to explain complex logic, public APIs, and important decisions.
* **Canister Development:** Follow best practices for Internet Computer canister development (e.g., efficient state management, cycle awareness, security considerations).

## Testing

Contributions should ideally include tests. While comprehensive testing infrastructure might still be under development, aim to:

* Write unit tests for helper functions and non-trivial logic.
* Manually test canister endpoints thoroughly using `dfx canister call` on a local replica.
* Consider integration testing approaches if applicable (e.g., using PocketIC).

## Code of Conduct

While we may not have a formal Code of Conduct document yet, all contributors are expected to interact respectfully and constructively. Please be welcoming and considerate towards others in all communications (issue discussions, code reviews, etc.). Harassment or exclusionary behavior will not be tolerated.

## Questions?

If you have questions about contributing or the project in general, feel free to open an issue on the project's issue tracker.

Thank you for contributing!