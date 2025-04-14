// Declare modules
mod types;
mod state;
mod llm;

// Use items from modules
use types::{Difficulty, StorablePrincipal, UserProfile};
use state::{USER_PROFILES, DARE_REPOSITORY}; // Access state directly or via helper functions if defined
use llm::fetch_llm_dare; // Import the LLM interaction function

use ic_cdk::api::caller;
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use std::collections::BTreeSet; // Keep for redeem_reward logic

// --- Constants (Can also live in state.rs or a config.rs) ---
const MAX_LEADERBOARD_SIZE: usize = 20;
const REWARD_MILESTONES: &[u32] = &[3, 7, 15, 30];

// --- Initialization and Upgrades ---

#[init]
fn init() {
    // Canister initialization logic
    ic_cdk::println!("Darely Bot Canister Initialized (LLM Version - Refactored).");
    // Note: Static dare initialization is removed as get_dare now uses LLM.
    // If you add fallback logic using DARE_REPOSITORY, initialize it here.
}

#[pre_upgrade]
fn pre_upgrade() {
    // Logic to run before upgrade (stable structures handle state automatically)
    ic_cdk::println!("Running pre_upgrade...");
}

#[post_upgrade]
fn post_upgrade() {
    // Logic to run after upgrade (stable structures handle state automatically)
    ic_cdk::println!("Running post_upgrade...");
}


// --- Canister Endpoints ---

#[update]
fn register() -> Result<String, String> {
    // Registers a new user if they don't exist.
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);
    // Access state via the imported static variable
    state::USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();
        if profiles.contains_key(&storable_caller) {
            Err(String::from("You are already registered."))
        } else {
            profiles.insert(storable_caller, UserProfile::default());
            Ok(format!("Successfully registered! Welcome, Principal {}.", caller_principal))
        }
    })
}

#[query]
fn get_my_profile() -> Result<UserProfile, String> {
    // Returns the profile of the calling user.
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);
    state::USER_PROFILES.with(|profiles_ref| {
         profiles_ref.borrow().get(&storable_caller) // Get profile using storable key
             .ok_or_else(|| String::from("User not found. Please /register first."))
    })
}

// Updated get_dare endpoint calling the llm module function
#[update]
async fn get_dare(difficulty_request: Difficulty) -> Result<String, String> {
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    // 1. Check if user is registered
    if state::USER_PROFILES.with(|p| p.borrow().get(&storable_caller)).is_none() {
        return Err("User not found. Please /register first.".to_string());
    }

    // 2. Call the LLM fetching logic from the llm module
    // The fetch_llm_dare function now handles API key check, HTTPS call, and parsing
    match llm::fetch_llm_dare(difficulty_request).await {
        Ok(dare_text) => {
            // Optional: Log the generated dare?
            // state::DARE_REPOSITORY.with(|repo| repo.borrow_mut().push(&Dare{...}));
            Ok(dare_text)
        }
        Err(e) => {
            // Propagate the error from the LLM module
            Err(format!("Failed to get dare from LLM: {}", e))
        }
    }
}

// submit_dare endpoint (remains mostly the same, simplified verification)
#[update]
fn submit_dare(proof: String) -> Result<String, String> {
    if proof.trim().is_empty() { return Err("Proof cannot be empty.".to_string()); }
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    state::USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();
        if let Some(mut profile) = profiles.remove(&storable_caller) { // Use remove/insert pattern
            // NOTE: Verification logic is simplified. Cannot check against a specific dare ID.
            profile.streak += 1;
            let streak = profile.streak;
            profiles.insert(storable_caller.clone(), profile); // Re-insert updated
            Ok(format!("Dare submitted successfully! Your new streak is {}. You can now /get_dare again.", streak))
        } else {
            Err("User not found. Please /register first.".to_string())
        }
    })
}

// redeem_reward endpoint (no changes needed from previous version)
#[update]
fn redeem_reward() -> Result<String, String> {
     let caller_principal = caller();
     let storable_caller = StorablePrincipal(caller_principal);
     let mut final_message = String::new();
     let mut user_found = false;

     state::USER_PROFILES.with(|profiles_ref| {
         let mut profiles = profiles_ref.borrow_mut();
         if let Some(mut profile) = profiles.remove(&storable_caller) {
             user_found = true;
             let current_streak = profile.streak;
             let mut already_redeemed = BTreeSet::from_iter(profile.redeemed_milestones.iter().cloned());
             let mut profile_updated = false;
             let mut specific_reward_msg = String::new();

             for &milestone in REWARD_MILESTONES {
                 if current_streak >= milestone && !already_redeemed.contains(&milestone) {
                     already_redeemed.insert(milestone);
                     profile_updated = true;
                     specific_reward_msg = format!("Congratulations! You've redeemed the streak {} reward!", milestone);
                     break;
                 }
             }

             if profile_updated {
                 profile.redeemed_milestones = already_redeemed.into_iter().collect();
                 final_message = specific_reward_msg;
             } else {
                 final_message = format!("No new rewards available at your current streak of {}.", current_streak);
             }
             profiles.insert(storable_caller.clone(), profile);
         } else {
             user_found = false;
         }
     });

     if user_found { Ok(final_message) }
     else { Err("User not found. Please /register first.".to_string()) }
}

// get_leaderboard endpoint (no changes needed from previous version)
#[query]
fn get_leaderboard() -> Vec<(candid::Principal, u32)> { // Ensure return type uses candid::Principal
    let mut leaderboard: Vec<(candid::Principal, u32)> = state::USER_PROFILES.with(|profiles_ref| {
        profiles_ref.borrow().iter()
            .map(|(storable_principal, profile)| (storable_principal.0, profile.streak)) // Extract raw Principal
            .collect()
    });
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
    leaderboard.truncate(MAX_LEADERBOARD_SIZE);
    leaderboard
}


// --- Candid Export ---
// This should remain in lib.rs to export the public interface
ic_cdk::export_candid!();