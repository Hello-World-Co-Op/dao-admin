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

    // FOS-5.6.10: Migrate ownership for records created before row-level security
    STATE.with(|state| {
        state.borrow_mut().migrate_ownership();
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

/// Verify caller is an authorized canister for the given role
/// @see AC-5.6.8.3 - Inter-canister call verification
fn require_authorized_canister(role: &str) -> Result<(), String> {
    let caller = ic_cdk::caller();
    STATE.with(|state| {
        let s = state.borrow();
        if s.is_authorized_canister(role, &caller) {
            Ok(())
        } else {
            Err(format!("Unauthorized: Expected {} canister", role))
        }
    })
}

/// Verify caller is either an authorized canister (for any of the given roles) OR an admin
/// @see AC-5.6.8.4 - Authorization for log_activity
fn require_authorized_canister_or_admin(roles: &[&str]) -> Result<(), String> {
    let caller = ic_cdk::caller();
    STATE.with(|state| {
        let s = state.borrow();

        // Check if caller is admin
        if s.is_admin(&caller) {
            return Ok(());
        }

        // Check if caller is any of the authorized canisters
        for role in roles {
            if s.is_authorized_canister(role, &caller) {
                return Ok(());
            }
        }

        Err("Unauthorized: Requires admin or authorized canister access".to_string())
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
fn get_admins() -> Result<Vec<Principal>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().admins.clone()))
}

// =============================================================================
// Authorized Canister Management
// =============================================================================

/// Register an authorized canister for inter-canister calls
/// @see AC-5.6.8.3 - Inter-canister call verification
#[update]
async fn register_authorized_canister(role: String, canister_id: Principal) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().register_authorized_canister(role.clone(), canister_id);
    });

    ic_cdk::println!("Authorized canister registered: {} = {}", role, canister_id);
    Ok(())
}

/// Unregister an authorized canister
#[update]
async fn unregister_authorized_canister(role: String) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().unregister_authorized_canister(&role);
    });

    ic_cdk::println!("Authorized canister unregistered: {}", role);
    Ok(())
}

/// List all authorized canisters (admin only)
#[query]
fn list_authorized_canisters() -> Result<Vec<(String, Principal)>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().get_authorized_canisters()))
}

// =============================================================================
// Permission Management (FOS-5.6.10)
// =============================================================================

/// Grant a permission to an admin
/// @see AC-5.6.10.3 - Granular CRUD permissions
#[update]
async fn grant_permission(principal: Principal, permission: AdminPermission) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().grant_permission(principal, permission.clone());
    });

    ic_cdk::println!("Granted permission {:?} to {}", permission, principal);
    Ok(())
}

/// Revoke a permission from an admin
/// @see AC-5.6.10.3 - Granular CRUD permissions
#[update]
async fn revoke_permission(principal: Principal, permission: AdminPermission) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().revoke_permission(&principal, &permission);
    });

    ic_cdk::println!("Revoked permission {:?} from {}", permission, principal);
    Ok(())
}

/// Get permissions for a principal (admin can view their own, controller can view all)
#[query]
fn get_permissions(principal: Option<Principal>) -> Result<Vec<AdminPermission>, String> {
    let caller = ic_cdk::caller();

    let target = principal.unwrap_or(caller);

    STATE.with(|state| {
        let s = state.borrow();

        // Only controllers can view other admins' permissions
        if target != caller && !s.is_controller(&caller) {
            return Err("Unauthorized: Only controllers can view other admins' permissions".to_string());
        }

        if !s.is_admin(&target) && !s.is_controller(&target) {
            return Err("Target is not an admin".to_string());
        }

        Ok(s.get_permissions(&target))
    })
}

/// Grant default permissions to all admins (controller only)
/// Call after upgrade to ensure all admins have basic permissions
#[update]
async fn grant_default_permissions_to_all_admins() -> Result<u32, String> {
    require_controller().await?;

    let count = STATE.with(|state| {
        let mut s = state.borrow_mut();
        let admins: Vec<Principal> = s.admins.clone();

        for admin in &admins {
            s.grant_default_permissions(*admin);
        }

        admins.len() as u32
    });

    ic_cdk::println!("Granted default permissions to {} admins", count);
    Ok(count)
}

// =============================================================================
// Audit Log API (FOS-5.6.10)
// =============================================================================

