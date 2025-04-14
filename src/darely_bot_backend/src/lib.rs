use candid::{CandidType, Principal, Decode, Encode};
use ic_cdk::api::{caller, time};
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{
    storable::Bound, BTreeMap as StableBTreeMap, DefaultMemoryImpl, Storable, StableVec
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet; // HashMap can still be useful for temporary operations

// --- Configuration & Constants ---
const MAX_LEADERBOARD_SIZE: usize = 20;
const REWARD_MILESTONES: &[u32] = &[4, 14, 22, 29]; // Example streak milestones

// --- Memory Management ---
type Memory = VirtualMemory<DefaultMemoryImpl>;

// Define Memory IDs for different stable structures
const USER_PROFILES_MEM_ID: MemoryId = MemoryId::new(0);
const DARES_MEM_ID: MemoryId = MemoryId::new(1);

thread_local! {
    // The memory manager is used to allocate virtual memory for stable structures.
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // --- Stable State ---
    static USER_PROFILES: RefCell<StableBTreeMap<StorablePrincipal, UserProfile, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(USER_PROFILES_MEM_ID)), // Get memory region
        )
    );

    // Dare Repository: A stable vector of Dares
    static DARE_REPOSITORY: RefCell<StableVec<Dare, Memory>> = RefCell::new(
        StableVec::init(
             MEMORY_MANAGER.with(|m| m.borrow().get(DARES_MEM_ID)), // Get memory region
        ).expect("Failed to initialize stable dare repository") // Use expect for init errors
    );
}

// --- Data Structures ---
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct StorablePrincipal(Principal);

