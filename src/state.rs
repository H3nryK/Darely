use candid::{CandidType, Principal};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell};

// Import the memory type and accessors
use crate::memory::{self, Memory};

// --- Enums and Structs ---

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DareDifficulty {
    Easy,
    Medium,
    Hard,
}

impl Storable for DareDifficulty {
    fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(rmp_serde::to_vec(self).expect("Serialization failed")) }
    fn from_bytes(bytes: Cow<[u8]>) -> Self { rmp_serde::from_slice(bytes.as_ref()).expect("Deserialization failed") }
    const BOUND: Bound = Bound::Bounded { max_size: 10, is_fixed_size: false }; // Small enum
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Dare {
    pub id: u64,
    pub text: String,
    pub difficulty: DareDifficulty,
    // pub creator: Principal, // Optional: track who added it
    // pub created_at: u64,    // Optional: timestamp
}

impl Storable for Dare {
     fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(rmp_serde::to_vec(self).expect("Serialization failed")) }
     fn from_bytes(bytes: Cow<[u8]>) -> Self { rmp_serde::from_slice(bytes.as_ref()).expect("Deserialization failed") }
     const BOUND: Bound = Bound::Unbounded; // Text can vary greatly
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RedemptionTask {
    pub id: u64,
    pub description: String,
    pub required_streak: u32,
    // pub reward_details: String, // Optional: Describe the reward/badge/etc.
    // pub creator: Principal,     // Optional
    // pub created_at: u64,        // Optional
}

impl Storable for RedemptionTask {
     fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(rmp_serde::to_vec(self).expect("Serialization failed")) }
     fn from_bytes(bytes: Cow<[u8]>) -> Self { rmp_serde::from_slice(bytes.as_ref()).expect("Deserialization failed") }
     const BOUND: Bound = Bound::Unbounded; // Description can vary
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub principal: Principal,
    pub current_dare_id: Option<u64>,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub dares_completed: u64,
    pub last_completion_timestamp: u64,
    // pub redeemed_task_ids: Vec<u64>, // Optional: Track claimed rewards
    // pub current_redemption_task_id: Option<u64>, // Optional: Track assigned task
}

impl Storable for UserProfile {
     fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(rmp_serde::to_vec(self).expect("Serialization failed")) }
     fn from_bytes(bytes: Cow<[u8]>) -> Self { rmp_serde::from_slice(bytes.as_ref()).expect("Deserialization failed") }
     // Increased bound slightly for potential future fields
     const BOUND: Bound = Bound::Bounded { max_size: 300, is_fixed_size: false };
}

// --- State Definition ---

#[derive(Serialize, Deserialize, Default, Clone, Debug)] // Added Clone, Debug
pub struct Config {
    admins: Vec<Principal>,
    next_dare_id: u64,
    next_task_id: u64,
    oc_public_key: String,
}

impl Storable for Config {
     fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(rmp_serde::to_vec(self).expect("Serialization failed")) }
     fn from_bytes(bytes: Cow<[u8]>) -> Self { rmp_serde::from_slice(bytes.as_ref()).expect("Deserialization failed") }
     const BOUND: Bound = Bound::Unbounded; // Admins list can grow
}

pub struct State {
    pub users: StableBTreeMap<Principal, UserProfile, Memory>,
    pub dares: StableBTreeMap<u64, Dare, Memory>,
    pub tasks: StableBTreeMap<u64, RedemptionTask, Memory>,
    pub config: StableBTreeMap<u64, Config, Memory>, // Use key 0 for singleton config
}

// --- State Management ---
thread_local! {
    static STATE: RefCell<Option<State>> = RefCell::default();
}

// --- Initialization and Access ---
pub fn init(oc_public_key: String, initial_admins: Vec<Principal>) {
    let config_memory = memory::get_config_memory();
    let user_memory = memory::get_user_memory();
    let dare_memory = memory::get_dare_memory();
    let task_memory = memory::get_task_memory();

    let initial_config = Config {
        admins: initial_admins,
        next_dare_id: 1, // Start IDs from 1
        next_task_id: 1,
        oc_public_key,
    };

    let mut config_map = StableBTreeMap::init(config_memory);
    if config_map.get(&0).is_none() { // Initialize only if empty
         config_map.insert(0, initial_config);
    }


    let state = State {
        users: StableBTreeMap::init(user_memory),
        dares: StableBTreeMap::init(dare_memory),
        tasks: StableBTreeMap::init(task_memory),
        config: config_map,
    };

    STATE.with(|s| { *s.borrow_mut() = Some(state); });
    ic_cdk::println!("Darely Bot state initialized.");
}

