mod state;
mod types;

use candid::Principal;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};

pub use state::{State, StableState, STATE};
pub use types::*;

// =============================================================================
// Canister Lifecycle
// =============================================================================

#[init]
fn init(controllers: Option<Vec<Principal>>) {
    let effective_controllers = controllers.unwrap_or_else(|| vec![ic_cdk::caller()]);
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.controllers = effective_controllers.clone();
        s.admins = effective_controllers;
    });

    ic_cdk::println!("===========================================");
    ic_cdk::println!("DAO Admin Initialization Complete");
    ic_cdk::println!("===========================================");
}

#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|state| {
        let s = state.borrow();
        let stable: StableState = (&*s).into();
        ic_cdk::storage::stable_save((stable,)).expect("Failed to save state to stable storage");
    });
}

#[post_upgrade]
fn post_upgrade() {
    let restored_state = match ic_cdk::storage::stable_restore::<(StableState,)>() {
        Ok((saved_state,)) => {
            ic_cdk::println!("Restored state from stable storage");
            State::from(saved_state)
        }
        Err(e) => {
            ic_cdk::println!("No previous state found ({}), using default state", e);
            State::new()
        }
    };

    STATE.with(|state| {
        *state.borrow_mut() = restored_state;
    });

    ic_cdk::println!("===========================================");
    ic_cdk::println!("DAO Admin Upgrade Complete");
    ic_cdk::println!("===========================================");
}

// =============================================================================
// Access Control
// =============================================================================

async fn require_controller() -> Result<(), String> {
    let caller = ic_cdk::caller();

    let is_authorized = STATE.with(|state| state.borrow().is_controller(&caller));

    if !is_authorized {
        use ic_cdk::api::management_canister::main::{canister_status, CanisterIdRecord};

        let status = canister_status(CanisterIdRecord {
            canister_id: ic_cdk::id(),
        })
        .await
        .map_err(|(code, msg)| format!("Failed to query canister status: {:?}: {}", code, msg))?
        .0;

        if !status.settings.controllers.contains(&caller) {
            return Err("Unauthorized: Only controllers can perform this action".to_string());
        }

        STATE.with(|state| {
            state.borrow_mut().controllers = status.settings.controllers;
        });
    }

    Ok(())
}

fn require_admin() -> Result<(), String> {
    let caller = ic_cdk::caller();
    STATE.with(|state| {
        if state.borrow().is_admin(&caller) {
            Ok(())
        } else {
            Err("Unauthorized: Admin access required".to_string())
        }
    })
}

// =============================================================================
// Admin Management
// =============================================================================

#[update]
async fn add_admin(principal: Principal) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().add_admin(principal);
    });

    ic_cdk::println!("Admin added: {}", principal);
    Ok(())
}

#[update]
async fn remove_admin(principal: Principal) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().remove_admin(&principal);
    });

    ic_cdk::println!("Admin removed: {}", principal);
    Ok(())
}

#[query]
fn get_admins() -> Vec<Principal> {
    STATE.with(|state| state.borrow().admins.clone())
}

// =============================================================================
// Contact API
// =============================================================================

#[update]
fn create_contact(request: CreateContactRequest) -> Result<Contact, String> {
    require_admin()?;

    let contact = STATE.with(|state| {
        state.borrow_mut().create_contact(request)
    });

    ic_cdk::println!("Created contact {}: {}", contact.id, contact.email);
    Ok(contact)
}

/// Called by user-service when a new user signs up
#[update]
fn create_contact_from_signup(request: CreateContactRequest) -> Result<Contact, String> {
    // This can be called by user-service canister
    // TODO: Add inter-canister call verification

    let contact = STATE.with(|state| {
        state.borrow_mut().create_contact(request)
    });

    // Auto-create a deal for the new lead
    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: format!("New signup: {}", contact.email),
        value: None,
        notes: Some("Auto-created from user signup".to_string()),
        expected_close_date: None,
    };

    let _ = STATE.with(|state| {
        state.borrow_mut().create_deal(deal_request)
    });

    ic_cdk::println!("Created contact from signup: {}", contact.email);
    Ok(contact)
}

#[query]
fn get_contact(id: ContactId) -> Option<Contact> {
    STATE.with(|state| state.borrow().get_contact(id).cloned())
}

#[query]
fn get_contact_by_email(email: String) -> Option<Contact> {
    STATE.with(|state| state.borrow().get_contact_by_email(&email).cloned())
}

