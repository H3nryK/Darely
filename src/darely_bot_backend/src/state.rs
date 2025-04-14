use crate::types::{StorablePrincipal, UserProfile, Dare}; // Import types from local module
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BTreeMap as StableBTreeMap, DefaultMemoryImpl, StableVec};
use std::cell::RefCell;

// --- Memory Management ---
pub type Memory = VirtualMemory<DefaultMemoryImpl>; // Make Memory type public

// Define Memory IDs for different stable structures
const USER_PROFILES_MEM_ID: MemoryId = MemoryId::new(0);
// Keep DARES_MEM_ID in case you want to log generated dares or have fallback static ones
const DARES_MEM_ID: MemoryId = MemoryId::new(1);

thread_local! {
    // The memory manager is used to allocate virtual memory for stable structures.
    // Make static variables pub for access from lib.rs
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // Stable storage for user profiles: Principal -> UserProfile
    pub static USER_PROFILES: RefCell<StableBTreeMap<StorablePrincipal, UserProfile, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(USER_PROFILES_MEM_ID)), // Get memory region
        )
    );

    // Stable storage for Dares (currently unused by get_dare, but kept for structure)
    // Potentially used for logging generated dares or as a fallback.
    pub static DARE_REPOSITORY: RefCell<StableVec<Dare, Memory>> = RefCell::new(
        StableVec::init(
             MEMORY_MANAGER.with(|m| m.borrow().get(DARES_MEM_ID)), // Get memory region
        ).expect("Failed to initialize stable dare repository")
    );
}