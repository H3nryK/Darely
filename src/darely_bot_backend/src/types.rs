use candid::{CandidType, Principal, Decode, Encode};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

// --- Storable Principal Wrapper ---

// Wrapper around Principal to implement Storable for stable map keys
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StorablePrincipal(pub Principal); // Make inner field pub if needed directly, or provide methods

impl Storable for StorablePrincipal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(&self.0).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { StorablePrincipal(Decode!(bytes.as_ref(), Principal).unwrap()) }
    const BOUND: Bound = Bound::Unbounded; // Principal size varies but has system limits
}

// --- Core Application Types ---

// Difficulty Enum (used as input for get_dare)
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty { Easy, Medium, Hard, }

// Storable implementation for Difficulty (needed if stored, e.g., in Dare struct)
impl Storable for Difficulty {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
    const BOUND: Bound = Bound::Bounded { max_size: 10, is_fixed_size: false }; // Small fixed size
}

// Dare struct (potentially for logging/fallback)
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Dare {
    pub id: u64, // Keep fields pub for access from other modules
    pub text: String,
    pub difficulty: Difficulty,
}

// Storable implementation for Dare
impl Storable for Dare {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
    // Adjust max_size based on expected max dare text length
    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

// UserProfile struct - NOTE: current_dare_id is removed for LLM integration simplicity
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserProfile {
    pub streak: u32,
    // current_dare_id: Option<u64>, // Removed: Not tracking specific LLM dare assigned
    pub redeemed_milestones: Vec<u32>, // Using Vec as BTreeSet isn't easily Storable
}

// Storable implementation for UserProfile
impl Storable for UserProfile {
     fn to_bytes(&self) -> std::borrow::Cow<[u8]> { Cow::Owned(Encode!(self).unwrap()) }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self { Decode!(bytes.as_ref(), Self).unwrap() }
     // Estimate max size needed
     const BOUND: Bound = Bound::Bounded { max_size: 128, is_fixed_size: false };
}


// --- Structs for OpenAI API Interaction ---

// Request structure for OpenAI Chat Completions
#[derive(Serialize, Debug)]
pub struct OpenAIRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<OpenAIMessage<'a>>,
    pub max_tokens: u32,
    pub temperature: f32, // Controls randomness (0.0 - 2.0)
    // Add other parameters like top_p if needed
}

#[derive(Serialize, Debug)]
pub struct OpenAIMessage<'a> {
    pub role: &'a str, // Typically "system", "user", or "assistant"
    pub content: &'a str,
}

// Response structure (only fields needed are deserialized)
#[derive(Deserialize, Debug)]
pub struct OpenAIResponse {
    // pub id: String, // Optional: if you need the response ID
    // pub object: String, // Optional
    // pub created: u64, // Optional
    // pub model: String, // Optional
    pub choices: Vec<OpenAIChoice>,
    // pub usage: OpenAIUsage, // Optional: track token usage
}

#[derive(Deserialize, Debug)]
pub struct OpenAIChoice {
    // pub index: u32, // Optional
    pub message: OpenAIMessageResponse,
    // pub finish_reason: String, // Optional: e.g., "stop", "length"
}

#[derive(Deserialize, Debug)]
pub struct OpenAIMessageResponse {
    // pub role: String, // Optional: should be "assistant"
    pub content: String, // The generated dare text
}