#[query]
fn get_contacts(
    filter: Option<ContactFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Contact>, String> {
    require_admin()?;

    Ok(STATE.with(|state| {
        state.borrow().get_contacts(filter, pagination.unwrap_or_default())
    }))
}

// =============================================================================
// Deal API
// =============================================================================

#[update]
fn create_deal(request: CreateDealRequest) -> Result<Deal, String> {
    require_admin()?;

    STATE.with(|state| {
        state.borrow_mut().create_deal(request)
    })
}

#[query]
fn get_deal(id: DealId) -> Option<Deal> {
    STATE.with(|state| state.borrow().get_deal(id).cloned())
}

#[update]
fn update_deal_stage(id: DealId, stage: DealStage) -> Result<Deal, String> {
    require_admin()?;

    STATE.with(|state| {
        state.borrow_mut().update_deal_stage(id, stage)
            .ok_or_else(|| "Deal not found".to_string())
    })
}

#[query]
fn get_deals(
    filter: Option<DealFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Deal>, String> {
    require_admin()?;

    Ok(STATE.with(|state| {
        state.borrow().get_deals(filter, pagination.unwrap_or_default())
    }))
}

// =============================================================================
// Transaction API
// =============================================================================

#[update]
fn create_transaction(request: CreateTransactionRequest) -> Result<Transaction, String> {
    require_admin()?;

    let transaction = STATE.with(|state| {
        state.borrow_mut().create_transaction(request)
    });

    ic_cdk::println!("Created transaction {}: {} {}", transaction.id, transaction.amount, transaction.currency);
    Ok(transaction)
}

#[query]
fn get_transactions(
    filter: Option<TransactionFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Transaction>, String> {
    require_admin()?;

    Ok(STATE.with(|state| {
        state.borrow().get_transactions(filter, pagination.unwrap_or_default())
    }))
}

#[query]
fn get_financial_summary(from: Timestamp, to: Timestamp) -> Result<FinancialSummary, String> {
    require_admin()?;

    Ok(STATE.with(|state| {
        state.borrow().get_financial_summary(from, to)
    }))
}

// =============================================================================
// Feature Flag API
// =============================================================================

#[update]
fn set_feature_flag(request: SetFeatureFlagRequest) -> Result<(), String> {
    require_admin()?;

    STATE.with(|state| {
        state.borrow_mut().set_feature_flag(request);
    });

    Ok(())
}

#[query]
fn get_feature_flag(key: String) -> Option<FeatureFlag> {
    STATE.with(|state| state.borrow().get_feature_flag(&key).cloned())
}

#[query]
fn is_feature_enabled(key: String) -> bool {
    let caller = ic_cdk::caller();
    STATE.with(|state| state.borrow().is_feature_enabled(&key, &caller))
}

#[query]
fn list_feature_flags() -> Result<Vec<FeatureFlag>, String> {
    require_admin()?;

    Ok(STATE.with(|state| state.borrow().list_feature_flags()))
}

// =============================================================================
// Analytics API
// =============================================================================

#[update]
fn log_activity(user_id: String, action: String, metadata: Option<String>) {
    STATE.with(|state| {
        state.borrow_mut().log_activity(user_id, action, metadata);
    });
}

#[update]
fn record_metrics(snapshot: MetricsSnapshot) -> Result<(), String> {
    require_admin()?;

    STATE.with(|state| {
        state.borrow_mut().record_metrics(snapshot);
    });

    Ok(())
}

#[query]
fn list_metrics(from: Timestamp, to: Timestamp, limit: Option<u64>) -> Result<Vec<MetricsSnapshot>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().list_metrics(from, to, limit)))
}

#[query]
fn get_latest_metrics() -> Result<Option<MetricsSnapshot>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().get_latest_metrics()))
}

// =============================================================================
// Stats & Health
// =============================================================================

#[derive(candid::CandidType, serde::Serialize)]
pub struct AdminStats {
    pub total_contacts: u64,
    pub total_deals: u64,
    pub total_transactions: u64,
    pub active_feature_flags: u64,
}

#[query]
fn get_admin_stats() -> Result<AdminStats, String> {
    require_admin()?;

    Ok(STATE.with(|state| {
        let s = state.borrow();
        AdminStats {
            total_contacts: s.contacts.len() as u64,
            total_deals: s.deals.len() as u64,
            total_transactions: s.transactions.len() as u64,
            active_feature_flags: s.feature_flags.values().filter(|f| f.enabled).count() as u64,
        }
    }))
}

#[query]
fn health() -> String {
    "ok".to_string()
}

// Export candid interface
ic_cdk::export_candid!();