pub fn post_upgrade_init(oc_public_key: String, initial_admins: Vec<Principal>) {
    let config_memory = memory::get_config_memory();
    let user_memory = memory::get_user_memory();
    let dare_memory = memory::get_dare_memory();
    let task_memory = memory::get_task_memory();

    // Re-initialize maps - data persists in stable memory
    let mut state = State {
        users: StableBTreeMap::init(user_memory),
        dares: StableBTreeMap::init(dare_memory),
        tasks: StableBTreeMap::init(task_memory),
        config: StableBTreeMap::init(config_memory),
    };

    // Ensure config exists and update OC key/admins if needed
    let oc_public_key_clone = oc_public_key.clone();
    let mut config = state.config.get(&0).map(|c| c.clone()).unwrap_or_else(|| { // Clone existing or create default
         ic_cdk::println!("WARN: Config not found post-upgrade, re-initializing.");
         Config {
            admins: initial_admins, // Be careful with overwriting admins on upgrade
            next_dare_id: state.dares.len() as u64 + 1, // Try to resume ID count
            next_task_id: state.tasks.len() as u64 + 1,
            oc_public_key: oc_public_key_clone,
        }
    });

    config.oc_public_key = oc_public_key; // Always update OC key from args
    // Logic to merge admins if needed:
    // for admin in initial_admins {
    //     if !config.admins.contains(&admin) {
    //         config.admins.push(admin);
    //     }
    // }
    state.config.insert(0, config);


    STATE.with(|s| { *s.borrow_mut() = Some(state); });
    ic_cdk::println!("Darely Bot state restored after upgrade.");
}

// Immutable access
pub fn read<F, R>(f: F) -> R where F: FnOnce(&State) -> R {
    STATE.with(|s| f(s.borrow().as_ref().expect("State not initialized")))
}

// Mutable access to Config (safer pattern: read, clone, modify, insert)
pub fn mutate_config<F, R>(f: F) -> R where F: FnOnce(&mut Config) -> R {
     STATE.with(|s| {
         let mut state_ref = s.borrow_mut();
         let state = state_ref.as_mut().expect("State not initialized");
         let mut config = state.config.get(&0).expect("Config not found").clone(); // Clone
         let result = f(&mut config);
         state.config.insert(0, config); // Re-insert modified clone
         result
     })
}

// --- Data Accessors ---

pub fn get_user(principal: &Principal) -> Option<UserProfile> { read(|state| state.users.get(principal)) }
pub fn insert_user(principal: Principal, profile: UserProfile) { STATE.with(|s| s.borrow_mut().as_mut().unwrap().users.insert(principal, profile)); }
pub fn get_oc_public_key() -> String { read(|state| state.config.get(&0).unwrap().oc_public_key.clone()) }
pub fn is_admin(principal: &Principal) -> bool { read(|state| state.config.get(&0).map_or(false, |c| c.admins.contains(principal))) }

pub fn add_admin(principal: Principal) -> Result<(), String> {
    mutate_config(|config| {
        if !config.admins.contains(&principal) {
            config.admins.push(principal); Ok(())
        } else { Err("Principal is already an admin".to_string()) }
    })
}

pub fn remove_admin(principal: Principal) -> Result<(), String> {
    mutate_config(|config| {
        if let Some(pos) = config.admins.iter().position(|p| p == &principal) {
            config.admins.remove(pos); Ok(())
        } else { Err("Principal is not an admin".to_string()) }
    })
}

pub fn get_next_dare_id() -> u64 { mutate_config(|config| { let id = config.next_dare_id; config.next_dare_id += 1; id }) }
pub fn insert_dare(dare: Dare) { STATE.with(|s| s.borrow_mut().as_mut().unwrap().dares.insert(dare.id, dare)); }
pub fn get_dare(id: u64) -> Option<Dare> { read(|state| state.dares.get(&id)) }
pub fn get_all_dares() -> Vec<Dare> { read(|state| state.dares.iter().map(|(_, d)| d.clone()).collect()) } // Helper for random selection
pub fn get_dares_by_difficulty(difficulty: DareDifficulty) -> Vec<Dare> { read(|state| state.dares.iter().filter(|(_, d)| d.difficulty == difficulty).map(|(_, d)| d.clone()).collect()) }

pub fn get_next_task_id() -> u64 { mutate_config(|config| { let id = config.next_task_id; config.next_task_id += 1; id }) }
pub fn insert_task(task: RedemptionTask) { STATE.with(|s| s.borrow_mut().as_mut().unwrap().tasks.insert(task.id, task)); }
pub fn get_task(id: u64) -> Option<RedemptionTask> { read(|state| state.tasks.get(&id)) }
pub fn get_tasks_for_streak(streak: u32) -> Vec<RedemptionTask> { read(|state| state.tasks.iter().filter(|(_, t)| t.required_streak <= streak).map(|(_, t)| t.clone()).collect()) }

pub fn get_all_users() -> Vec<(Principal, UserProfile)> { read(|state| state.users.iter().collect()) }