use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};
use std::cell::RefCell;

// --- Memory IDs ---
// Moved definitions here for clarity
pub const UPGRADES_MEMORY_ID: u8 = 0; // Keep if manual upgrade serialization is used
pub const USER_MEMORY_ID: u8 = 1;
pub const DARES_MEMORY_ID: u8 = 2;
pub const TASKS_MEMORY_ID: u8 = 3;
pub const CONFIG_MEMORY_ID: u8 = 4; // If Config is in stable memory

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    // Initialize MemoryManager using RefCell for flexible access
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>>
        = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

// Function to get memory for a specific purpose
pub fn get_memory(id_raw: u8) -> Memory {
    let memory_id = MemoryId::new(id_raw);
    MEMORY_MANAGER.with(|m| m.borrow().get(memory_id))
}

// Convenience functions (optional but helpful)
pub fn get_upgrades_memory() -> Memory {
    // Only needed if you manually serialize/deserialize *some* data during upgrades
    get_memory(UPGRADES_MEMORY_ID)
}
pub fn get_user_memory() -> Memory {
    get_memory(USER_MEMORY_ID)
}
pub fn get_dare_memory() -> Memory {
    get_memory(DARES_MEMORY_ID)
}
pub fn get_task_memory() -> Memory {
    get_memory(TASKS_MEMORY_ID)
}
pub fn get_config_memory() -> Memory {
    get_memory(CONFIG_MEMORY_ID)
}