/// Get audit log entries
/// @see AC-5.6.10.4, AC-5.6.10.5 - Audit logging
#[query]
fn get_audit_log(
    action_filter: Option<String>,
    target_type_filter: Option<String>,
    actor_filter: Option<Principal>,
    limit: Option<u64>,
) -> Result<Vec<AuditLogEntry>, String> {
    require_admin()?;

    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let s = state.borrow();

        // Check ViewAuditLogs permission
        if !s.has_permission(&caller, &AdminPermission::ViewAuditLogs) && !s.is_controller(&caller) {
            return Err("Unauthorized: ViewAuditLogs permission required".to_string());
        }

        Ok(s.get_audit_log(
            action_filter.as_deref(),
            target_type_filter.as_deref(),
            actor_filter.as_ref(),
            limit,
        ))
    })
}

// =============================================================================
// Contact API
// =============================================================================

/// Create a new contact (admin only)
/// @see AC-5.6.10.1 - Sets owner_id to caller for row-level security
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn create_contact(request: CreateContactRequest) -> Result<Contact, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    let contact = STATE.with(|state| {
        let mut s = state.borrow_mut();
        let contact = s.create_contact(request.clone(), caller);

        // Audit log
        s.record_audit_log(
            caller,
            "create_contact",
            "contact",
            &contact.id.to_string(),
            Some(serde_json::json!({
                "email": request.email,
                "source": format!("{:?}", request.source.unwrap_or_default()),
            }).to_string()),
        );

        contact
    });

    ic_cdk::println!("Created contact {}", contact.id);
    Ok(contact)
}

/// Called by user-service when a new user signs up
/// @see AC-5.6.8.3 - Validates caller is user-service canister
/// @see AC-5.6.10.1 - Sets owner_id to service principal (caller)
/// @see AC-5.6.10.4 - Audit logging for CRM operations
#[update]
fn create_contact_from_signup(request: CreateContactRequest) -> Result<Contact, String> {
    // Verify caller is the authorized user-service canister
    require_authorized_canister("user-service")?;

    let caller = ic_cdk::caller();

    let contact = STATE.with(|state| {
        let mut s = state.borrow_mut();
        let contact = s.create_contact(request.clone(), caller);

        // Audit log for contact creation (AC-5.6.10.4)
        s.record_audit_log(
            caller,
            "create_contact_from_signup",
            "contact",
            &contact.id.to_string(),
            Some(serde_json::json!({
                "source": "user_signup",
                "user_id": request.user_id,
            }).to_string()),
        );

        contact
    });

    // Auto-create a deal for the new lead
    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: format!("New signup: Contact #{}", contact.id),
        value: None,
        notes: Some("Auto-created from user signup".to_string()),
        expected_close_date: None,
    };

    let deal = STATE.with(|state| {
        let mut s = state.borrow_mut();
        let deal = s.create_deal(deal_request.clone(), caller)?;

        // Audit log for auto-created deal (AC-5.6.10.4)
        s.record_audit_log(
            caller,
            "create_deal_from_signup",
            "deal",
            &deal.id.to_string(),
            Some(serde_json::json!({
                "contact_id": deal_request.contact_id,
                "source": "auto_signup",
            }).to_string()),
        );

        Ok::<_, String>(deal)
    });

    if let Err(e) = deal {
        // Log but don't fail - contact was created successfully
        ic_cdk::println!("Warning: Failed to auto-create deal: {}", e);
    }

    Ok(contact)
}

/// Get contact by ID (admin only)
/// @see AC-5.6.8.5 - Query endpoints require admin authorization
#[query]
fn get_contact(id: ContactId) -> Result<Option<Contact>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().get_contact(id).cloned()))
}

/// Get contact by email (admin only)
/// @see AC-5.6.8.5 - Query endpoints require admin authorization
/// Note: This is especially sensitive due to email enumeration risk
#[query]
fn get_contact_by_email(email: String) -> Result<Option<Contact>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().get_contact_by_email(&email).cloned()))
}

/// Get contacts with row-level security filtering
/// @see AC-5.6.10.1 - Row-level security filtering
#[query]
fn get_contacts(
    filter: Option<ContactFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Contact>, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    Ok(STATE.with(|state| {
        state.borrow().get_contacts(filter, pagination.unwrap_or_default(), &caller)
    }))
}

