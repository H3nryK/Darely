use candid::{CandidType, Principal};
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_http_certification::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
// Removed direct state import, use state module functions
// use state::Config; // Config is accessed via state module functions now

// Use state module directly
pub mod memory;
pub mod router;
pub mod state;

#[init]
fn init(args: InitOrUpgradeArgs) {
    // Call the state initialization function
    state::init(args.oc_public_key, args.initial_admins);
}

#[pre_upgrade]
fn pre_upgrade() {
    // StableBTreeMaps persist automatically.
    // Only non-stable data (if any) would need manual serialization.
    // Config is in stable map, so likely nothing needed here.
    ic_cdk::println!("Running pre_upgrade...");
}

#[post_upgrade]
fn post_upgrade(args: InitOrUpgradeArgs) {
    // Call state re-initialization function
    ic_cdk::println!("Running post_upgrade...");
    state::post_upgrade_init(args.oc_public_key, args.initial_admins);
}

#[query]
async fn http_request(request: HttpRequest) -> HttpResponse {
    // Query calls are read-only, usually don't need caller check unless filtering results
    router::handle(request, true).await
}

#[update]
async fn http_request_update(request: HttpRequest) -> HttpResponse {
    // Update calls modify state, caller check often important in command logic
    let caller = ic_cdk::caller();
    ic_cdk::println!("http_request_update called by: {}", caller);
    router::handle(request, false).await
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitOrUpgradeArgs {
    pub oc_public_key: String,
    pub initial_admins: Vec<Principal>, // Specify initial admins on deploy/upgrade
}