impl Storable for StorablePrincipal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(&self.0).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { StorablePrincipal(Decode!(bytes.as_ref(), Principal).unwrap()) }
    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Storable for Difficulty {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
    const BOUND: Bound = Bound::Bounded { max_size: 10, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct Dare {
    id: u64, // Use u64 for stable vec index
    text: String,
    difficulty: Difficulty,
}

impl Storable for Dare {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
struct UserProfile {
    streak: u32,
    current_dare_id: Option<u64>,
    redeemed_milestones: Vec<u32>,
}

impl Storable for UserProfile {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
   fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
    const BOUND: Bound = Bound::Bounded { max_size: 128, is_fixed_size: false };
}

// --- Initialization and Upgrades ---

#[init]
fn init() {
    // Step 1: Check if empty using an immutable borrow within its own scope
    let is_empty = DARE_REPOSITORY.with(|repo_ref| {
        repo_ref.borrow().is_empty()
    });

    // Step 2: If it was empty, perform the mutable operations in a new scope
    if is_empty {
        ic_cdk::println!("Dare Repository is empty. Initializing...");
        DARE_REPOSITORY.with(|repo_ref| {
            // Okay to borrow mutably now, no other borrows are active in this specific scope
            let mut repo_mut = repo_ref.borrow_mut();
            let sample_dares = vec![
                Dare { id: 0, text: "Do 10 jumping jacks.".to_string(), difficulty: Difficulty::Easy },
                Dare { id: 1, text: "Share a helpful tip in the main chat.".to_string(), difficulty: Difficulty::Easy },
                Dare { id: 2, text: "Write a short poem (4 lines).".to_string(), difficulty: Difficulty::Medium },
                Dare { id: 3, text: "Learn 5 basic words in a new language.".to_string(), difficulty: Difficulty::Medium },
                Dare { id: 4, text: "Solve a Sudoku puzzle.".to_string(), difficulty: Difficulty::Hard },
                Dare { id: 5, text: "Briefly explain a complex topic simply.".to_string(), difficulty: Difficulty::Hard },
            ];
            for dare in sample_dares {
                // Use if let Err to handle potential errors during push more gracefully than expect
                if let Err(e) = repo_mut.push(&dare) {
                     // Trap explicitly if adding initial dares fails - this is a setup problem.
                     ic_cdk::trap(&format!("FATAL: Failed to add initial dare: {:?}", e));
                }
            }
            ic_cdk::println!("Initialized Dare Repository with {} dares.", repo_mut.len());
        });
    } else {
        // Optional: Log the length if not empty (requires another borrow)
        DARE_REPOSITORY.with(|repo_ref| {
             ic_cdk::println!("Dare Repository already initialized with {} dares.", repo_ref.borrow().len());
        });
    }
}

// Pre-upgrade hook (required for stable structures, though often empty if using MemoryManager well)
#[pre_upgrade]
fn pre_upgrade() {
     ic_cdk::println!("Running pre_upgrade...");
     // No explicit save needed for stable structures managed by MEMORY_MANAGER
}

// Post-upgrade hook (required for stable structures)
#[post_upgrade]
fn post_upgrade() {
     ic_cdk::println!("Running post_upgrade...");
    // State is automatically restored via MEMORY_MANAGER and stable structure initialization
    // Can add verification logic here if needed
    DARE_REPOSITORY.with(|repo_ref| {
         ic_cdk::println!("Dare Repository contains {} dares after upgrade.", repo_ref.borrow().len());
    });
    USER_PROFILES.with(|profiles_ref| {
        ic_cdk::println!("User Profiles map contains {} users after upgrade.", profiles_ref.borrow().len());
   });
}

// --- Helper Functions ---

// Simple pseudo-random index selection using time (INSECURE)
fn get_pseudo_random_u64(max_exclusive: u64) -> u64 {
    if max_exclusive == 0 {
        return 0;
    }
    let timestamp_nanos = time();
    timestamp_nanos % max_exclusive
}

// --- Canister Endpoints ---

// Register User
#[update]
fn register() -> Result<String, String> {
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();
        if profiles.contains_key(&storable_caller) {
            Err(String::from("You are already registered."))
        } else {
            // Insert default profile using the StorablePrincipal wrapper
            profiles.insert(storable_caller, UserProfile::default());
            Ok(format!("Successfully registered! Welcome, Principal {}.", caller_principal))
        }
    })
}

// Get User's Own Profile
#[query]
fn get_my_profile() -> Result<UserProfile, String> {
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    USER_PROFILES.with(|profiles_ref| {
         profiles_ref.borrow()
             .get(&storable_caller) // Use the wrapper key
             .ok_or_else(|| String::from("User not found. Please /register first."))
    })
}

// Get a Dare based on Difficulty
#[update] // Modifies user's current_dare_id
fn get_dare(difficulty_request: Difficulty) -> Result<String, String> {
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    // --- Step 1: Update User Profile (check if registered, check current dare) ---
    let profile_update_result = USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();
        // Get mutable profile, using the wrapper key
        if let Some(user_profile) = profiles.remove(&storable_caller) { // Temporarily remove to update
            // Check if user already has an active dare
            if user_profile.current_dare_id.is_some() {
                 // Re-insert profile before returning error
                 profiles.insert(storable_caller.clone(), user_profile);
                 return Err("HAS_ACTIVE_DARE".to_string()); // Use code for easier handling later
            }
             // Ok to proceed, return profile for dare assignment
             Ok(user_profile)
        } else {
             Err("User not found. Please /register first.".to_string())
        }
    });

    let mut user_profile = match profile_update_result {
        Ok(profile) => profile,
        Err(e) if e == "HAS_ACTIVE_DARE" => {
            // Need to retrieve the active dare text to show user
             return USER_PROFILES.with(|profiles_ref| {
                DARE_REPOSITORY.with(|repo_ref| {
                    if let Some(profile) = profiles_ref.borrow().get(&storable_caller){
                        if let Some(active_dare_id) = profile.current_dare_id {
                             repo_ref.borrow().get(active_dare_id)
                                .map(|d| Err(format!("You already have an active dare (ID: {}): '{}'. Complete it first using /submit.", active_dare_id, d.text)))
                                .unwrap_or_else(|| Err("Error finding your current dare's text.".to_string()))
                        } else { Err("Internal error: No active dare ID found.".to_string())}
                    } else { Err("Internal error: User profile disappeared.".to_string())} // Should not happen
                })
            });
        }
        Err(e) => return Err(e), // Return other errors (like "not registered")
    };


    // --- Step 2: Select Dare from Repository ---
    let assignment_result: Result<(u64, String), String> = DARE_REPOSITORY.with(|repo_ref| {
        let repo = repo_ref.borrow();
        if repo.is_empty() {
            return Err("No dares available in the repository.".to_string());
        }

        // Filter dares by requested difficulty
        let filtered_dares: Vec<(u64, Dare)> = repo.iter()
            .enumerate() // Use enumerate to get index (u64) and value
            .filter(|(_idx, dare)| dare.difficulty == difficulty_request)
            .map(|(idx, dare)| (idx as u64, dare)) // Map to (id, Dare)
            .collect();

        if filtered_dares.is_empty() {
            return Err(format!("No dares found for difficulty: {:?}.", difficulty_request));
        }

        // Select a random dare from the filtered list
        let random_filtered_index = get_pseudo_random_u64(filtered_dares.len() as u64);
        if let Some((selected_dare_id, selected_dare)) = filtered_dares.get(random_filtered_index as usize) {
            Ok((*selected_dare_id, selected_dare.text.clone()))
        } else {
            Err("Failed to select a dare from the filtered list.".to_string()) // Should not happen
        }
    });

    // --- Step 3: Finalize Profile Update and Return ---
    match assignment_result {
        Ok((assigned_dare_id, dare_text)) => {
            user_profile.current_dare_id = Some(assigned_dare_id);
            // Re-insert the updated profile
            USER_PROFILES.with(|profiles_ref| {
                profiles_ref.borrow_mut().insert(storable_caller, user_profile);
            });
            Ok(format!("Your new {:?} dare (ID: {}): {}", difficulty_request, assigned_dare_id, dare_text))
        }
        Err(e) => {
            // If dare selection failed, re-insert the original profile without changes
            USER_PROFILES.with(|profiles_ref| {
                profiles_ref.borrow_mut().insert(storable_caller, user_profile);
            });
            Err(e)
        }
    }
}