/// Update a contact with permission check
/// @see AC-5.6.10.3 - Granular CRUD permissions (EditOwnContacts/EditAllContacts)
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn update_contact(request: UpdateContactRequest) -> Result<Contact, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Get contact to check ownership
        let contact = s.get_contact(request.id)
            .ok_or("Contact not found")?
            .clone();

        // Check permissions
        let has_edit_all = s.has_permission(&caller, &AdminPermission::EditAllContacts);
        let has_edit_own = s.has_permission(&caller, &AdminPermission::EditOwnContacts);
        let is_owner = contact.owner_id.as_ref() == Some(&caller);

        if !has_edit_all && !(has_edit_own && is_owner) {
            return Err("Unauthorized: Cannot edit this contact".to_string());
        }

        // Capture old values for audit log
        let old_values = serde_json::json!({
            "name": contact.name,
            "company": contact.company,
            "status": format!("{:?}", contact.status),
        }).to_string();

        // Perform update
        let updated = s.update_contact(
            request.id,
            request.name,
            request.company,
            request.job_title,
            request.interest_area,
            request.notes,
            request.status,
        ).ok_or("Failed to update contact")?;

        // Audit log
        s.record_audit_log(
            caller,
            "update_contact",
            "contact",
            &request.id.to_string(),
            Some(old_values),
        );

        Ok(updated)
    })
}

/// Delete a contact with permission check
/// @see AC-5.6.10.3 - Granular CRUD permissions (DeleteOwnContacts/DeleteAllContacts)
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn delete_contact(id: ContactId) -> Result<Contact, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Get contact to check ownership
        let contact = s.get_contact(id)
            .ok_or("Contact not found")?
            .clone();

        // Check permissions
        let has_delete_all = s.has_permission(&caller, &AdminPermission::DeleteAllContacts);
        let has_delete_own = s.has_permission(&caller, &AdminPermission::DeleteOwnContacts);
        let is_owner = contact.owner_id.as_ref() == Some(&caller);

        if !has_delete_all && !(has_delete_own && is_owner) {
            return Err("Unauthorized: Cannot delete this contact".to_string());
        }

        // Audit log before deletion
        s.record_audit_log(
            caller,
            "delete_contact",
            "contact",
            &id.to_string(),
            Some(serde_json::json!({
                "email": contact.email,
                "name": contact.name,
            }).to_string()),
        );

        // Perform deletion
        s.delete_contact(id)
            .ok_or("Failed to delete contact".to_string())
    })
}

// =============================================================================
// Deal API
// =============================================================================

/// Create a new deal (admin only)
/// @see AC-5.6.10.1 - Sets owner_id to caller for row-level security
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn create_deal(request: CreateDealRequest) -> Result<Deal, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();
        let deal = s.create_deal(request.clone(), caller)?;

        // Audit log
        s.record_audit_log(
            caller,
            "create_deal",
            "deal",
            &deal.id.to_string(),
            Some(serde_json::json!({
                "contact_id": request.contact_id,
                "name": request.name,
                "value": request.value,
            }).to_string()),
        );

        Ok(deal)
    })
}

/// Get deal by ID (admin only)
/// @see AC-5.6.8.5 - Query endpoints require admin authorization
#[query]
fn get_deal(id: DealId) -> Result<Option<Deal>, String> {
    require_admin()?;
    Ok(STATE.with(|state| state.borrow().get_deal(id).cloned()))
}

/// Update deal stage with ownership check
/// @see AC-5.6.10.3 - Granular CRUD permissions
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn update_deal_stage(id: DealId, stage: DealStage) -> Result<Deal, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Get deal to check ownership
        let deal = s.get_deal(id)
            .ok_or("Deal not found")?
            .clone();

        // Check permissions
        let has_edit_all = s.has_permission(&caller, &AdminPermission::EditAllDeals);
        let has_edit_own = s.has_permission(&caller, &AdminPermission::EditOwnDeals);
        let is_owner = deal.owner_id.as_ref() == Some(&caller);

        if !has_edit_all && !(has_edit_own && is_owner) {
            return Err("Unauthorized: Cannot update this deal".to_string());
        }

        // Capture old stage for audit
        let old_stage = format!("{:?}", deal.stage);

        // Perform update
        let updated = s.update_deal_stage(id, stage.clone())
            .ok_or("Failed to update deal stage")?;

        // Audit log
        s.record_audit_log(
            caller,
            "update_deal_stage",
            "deal",
            &id.to_string(),
            Some(serde_json::json!({
                "old_stage": old_stage,
                "new_stage": format!("{:?}", stage),
            }).to_string()),
        );

        Ok(updated)
    })
}

/// Update a deal with permission check
/// @see AC-5.6.10.3 - Granular CRUD permissions (EditOwnDeals/EditAllDeals)
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn update_deal(request: UpdateDealRequest) -> Result<Deal, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Get deal to check ownership
        let deal = s.get_deal(request.id)
            .ok_or("Deal not found")?
            .clone();

        // Check permissions
        let has_edit_all = s.has_permission(&caller, &AdminPermission::EditAllDeals);
        let has_edit_own = s.has_permission(&caller, &AdminPermission::EditOwnDeals);
        let is_owner = deal.owner_id.as_ref() == Some(&caller);

        if !has_edit_all && !(has_edit_own && is_owner) {
            return Err("Unauthorized: Cannot edit this deal".to_string());
        }

        // Capture old values for audit
        let old_values = serde_json::json!({
            "name": deal.name,
            "value": deal.value,
            "stage": format!("{:?}", deal.stage),
        }).to_string();

        // Perform update
        let updated = s.update_deal(
            request.id,
            request.name,
            request.value,
            request.stage,
            request.notes,
            request.expected_close_date,
        ).ok_or("Failed to update deal")?;

        // Audit log
        s.record_audit_log(
            caller,
            "update_deal",
            "deal",
            &request.id.to_string(),
            Some(old_values),
        );

        Ok(updated)
    })
}

/// Delete a deal with permission check
/// @see AC-5.6.10.3 - Granular CRUD permissions (DeleteOwnDeals/DeleteAllDeals)
/// @see AC-5.6.10.4 - Audit logging
#[update]
fn delete_deal(id: DealId) -> Result<Deal, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Get deal to check ownership
        let deal = s.get_deal(id)
            .ok_or("Deal not found")?
            .clone();

        // Check permissions
        let has_delete_all = s.has_permission(&caller, &AdminPermission::DeleteAllDeals);
        let has_delete_own = s.has_permission(&caller, &AdminPermission::DeleteOwnDeals);
        let is_owner = deal.owner_id.as_ref() == Some(&caller);

        if !has_delete_all && !(has_delete_own && is_owner) {
            return Err("Unauthorized: Cannot delete this deal".to_string());
        }

        // Audit log before deletion
        s.record_audit_log(
            caller,
            "delete_deal",
            "deal",
            &id.to_string(),
            Some(serde_json::json!({
                "name": deal.name,
                "value": deal.value,
                "contact_id": deal.contact_id,
            }).to_string()),
        );

        // Perform deletion
        s.delete_deal(id)
            .ok_or("Failed to delete deal".to_string())
    })
}

/// Get deals with row-level security filtering
/// @see AC-5.6.10.1 - Row-level security filtering
#[query]
fn get_deals(
    filter: Option<DealFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Deal>, String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    Ok(STATE.with(|state| {
        state.borrow().get_deals(filter, pagination.unwrap_or_default(), &caller)
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

/// Set feature flag with permission check and audit logging
/// @see AC-5.6.10.5 - Feature flag audit logging
#[update]
fn set_feature_flag(request: SetFeatureFlagRequest) -> Result<(), String> {
    require_admin()?;
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Check ManageFeatureFlags permission
        if !s.has_permission(&caller, &AdminPermission::ManageFeatureFlags) && !s.is_controller(&caller) {
            return Err("Unauthorized: ManageFeatureFlags permission required".to_string());
        }

        // Get old value for audit
        let old_value = s.get_feature_flag(&request.key).map(|f| serde_json::json!({
            "enabled": f.enabled,
            "percentage": f.percentage,
        }).to_string());

        // Perform update
        s.set_feature_flag(request.clone());

        // Audit log
        s.record_audit_log(
            caller,
            "set_feature_flag",
            "feature_flag",
            &request.key,
            Some(serde_json::json!({
                "old_value": old_value.unwrap_or_else(|| "null".to_string()),
                "new_enabled": request.enabled,
                "new_percentage": request.percentage,
            }).to_string()),
        );

        Ok(())
    })
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

/// Log user activity (requires admin or authorized canister)
/// @see AC-5.6.8.4 - log_activity requires authorization
/// @see AC-5.6.8 Task 4.2 - Rate limiting to prevent log flooding
#[update]
fn log_activity(user_id: String, action: String, metadata: Option<String>) -> Result<(), String> {
    // Verify caller is admin or an authorized canister
    require_authorized_canister_or_admin(&["user-service", "auth-service", "frontend"])?;

    let caller = ic_cdk::caller();

    // Check and enforce rate limit (FOS-5.6.8)
    STATE.with(|state| {
        state.borrow_mut().check_rate_limit(&caller)
    })?;

    STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.log_activity(user_id, action, metadata);

        // Periodic cleanup: sweep stale rate limit buckets every ~100 activity logs
        // Uses activity log length as a simple counter
        if s.activity_log.len() % 100 == 0 {
            s.cleanup_rate_limits();
        }
    });

    Ok(())
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