// Submit Dare Completion
#[update]
fn submit_dare(proof: String) -> Result<String, String> {
    // Basic proof validation
    if proof.trim().is_empty() {
        return Err("Proof cannot be empty.".to_string());
    }

    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);

    USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();

        // Get mutable access by temporary removal
        if let Some(mut profile) = profiles.remove(&storable_caller) {
            if profile.current_dare_id.is_some() {
                // **VERIFICATION LOGIC WOULD GO HERE**
                // E.g., check proof against expected outcome based on profile.current_dare_id

                // Assume valid for now: Increment streak & clear dare ID
                profile.streak += 1;
                profile.current_dare_id = None;

                let streak = profile.streak;
                // Re-insert updated profile
                profiles.insert(storable_caller, profile);
                Ok(format!("Dare submitted successfully! Your new streak is {}. You can now /get_dare again.", streak))

            } else {
                // Re-insert unchanged profile before returning error
                profiles.insert(storable_caller, profile);
                Err("You don't have an active dare to submit. Use /get_dare first.".to_string())
            }
        } else {
            Err("User not found. Please /register first.".to_string())
        }
    })
}

// Redeem Streak Rewards
#[update]
fn redeem_reward() -> Result<String, String> {
    let caller_principal = caller();
    let storable_caller = StorablePrincipal(caller_principal);
    
    let mut final_message = String::new();
    let mut user_found = false;

    USER_PROFILES.with(|profiles_ref| {
        let mut profiles = profiles_ref.borrow_mut();
        if let Some(mut profile) = profiles.remove(&storable_caller) { // Use remove/insert pattern
            user_found = true; // Mark that we found the user
            let current_streak = profile.streak;
            // Use BTreeSet for efficient checking of redeemed milestones temporarily
            let mut already_redeemed = BTreeSet::from_iter(profile.redeemed_milestones.iter().cloned());
            let mut profile_updated = false;
            let mut specific_reward_msg = String::new();

            for &milestone in REWARD_MILESTONES {
                if current_streak >= milestone && !already_redeemed.contains(&milestone) {
                    already_redeemed.insert(milestone); // Update the temporary set
                    profile_updated = true;
                    specific_reward_msg = format!("Congratulations! You've redeemed the streak {} reward!", milestone);
                    break; // Redeem only one reward per call
                }
            }

            if profile_updated {
                // Update the profile's Vec if changes were made
                profile.redeemed_milestones = already_redeemed.into_iter().collect();
                final_message = specific_reward_msg; // Set the success message
            } else {
                // No new reward, set the standard message
                final_message = format!("No new rewards available at your current streak of {}.", current_streak);
            }

            // Re-insert the potentially updated profile
            profiles.insert(storable_caller.clone(), profile);

        } else {
            user_found = false; // Mark that user was not found
            // Don't return Err from inside the closure
        }
    });// Closure now implicitly returns ()

    // Construct final Result outside the closure based on flags/messages set within
    if user_found {
        Ok(final_message) // Return the message determined inside the closure
    } else {
        Err("User not found. Please /register first.".to_string()) // Return Err if user wasn't found
    }
}


// Get Leaderboard
#[query]
fn get_leaderboard() -> Vec<(Principal, u32)> {
    let mut leaderboard: Vec<(Principal, u32)> = USER_PROFILES.with(|profiles_ref| {
        profiles_ref.borrow().iter()
            .map(|(storable_principal, profile)| (storable_principal.0, profile.streak)) // Extract Principal and streak
            .collect()
    });

    // Sort by streak descending
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));

    // Truncate to max size
    leaderboard.truncate(MAX_LEADERBOARD_SIZE);

    leaderboard
}

// --- Admin Functions (Optional Placeholders) ---
// #[update]
// fn add_dare(text: String, difficulty: Difficulty) -> Result<u64, String> {
//     // TODO: Add admin check (e.g., using an ADMINS stable map)
//     // Check caller() is in ADMINS map
//     DARE_REPOSITORY.with(|repo_ref| {
//          let mut repo = repo_ref.borrow_mut();
//          let new_id = repo.len(); // ID will be the index
//          let dare = Dare { id: new_id, text, difficulty };
//          match repo.push(&dare) {
//               Ok(_) => Ok(new_id),
//               Err(e) => Err(format!("Failed to add dare: {:?}", e)),
//          }
//     })
// }

// --- Candid Export ---
ic_cdk::export_candid!();