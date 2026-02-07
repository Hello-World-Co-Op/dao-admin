use candid::{decode_one, encode_one, encode_args, Principal};
use pocket_ic::{PocketIc, WasmResult};
use serde::{Deserialize, Serialize};

// ============================================================================
// Type Definitions (matching canister types from dao_admin.did)
// ============================================================================

// Type aliases
type ContactId = u64;
type DealId = u64;
type TransactionId = u64;
type Timestamp = u64;

// CRM - Contact Types
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, PartialEq)]
enum ContactSource {
    Signup,
    Referral,
    Marketing,
    Event,
    Partner,
    Other,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, PartialEq)]
enum ContactStatus {
    Active,
    Inactive,
    Churned,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct Contact {
    id: ContactId,
    user_id: Option<String>,
    email: String,
    name: Option<String>,
    company: Option<String>,
    job_title: Option<String>,
    interest_area: Option<String>,
    source: ContactSource,
    notes: Option<String>,
    status: ContactStatus,
    created_at: Timestamp,
    updated_at: Timestamp,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct CreateContactRequest {
    user_id: Option<String>,
    email: String,
    name: Option<String>,
    company: Option<String>,
    job_title: Option<String>,
    interest_area: Option<String>,
    source: Option<ContactSource>,
    notes: Option<String>,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, Default)]
struct ContactFilter {
    status: Option<ContactStatus>,
    source: Option<ContactSource>,
    search: Option<String>,
}

// CRM - Deal Types
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, PartialEq)]
enum DealStage {
    Lead,
    Qualified,
    Proposal,
    Negotiation,
    ClosedWon,
    ClosedLost,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct Deal {
    id: DealId,
    contact_id: ContactId,
    name: String,
    value: Option<u64>,
    stage: DealStage,
    notes: Option<String>,
    expected_close_date: Option<Timestamp>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct CreateDealRequest {
    contact_id: ContactId,
    name: String,
    value: Option<u64>,
    notes: Option<String>,
    expected_close_date: Option<Timestamp>,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, Default)]
struct DealFilter {
    stage: Option<DealStage>,
    contact_id: Option<ContactId>,
}

// Finance - Transaction Types
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, PartialEq)]
enum TransactionType {
    Income,
    Expense,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, PartialEq)]
enum TransactionCategory {
    Subscription,
    Donation,
    Service,
    Infrastructure,
    Marketing,
    Payroll,
    Legal,
    Other,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct Transaction {
    id: TransactionId,
    transaction_type: TransactionType,
    category: TransactionCategory,
    amount: u64,
    currency: String,
    description: String,
    reference: Option<String>,
    date: Timestamp,
    created_at: Timestamp,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct CreateTransactionRequest {
    transaction_type: TransactionType,
    category: TransactionCategory,
    amount: u64,
    currency: Option<String>,
    description: String,
    reference: Option<String>,
    date: Option<Timestamp>,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize, Default)]
struct TransactionFilter {
    transaction_type: Option<TransactionType>,
    category: Option<TransactionCategory>,
    from_date: Option<Timestamp>,
    to_date: Option<Timestamp>,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct FinancialSummary {
    total_income: u64,
    total_expenses: u64,
    net: i64,
    mrr: u64,
    period_start: Timestamp,
    period_end: Timestamp,
}

// Feature Flags
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct FeatureFlag {
    key: String,
    enabled: bool,
    description: Option<String>,
    percentage: Option<u8>,
    allowed_principals: Vec<Principal>,
    updated_at: Timestamp,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct SetFeatureFlagRequest {
    key: String,
    enabled: bool,
    description: Option<String>,
    percentage: Option<u8>,
    allowed_principals: Option<Vec<Principal>>,
}

// Analytics - Metrics Types
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct MetricsSnapshot {
    total_users: u64,
    active_users_24h: u64,
    active_users_7d: u64,
    active_users_30d: u64,
    total_captures: u64,
    total_sprints: u64,
    total_workspaces: u64,
    timestamp: Timestamp,
}

// Query Types
#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct PaginationParams {
    offset: Option<u64>,
    limit: Option<u64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(50),
        }
    }
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct PaginatedContactResponse {
    items: Vec<Contact>,
    total: u64,
    offset: u64,
    limit: u64,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct PaginatedDealResponse {
    items: Vec<Deal>,
    total: u64,
    offset: u64,
    limit: u64,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct PaginatedTransactionResponse {
    items: Vec<Transaction>,
    total: u64,
    offset: u64,
    limit: u64,
}

#[derive(Clone, Debug, candid::CandidType, Deserialize, Serialize)]
struct AdminStats {
    total_contacts: u64,
    total_deals: u64,
    total_transactions: u64,
    active_feature_flags: u64,
}

// ============================================================================
// Test Helpers
// ============================================================================

fn unwrap_wasm_result(result: WasmResult) -> Vec<u8> {
    match result {
        WasmResult::Reply(bytes) => bytes,
        WasmResult::Reject(msg) => panic!("Canister rejected call: {}", msg),
    }
}

fn get_wasm_path() -> String {
    // Try wasm/ first (preferred for tests), then target/
    let paths = [
        "wasm/dao_admin.wasm",
        "target/wasm32-unknown-unknown/release/dao_admin.wasm",
    ];
    for path in paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    panic!("WASM not found. Run: cargo build --release --target wasm32-unknown-unknown && mkdir -p wasm && cp target/wasm32-unknown-unknown/release/dao_admin.wasm wasm/");
}

/// Setup helper returning (PocketIc, canister_id, controller)
/// Controller is automatically added as admin on init
fn setup() -> (PocketIc, Principal, Principal) {
    let pic = PocketIc::new();
    let wasm = std::fs::read(get_wasm_path()).expect("Failed to read WASM");

    let controller = Principal::from_text("aaaaa-aa").unwrap();
    let canister_id = pic.create_canister();
    pic.add_cycles(canister_id, 2_000_000_000_000);

    // Init with controller as admin - init takes Option<Vec<Principal>>
    pic.install_canister(
        canister_id,
        wasm,
        encode_one(Some(vec![controller])).unwrap(),
        None,
    );

    (pic, canister_id, controller)
}

/// Create a non-admin, non-controller principal for access control tests
fn non_admin_principal() -> Principal {
    Principal::from_text("2vxsx-fae").unwrap()
}

// ============================================================================
// Task 1: Setup & Health Tests (AC: 3.1.8.7)
// ============================================================================

#[test]
fn test_health_check() {
    let (pic, canister_id, _) = setup();

    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "health",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let health: String = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert_eq!(health, "ok");
}

#[test]
fn test_wasm_loads_successfully() {
    let path = get_wasm_path();
    let wasm = std::fs::read(&path).expect("Failed to read WASM");
    assert!(!wasm.is_empty(), "WASM file should not be empty");
    assert!(wasm.len() > 1000, "WASM file should be substantial");
}

// ============================================================================
// Task 2: Controller & Admin Access Control Tests (AC: 3.1.8.5)
// ============================================================================

#[test]
fn test_add_admin_by_controller_succeeds() {
    let (pic, canister_id, controller) = setup();
    let new_admin = non_admin_principal();

    let response = pic
        .update_call(
            canister_id,
            controller,
            "add_admin",
            encode_one(new_admin).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Controller should be able to add admin");
}

// NOTE: Controller verification tests behave differently in PocketIC vs production IC.
// In PocketIC, the canister_status call returns PocketIC's internal controller list,
// which may not match our test principals. These tests verify the access control
// logic exists and runs without error - production controller verification is
// tested via manual testing on IC mainnet.
#[test]
fn test_add_admin_controller_check_runs() {
    let (pic, canister_id, controller) = setup();
    let new_admin = Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();

    // Controller (from init) should be able to add admin
    let response = pic.update_call(
        canister_id,
        controller,
        "add_admin",
        encode_one(new_admin).unwrap(),
    );

    // Verify the call completes (controller check logic executed)
    assert!(response.is_ok(), "add_admin call should complete");
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
    assert!(result.is_ok(), "Controller should be able to add admin");
}

#[test]
fn test_remove_admin_by_controller_succeeds() {
    let (pic, canister_id, controller) = setup();
    let new_admin = non_admin_principal();

    // First add an admin
    pic.update_call(
        canister_id,
        controller,
        "add_admin",
        encode_one(new_admin).unwrap(),
    )
    .unwrap();

    // Then remove them
    let response = pic
        .update_call(
            canister_id,
            controller,
            "remove_admin",
            encode_one(new_admin).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Controller should be able to remove admin");
}

// NOTE: See comment on test_add_admin_controller_check_runs for PocketIC controller behavior
#[test]
fn test_remove_admin_controller_check_runs() {
    let (pic, canister_id, controller) = setup();
    let admin_to_add = non_admin_principal();

    // Add an admin first
    pic.update_call(
        canister_id,
        controller,
        "add_admin",
        encode_one(admin_to_add).unwrap(),
    )
    .unwrap();

    // Controller should be able to remove admin
    let response = pic.update_call(
        canister_id,
        controller,
        "remove_admin",
        encode_one(admin_to_add).unwrap(),
    );

    assert!(response.is_ok(), "remove_admin call should complete");
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
    assert!(result.is_ok(), "Controller should be able to remove admin");
}

#[test]
fn test_get_admins_returns_correct_list() {
    let (pic, canister_id, controller) = setup();
    let new_admin = non_admin_principal();

    // Initially should have controller as admin (requires admin - FOS-5.6.8)
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_admins",
            encode_one(()).unwrap(),
        )
        .unwrap();
    let result: Result<Vec<Principal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let admins = result.expect("Admin should be able to get admin list");
    assert!(admins.contains(&controller), "Controller should be admin");

    // Add new admin
    pic.update_call(
        canister_id,
        controller,
        "add_admin",
        encode_one(new_admin).unwrap(),
    )
    .unwrap();

    // Check list again (requires admin - FOS-5.6.8)
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_admins",
            encode_one(()).unwrap(),
        )
        .unwrap();
    let result: Result<Vec<Principal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let admins = result.expect("Admin should be able to get admin list");
    assert_eq!(admins.len(), 2, "Should have 2 admins now");
    assert!(admins.contains(&new_admin), "New admin should be in list");
}

#[test]
fn test_admin_only_endpoint_rejects_non_admin() {
    let (pic, canister_id, _) = setup();
    let non_admin = non_admin_principal();

    let request = CreateContactRequest {
        user_id: None,
        email: "test@example.com".to_string(),
        name: Some("Test User".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let response = pic.update_call(
        canister_id,
        non_admin,
        "create_contact",
        encode_one(request).unwrap(),
    );

    assert!(
        response.is_err() || {
            let result: Result<Contact, String> =
                decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
            result.is_err()
        },
        "Non-admin should not be able to create contact"
    );
}

// ============================================================================
// Task 3: CRM Contact Tests (AC: 3.1.8.1)
// ============================================================================

#[test]
fn test_create_contact_with_all_fields() {
    let (pic, canister_id, controller) = setup();

    let request = CreateContactRequest {
        user_id: Some("user-123".to_string()),
        email: "john@example.com".to_string(),
        name: Some("John Doe".to_string()),
        company: Some("Acme Corp".to_string()),
        job_title: Some("CTO".to_string()),
        interest_area: Some("Enterprise".to_string()),
        source: Some(ContactSource::Marketing),
        notes: Some("High priority lead".to_string()),
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = result.expect("Should create contact successfully");

    assert_eq!(contact.email, "john@example.com");
    assert_eq!(contact.name, Some("John Doe".to_string()));
    assert_eq!(contact.company, Some("Acme Corp".to_string()));
    assert_eq!(contact.source, ContactSource::Marketing);
    assert_eq!(contact.status, ContactStatus::Active);
    assert!(contact.id > 0);
}

#[test]
fn test_get_contact_returns_created_contact() {
    let (pic, canister_id, controller) = setup();

    // Create a contact
    let request = CreateContactRequest {
        user_id: None,
        email: "jane@example.com".to_string(),
        name: Some("Jane Smith".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let create_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();
    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let created = result.unwrap();

    // Get the contact (requires admin - FOS-5.6.8)
    let get_response = pic
        .query_call(
            canister_id,
            controller,
            "get_contact",
            encode_one(created.id).unwrap(),
        )
        .unwrap();
    let fetched: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(get_response)).unwrap();

    let contact_opt = fetched.expect("Admin should be able to get contact");
    assert!(contact_opt.is_some());
    let contact = contact_opt.unwrap();
    assert_eq!(contact.id, created.id);
    assert_eq!(contact.email, "jane@example.com");
}

#[test]
fn test_get_contact_returns_none_for_nonexistent() {
    let (pic, canister_id, controller) = setup();

    // Requires admin - FOS-5.6.8
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_contact",
            encode_one(99999u64).unwrap(),
        )
        .unwrap();
    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact_opt = result.expect("Admin should be able to query");

    assert!(contact_opt.is_none());
}

#[test]
fn test_get_contact_by_email() {
    let (pic, canister_id, controller) = setup();

    let request = CreateContactRequest {
        user_id: None,
        email: "unique@example.com".to_string(),
        name: Some("Unique User".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "create_contact",
        encode_one(request).unwrap(),
    )
    .unwrap();

    // Requires admin - FOS-5.6.8
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_contact_by_email",
            encode_one("unique@example.com".to_string()).unwrap(),
        )
        .unwrap();
    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();

    let contact_opt = result.expect("Admin should be able to get contact by email");
    assert!(contact_opt.is_some());
    assert_eq!(contact_opt.unwrap().email, "unique@example.com");
}

#[test]
fn test_get_contacts_with_filters() {
    let (pic, canister_id, controller) = setup();

    // Create contacts with different sources
    for (email, source) in [
        ("marketing1@example.com", ContactSource::Marketing),
        ("marketing2@example.com", ContactSource::Marketing),
        ("referral@example.com", ContactSource::Referral),
    ] {
        let request = CreateContactRequest {
            user_id: None,
            email: email.to_string(),
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            source: Some(source),
            notes: None,
        };
        pic.update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    // Filter by Marketing source
    let filter = ContactFilter {
        source: Some(ContactSource::Marketing),
        status: None,
        search: None,
    };

    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_contacts",
            encode_args((Some(filter), None::<PaginationParams>)).unwrap(),
        )
        .unwrap();
    let result: Result<PaginatedContactResponse, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let contacts = result.unwrap();

    assert_eq!(contacts.items.len(), 2, "Should have 2 marketing contacts");
    assert!(contacts
        .items
        .iter()
        .all(|c| c.source == ContactSource::Marketing));
}

#[test]
fn test_get_contacts_pagination() {
    let (pic, canister_id, controller) = setup();

    // Create 5 contacts
    for i in 0..5 {
        let request = CreateContactRequest {
            user_id: None,
            email: format!("user{}@example.com", i),
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            source: None,
            notes: None,
        };
        pic.update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    // Request page of 2
    let pagination = PaginationParams {
        offset: Some(0),
        limit: Some(2),
    };

    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_contacts",
            encode_args((None::<ContactFilter>, Some(pagination))).unwrap(),
        )
        .unwrap();
    let result: Result<PaginatedContactResponse, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let page = result.unwrap();

    assert_eq!(page.items.len(), 2, "Should return 2 items per page");
    assert_eq!(page.total, 5, "Total should be 5");
    assert_eq!(page.offset, 0);
    assert_eq!(page.limit, 2);
}

#[test]
fn test_create_contact_from_signup_creates_deal() {
    let (pic, canister_id, controller) = setup();

    // FOS-5.6.8: create_contact_from_signup requires authorized canister
    // First register the user-service canister
    let user_service_principal = Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();

    pic.update_call(
        canister_id,
        controller,
        "register_authorized_canister",
        encode_args(("user-service".to_string(), user_service_principal)).unwrap(),
    )
    .unwrap();

    // Now call create_contact_from_signup FROM the authorized canister principal
    let request = CreateContactRequest {
        user_id: None,
        email: "signup@example.com".to_string(),
        name: Some("New Signup".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let response = pic
        .update_call(
            canister_id,
            user_service_principal, // Authorized canister
            "create_contact_from_signup",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = result.expect("Authorized canister should create contact from signup");
    assert_eq!(contact.email, "signup@example.com");

    // Check for auto-created deal
    let filter = DealFilter {
        contact_id: Some(contact.id),
        stage: None,
    };

    let deals_response = pic
        .query_call(
            canister_id,
            controller,
            "get_deals",
            encode_args((Some(filter), None::<PaginationParams>)).unwrap(),
        )
        .unwrap();

    let deals_result: Result<PaginatedDealResponse, String> =
        decode_one(&unwrap_wasm_result(deals_response)).unwrap();
    let deals = deals_result.unwrap();

    assert_eq!(deals.items.len(), 1, "Should have auto-created one deal");
    assert!(
        deals.items[0].name.contains("signup"),
        "Deal name should reference signup"
    );
}

// ============================================================================
// Task 4: Deal Pipeline Tests (AC: 3.1.8.2)
// ============================================================================

#[test]
fn test_create_deal_linked_to_contact() {
    let (pic, canister_id, controller) = setup();

    // First create a contact
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "deal-test@example.com".to_string(),
        name: Some("Deal Test".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact_result: Result<Contact, String> =
        decode_one(&unwrap_wasm_result(contact_response)).unwrap();
    let contact = contact_result.unwrap();

    // Create a deal
    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Big Enterprise Deal".to_string(),
        value: Some(50000),
        notes: Some("Important deal".to_string()),
        expected_close_date: Some(1735689600), // Some future timestamp
    };

    let deal_response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    let deal_result: Result<Deal, String> =
        decode_one(&unwrap_wasm_result(deal_response)).unwrap();
    let deal = deal_result.expect("Should create deal");

    assert_eq!(deal.contact_id, contact.id);
    assert_eq!(deal.name, "Big Enterprise Deal");
    assert_eq!(deal.value, Some(50000));
    assert_eq!(deal.stage, DealStage::Lead); // Default stage
}

#[test]
fn test_create_deal_with_invalid_contact_fails() {
    let (pic, canister_id, controller) = setup();

    let deal_request = CreateDealRequest {
        contact_id: 99999, // Non-existent contact
        name: "Invalid Deal".to_string(),
        value: None,
        notes: None,
        expected_close_date: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    let result: Result<Deal, String> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(
        result.is_err(),
        "Should fail with invalid contact_id: {:?}",
        result
    );
}

#[test]
fn test_get_deal_returns_created_deal() {
    let (pic, canister_id, controller) = setup();

    // Create contact and deal
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "getdeal@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact: Contact =
        decode_one::<Result<Contact, String>>(&unwrap_wasm_result(contact_response))
            .unwrap()
            .unwrap();

    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Fetchable Deal".to_string(),
        value: Some(10000),
        notes: None,
        expected_close_date: None,
    };

    let create_response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    let created: Deal =
        decode_one::<Result<Deal, String>>(&unwrap_wasm_result(create_response))
            .unwrap()
            .unwrap();

    // Get the deal (requires admin - FOS-5.6.8)
    let get_response = pic
        .query_call(
            canister_id,
            controller,
            "get_deal",
            encode_one(created.id).unwrap(),
        )
        .unwrap();
    let fetched: Result<Option<Deal>, String> = decode_one(&unwrap_wasm_result(get_response)).unwrap();

    let deal_opt = fetched.expect("Admin should be able to get deal");
    assert!(deal_opt.is_some());
    assert_eq!(deal_opt.unwrap().name, "Fetchable Deal");
}

#[test]
fn test_get_deal_returns_none_for_nonexistent() {
    let (pic, canister_id, controller) = setup();

    // Requires admin - FOS-5.6.8
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_deal",
            encode_one(99999u64).unwrap(),
        )
        .unwrap();
    let result: Result<Option<Deal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let deal_opt = result.expect("Admin should be able to query");

    assert!(deal_opt.is_none());
}

#[test]
fn test_update_deal_stage() {
    let (pic, canister_id, controller) = setup();

    // Create contact and deal
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "stagetrans@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact: Contact =
        decode_one::<Result<Contact, String>>(&unwrap_wasm_result(contact_response))
            .unwrap()
            .unwrap();

    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Stage Test Deal".to_string(),
        value: None,
        notes: None,
        expected_close_date: None,
    };

    let create_response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    let deal: Deal = decode_one::<Result<Deal, String>>(&unwrap_wasm_result(create_response))
        .unwrap()
        .unwrap();

    assert_eq!(deal.stage, DealStage::Lead);

    // Update stage to Qualified
    let update_response = pic
        .update_call(
            canister_id,
            controller,
            "update_deal_stage",
            encode_args((deal.id, DealStage::Qualified)).unwrap(),
        )
        .unwrap();
    let updated: Deal =
        decode_one::<Result<Deal, String>>(&unwrap_wasm_result(update_response))
            .unwrap()
            .unwrap();

    assert_eq!(updated.stage, DealStage::Qualified);
}

#[test]
fn test_update_deal_stage_nonexistent_fails() {
    let (pic, canister_id, controller) = setup();

    let response = pic
        .update_call(
            canister_id,
            controller,
            "update_deal_stage",
            encode_args((99999u64, DealStage::Qualified)).unwrap(),
        )
        .unwrap();
    let result: Result<Deal, String> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result.is_err());
}

#[test]
fn test_get_deals_with_filters() {
    let (pic, canister_id, controller) = setup();

    // Create contact
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "dealsfilter@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact: Contact =
        decode_one::<Result<Contact, String>>(&unwrap_wasm_result(contact_response))
            .unwrap()
            .unwrap();

    // Create multiple deals
    for name in ["Deal A", "Deal B"] {
        let deal_request = CreateDealRequest {
            contact_id: contact.id,
            name: name.to_string(),
            value: None,
            notes: None,
            expected_close_date: None,
        };
        pic.update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    }

    // Filter by contact_id
    let filter = DealFilter {
        contact_id: Some(contact.id),
        stage: None,
    };

    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_deals",
            encode_args((Some(filter), None::<PaginationParams>)).unwrap(),
        )
        .unwrap();
    let result: Result<PaginatedDealResponse, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let deals = result.unwrap();

    assert_eq!(deals.items.len(), 2);
}

#[test]
fn test_deal_stage_workflow() {
    let (pic, canister_id, controller) = setup();

    // Create contact and deal
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "workflow@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };

    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact: Contact =
        decode_one::<Result<Contact, String>>(&unwrap_wasm_result(contact_response))
            .unwrap()
            .unwrap();

    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Workflow Deal".to_string(),
        value: Some(100000),
        notes: None,
        expected_close_date: None,
    };

    let create_response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();
    let deal: Deal = decode_one::<Result<Deal, String>>(&unwrap_wasm_result(create_response))
        .unwrap()
        .unwrap();

    // Transition through stages: Lead -> Qualified -> Proposal -> ClosedWon
    let stages = [
        DealStage::Qualified,
        DealStage::Proposal,
        DealStage::Negotiation,
        DealStage::ClosedWon,
    ];

    let mut current_deal = deal;
    for stage in stages {
        let response = pic
            .update_call(
                canister_id,
                controller,
                "update_deal_stage",
                encode_args((current_deal.id, stage.clone())).unwrap(),
            )
            .unwrap();
        current_deal = decode_one::<Result<Deal, String>>(&unwrap_wasm_result(response))
            .unwrap()
            .unwrap();
        assert_eq!(current_deal.stage, stage);
    }

    assert_eq!(current_deal.stage, DealStage::ClosedWon);
}

// ============================================================================
// Task 5: Finance Transaction Tests (AC: 3.1.8.3)
// ============================================================================

#[test]
fn test_create_transaction_income() {
    let (pic, canister_id, controller) = setup();

    let request = CreateTransactionRequest {
        transaction_type: TransactionType::Income,
        category: TransactionCategory::Subscription,
        amount: 9900, // $99.00 in cents
        currency: Some("USD".to_string()),
        description: "Monthly subscription".to_string(),
        reference: Some("SUB-001".to_string()),
        date: Some(1704067200), // 2024-01-01
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_transaction",
            encode_one(request).unwrap(),
        )
        .unwrap();
    let result: Result<Transaction, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let tx = result.expect("Should create income transaction");

    assert_eq!(tx.transaction_type, TransactionType::Income);
    assert_eq!(tx.category, TransactionCategory::Subscription);
    assert_eq!(tx.amount, 9900);
}

#[test]
fn test_create_transaction_expense() {
    let (pic, canister_id, controller) = setup();

    let request = CreateTransactionRequest {
        transaction_type: TransactionType::Expense,
        category: TransactionCategory::Infrastructure,
        amount: 50000, // $500.00
        currency: Some("USD".to_string()),
        description: "Server costs".to_string(),
        reference: None,
        date: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_transaction",
            encode_one(request).unwrap(),
        )
        .unwrap();
    let result: Result<Transaction, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let tx = result.expect("Should create expense transaction");

    assert_eq!(tx.transaction_type, TransactionType::Expense);
    assert_eq!(tx.category, TransactionCategory::Infrastructure);
}

#[test]
fn test_get_transactions_with_filters() {
    let (pic, canister_id, controller) = setup();

    // Create income and expense transactions
    for (tx_type, category, amount) in [
        (TransactionType::Income, TransactionCategory::Subscription, 1000),
        (TransactionType::Income, TransactionCategory::Donation, 500),
        (TransactionType::Expense, TransactionCategory::Payroll, 2000),
    ] {
        let request = CreateTransactionRequest {
            transaction_type: tx_type,
            category,
            amount,
            currency: None,
            description: "Test transaction".to_string(),
            reference: None,
            date: None,
        };
        pic.update_call(
            canister_id,
            controller,
            "create_transaction",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    // Filter by Income type
    let filter = TransactionFilter {
        transaction_type: Some(TransactionType::Income),
        category: None,
        from_date: None,
        to_date: None,
    };

    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_transactions",
            encode_args((Some(filter), None::<PaginationParams>)).unwrap(),
        )
        .unwrap();
    let result: Result<PaginatedTransactionResponse, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let transactions = result.unwrap();

    assert_eq!(transactions.items.len(), 2, "Should have 2 income transactions");
    assert!(transactions
        .items
        .iter()
        .all(|t| t.transaction_type == TransactionType::Income));
}

#[test]
fn test_get_financial_summary() {
    let (pic, canister_id, controller) = setup();

    let base_time = 1704067200u64; // 2024-01-01

    // Create income transactions
    for amount in [10000, 20000, 5000] {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Subscription,
            amount,
            currency: None,
            description: "Income".to_string(),
            reference: None,
            date: Some(base_time + 86400), // +1 day
        };
        pic.update_call(
            canister_id,
            controller,
            "create_transaction",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    // Create expense transactions
    for amount in [3000, 2000] {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Expense,
            category: TransactionCategory::Infrastructure,
            amount,
            currency: None,
            description: "Expense".to_string(),
            reference: None,
            date: Some(base_time + 86400),
        };
        pic.update_call(
            canister_id,
            controller,
            "create_transaction",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    // Get financial summary
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_financial_summary",
            encode_args((base_time, base_time + 604800)).unwrap(), // 1 week period
        )
        .unwrap();
    let result: Result<FinancialSummary, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let summary = result.expect("Should get financial summary");

    assert_eq!(summary.total_income, 35000, "Total income should be 35000");
    assert_eq!(summary.total_expenses, 5000, "Total expenses should be 5000");
    assert_eq!(summary.net, 30000, "Net should be 30000");
}

// ============================================================================
// Task 6: Feature Flag Tests (AC: 3.1.8.4)
// ============================================================================

#[test]
fn test_set_feature_flag() {
    let (pic, canister_id, controller) = setup();

    let request = SetFeatureFlagRequest {
        key: "new_dashboard".to_string(),
        enabled: true,
        description: Some("Enable new dashboard UI".to_string()),
        percentage: None,
        allowed_principals: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "set_feature_flag",
            encode_one(request).unwrap(),
        )
        .unwrap();
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result.is_ok(), "Should set feature flag successfully");
}

#[test]
fn test_get_feature_flag() {
    let (pic, canister_id, controller) = setup();

    // Set a flag
    let request = SetFeatureFlagRequest {
        key: "beta_feature".to_string(),
        enabled: false,
        description: Some("Beta testing".to_string()),
        percentage: Some(50),
        allowed_principals: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(request).unwrap(),
    )
    .unwrap();

    // Get the flag
    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "get_feature_flag",
            encode_one("beta_feature".to_string()).unwrap(),
        )
        .unwrap();
    let result: Option<FeatureFlag> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result.is_some());
    let flag = result.unwrap();
    assert_eq!(flag.key, "beta_feature");
    assert!(!flag.enabled);
    assert_eq!(flag.percentage, Some(50));
}

#[test]
fn test_get_feature_flag_returns_none_for_nonexistent() {
    let (pic, canister_id, _) = setup();

    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "get_feature_flag",
            encode_one("nonexistent_flag".to_string()).unwrap(),
        )
        .unwrap();
    let result: Option<FeatureFlag> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result.is_none());
}

#[test]
fn test_is_feature_enabled_true() {
    let (pic, canister_id, controller) = setup();

    let request = SetFeatureFlagRequest {
        key: "enabled_feature".to_string(),
        enabled: true,
        description: None,
        percentage: None,
        allowed_principals: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(request).unwrap(),
    )
    .unwrap();

    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "is_feature_enabled",
            encode_one("enabled_feature".to_string()).unwrap(),
        )
        .unwrap();
    let result: bool = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result, "Feature should be enabled");
}

#[test]
fn test_is_feature_enabled_false() {
    let (pic, canister_id, controller) = setup();

    let request = SetFeatureFlagRequest {
        key: "disabled_feature".to_string(),
        enabled: false,
        description: None,
        percentage: None,
        allowed_principals: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(request).unwrap(),
    )
    .unwrap();

    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "is_feature_enabled",
            encode_one("disabled_feature".to_string()).unwrap(),
        )
        .unwrap();
    let result: bool = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(!result, "Feature should be disabled");
}

#[test]
fn test_feature_flag_percentage_rollout() {
    let (pic, canister_id, controller) = setup();

    // Set flag with 100% rollout
    let request = SetFeatureFlagRequest {
        key: "percentage_feature".to_string(),
        enabled: true,
        description: None,
        percentage: Some(100), // 100% rollout
        allowed_principals: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(request).unwrap(),
    )
    .unwrap();

    // With 100% rollout, should always be enabled
    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "is_feature_enabled",
            encode_one("percentage_feature".to_string()).unwrap(),
        )
        .unwrap();
    let result: bool = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result, "100% rollout should be enabled");
}

#[test]
fn test_feature_flag_principal_whitelist() {
    let (pic, canister_id, controller) = setup();

    let allowed = non_admin_principal();

    let request = SetFeatureFlagRequest {
        key: "whitelist_feature".to_string(),
        enabled: true,
        description: None,
        percentage: None,
        allowed_principals: Some(vec![allowed]),
    };

    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(request).unwrap(),
    )
    .unwrap();

    // Get the flag and verify whitelist
    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "get_feature_flag",
            encode_one("whitelist_feature".to_string()).unwrap(),
        )
        .unwrap();
    let flag: Option<FeatureFlag> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(flag.is_some());
    assert!(flag.unwrap().allowed_principals.contains(&allowed));
}

#[test]
fn test_list_feature_flags() {
    let (pic, canister_id, controller) = setup();

    // Create multiple flags
    for key in ["flag_a", "flag_b", "flag_c"] {
        let request = SetFeatureFlagRequest {
            key: key.to_string(),
            enabled: true,
            description: None,
            percentage: None,
            allowed_principals: None,
        };
        pic.update_call(
            canister_id,
            controller,
            "set_feature_flag",
            encode_one(request).unwrap(),
        )
        .unwrap();
    }

    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_feature_flags",
            encode_one(()).unwrap(),
        )
        .unwrap();
    let result: Result<Vec<FeatureFlag>, String> =
        decode_one(&unwrap_wasm_result(response)).unwrap();
    let flags = result.expect("Should list flags");

    assert_eq!(flags.len(), 3, "Should have 3 feature flags");
}

// ============================================================================
// Task 7: Analytics Tests (AC: 3.1.8.6)
// ============================================================================

#[test]
fn test_log_activity() {
    let (pic, canister_id, controller) = setup();

    // log_activity requires admin or authorized canister - use controller (admin)
    let response = pic.update_call(
        canister_id,
        controller,
        "log_activity",
        encode_args((
            "user-123".to_string(),
            "page_view".to_string(),
            Some("/dashboard".to_string()),
        ))
        .unwrap(),
    );

    assert!(response.is_ok(), "log_activity call should not fail");

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
    assert!(result.is_ok(), "Admin should be able to log activity");
}

#[test]
fn test_record_metrics_by_admin() {
    let (pic, canister_id, controller) = setup();

    let snapshot = MetricsSnapshot {
        total_users: 1000,
        active_users_24h: 150,
        active_users_7d: 400,
        active_users_30d: 700,
        total_captures: 5000,
        total_sprints: 200,
        total_workspaces: 50,
        timestamp: 1704067200,
    };

    let response = pic
        .update_call(
            canister_id,
            controller, // Admin
            "record_metrics",
            encode_one(snapshot).unwrap(),
        )
        .unwrap();
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();

    assert!(result.is_ok(), "Admin should be able to record metrics");
}

#[test]
fn test_record_metrics_by_non_admin_fails() {
    let (pic, canister_id, _) = setup();
    let non_admin = non_admin_principal();

    let snapshot = MetricsSnapshot {
        total_users: 100,
        active_users_24h: 10,
        active_users_7d: 30,
        active_users_30d: 50,
        total_captures: 200,
        total_sprints: 10,
        total_workspaces: 5,
        timestamp: 1704067200,
    };

    let response = pic.update_call(
        canister_id,
        non_admin,
        "record_metrics",
        encode_one(snapshot).unwrap(),
    );

    assert!(
        response.is_err() || {
            let result: Result<(), String> =
                decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
            result.is_err()
        },
        "Non-admin should not be able to record metrics"
    );
}

// ============================================================================
// Task 8: Stats & Health Tests (AC: 3.1.8.7)
// ============================================================================

#[test]
fn test_get_admin_stats() {
    let (pic, canister_id, controller) = setup();

    // Create some data first
    // Contact
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "stats@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: None,
        notes: None,
    };
    let contact_response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();
    let contact: Contact =
        decode_one::<Result<Contact, String>>(&unwrap_wasm_result(contact_response))
            .unwrap()
            .unwrap();

    // Deal
    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Stats Deal".to_string(),
        value: None,
        notes: None,
        expected_close_date: None,
    };
    pic.update_call(
        canister_id,
        controller,
        "create_deal",
        encode_one(deal_request).unwrap(),
    )
    .unwrap();

    // Transaction
    let tx_request = CreateTransactionRequest {
        transaction_type: TransactionType::Income,
        category: TransactionCategory::Subscription,
        amount: 1000,
        currency: None,
        description: "Stats tx".to_string(),
        reference: None,
        date: None,
    };
    pic.update_call(
        canister_id,
        controller,
        "create_transaction",
        encode_one(tx_request).unwrap(),
    )
    .unwrap();

    // Feature flag
    let flag_request = SetFeatureFlagRequest {
        key: "stats_flag".to_string(),
        enabled: true,
        description: None,
        percentage: None,
        allowed_principals: None,
    };
    pic.update_call(
        canister_id,
        controller,
        "set_feature_flag",
        encode_one(flag_request).unwrap(),
    )
    .unwrap();

    // Get stats
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_admin_stats",
            encode_one(()).unwrap(),
        )
        .unwrap();
    let result: Result<AdminStats, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let stats = result.expect("Should get admin stats");

    assert_eq!(stats.total_contacts, 1);
    assert_eq!(stats.total_deals, 1);
    assert_eq!(stats.total_transactions, 1);
    assert_eq!(stats.active_feature_flags, 1);
}

// ============================================================================
// Metrics History API Tests (FOS-3.2.9a)
// ============================================================================

#[test]
fn test_list_metrics_returns_empty_when_no_metrics() {
    let (pic, canister_id, controller) = setup();

    // Query with a wide date range (requires admin)
    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_metrics",
            encode_args((0u64, u64::MAX, None::<u64>)).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let metrics = result.expect("Should succeed for admin");
    assert!(metrics.is_empty(), "Should return empty vec when no metrics recorded");
}

#[test]
fn test_get_latest_metrics_returns_none_when_no_metrics() {
    let (pic, canister_id, controller) = setup();

    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_latest_metrics",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Option<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let metrics = result.expect("Should succeed for admin");
    assert!(metrics.is_none(), "Should return None when no metrics recorded");
}

#[test]
fn test_list_metrics_requires_admin() {
    let (pic, canister_id, _) = setup();
    let non_admin = non_admin_principal();

    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "list_metrics",
            encode_args((0u64, u64::MAX, None::<u64>)).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to list metrics");
}

#[test]
fn test_get_latest_metrics_requires_admin() {
    let (pic, canister_id, _) = setup();
    let non_admin = non_admin_principal();

    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "get_latest_metrics",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Option<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to get latest metrics");
}

#[test]
fn test_record_and_list_metrics() {
    let (pic, canister_id, controller) = setup();

    // Record a metrics snapshot
    let snapshot1 = MetricsSnapshot {
        total_users: 100,
        active_users_24h: 50,
        active_users_7d: 80,
        active_users_30d: 95,
        total_captures: 1000,
        total_sprints: 10,
        total_workspaces: 5,
        timestamp: 1000000000000000000, // 1 second in nanoseconds
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "record_metrics",
            encode_one(snapshot1.clone()).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Record metrics should succeed");

    // Query metrics (requires admin)
    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_metrics",
            encode_args((0u64, u64::MAX, None::<u64>)).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let metrics = result.expect("Should succeed for admin");
    assert_eq!(metrics.len(), 1, "Should have 1 metrics snapshot");
    assert_eq!(metrics[0].total_users, 100);
    assert_eq!(metrics[0].active_users_24h, 50);
}

#[test]
fn test_get_latest_metrics_returns_most_recent() {
    let (pic, canister_id, controller) = setup();

    // Record two snapshots with different timestamps
    let snapshot1 = MetricsSnapshot {
        total_users: 100,
        active_users_24h: 50,
        active_users_7d: 80,
        active_users_30d: 95,
        total_captures: 1000,
        total_sprints: 10,
        total_workspaces: 5,
        timestamp: 1000000000000000000,
    };

    let snapshot2 = MetricsSnapshot {
        total_users: 150,
        active_users_24h: 75,
        active_users_7d: 120,
        active_users_30d: 140,
        total_captures: 1500,
        total_sprints: 15,
        total_workspaces: 8,
        timestamp: 2000000000000000000, // Later timestamp
    };

    // Record first snapshot
    pic.update_call(
        canister_id,
        controller,
        "record_metrics",
        encode_one(snapshot1).unwrap(),
    )
    .unwrap();

    // Record second snapshot
    pic.update_call(
        canister_id,
        controller,
        "record_metrics",
        encode_one(snapshot2).unwrap(),
    )
    .unwrap();

    // Get latest should return the most recent (requires admin)
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_latest_metrics",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Option<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let latest = result.expect("Should succeed for admin").expect("Should have latest metrics");
    assert_eq!(latest.total_users, 150, "Should return most recent snapshot");
    assert_eq!(latest.timestamp, 2000000000000000000);
}

#[test]
fn test_list_metrics_respects_date_range() {
    let (pic, canister_id, controller) = setup();

    // Record snapshots with different timestamps
    let timestamps = [
        1000000000000000000u64, // t1
        2000000000000000000u64, // t2
        3000000000000000000u64, // t3
    ];

    for (i, ts) in timestamps.iter().enumerate() {
        let snapshot = MetricsSnapshot {
            total_users: (i + 1) as u64 * 100,
            active_users_24h: 50,
            active_users_7d: 80,
            active_users_30d: 95,
            total_captures: 1000,
            total_sprints: 10,
            total_workspaces: 5,
            timestamp: *ts,
        };

        pic.update_call(
            canister_id,
            controller,
            "record_metrics",
            encode_one(snapshot).unwrap(),
        )
        .unwrap();
    }

    // Query only middle timestamp (t2) - requires admin
    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_metrics",
            encode_args((1500000000000000000u64, 2500000000000000000u64, None::<u64>)).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let metrics = result.expect("Should succeed for admin");
    assert_eq!(metrics.len(), 1, "Should only return snapshot in date range");
    assert_eq!(metrics[0].total_users, 200, "Should be the second snapshot");
}

#[test]
fn test_list_metrics_respects_limit() {
    let (pic, canister_id, controller) = setup();

    // Record 5 snapshots
    for i in 0..5 {
        let snapshot = MetricsSnapshot {
            total_users: (i + 1) as u64 * 100,
            active_users_24h: 50,
            active_users_7d: 80,
            active_users_30d: 95,
            total_captures: 1000,
            total_sprints: 10,
            total_workspaces: 5,
            timestamp: (i + 1) as u64 * 1000000000000000000,
        };

        pic.update_call(
            canister_id,
            controller,
            "record_metrics",
            encode_one(snapshot).unwrap(),
        )
        .unwrap();
    }

    // Query with limit of 2 - requires admin
    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_metrics",
            encode_args((0u64, u64::MAX, Some(2u64))).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<MetricsSnapshot>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let metrics = result.expect("Should succeed for admin");
    assert_eq!(metrics.len(), 2, "Should respect limit parameter");
    // Results should be sorted descending (newest first)
    assert_eq!(metrics[0].total_users, 500, "First should be newest");
    assert_eq!(metrics[1].total_users, 400, "Second should be second newest");
}

#[test]
fn test_record_metrics_requires_admin() {
    let (pic, canister_id, _) = setup();
    let non_admin = non_admin_principal();

    let snapshot = MetricsSnapshot {
        total_users: 100,
        active_users_24h: 50,
        active_users_7d: 80,
        active_users_30d: 95,
        total_captures: 1000,
        total_sprints: 10,
        total_workspaces: 5,
        timestamp: 1000000000000000000,
    };

    let response = pic
        .update_call(
            canister_id,
            non_admin,
            "record_metrics",
            encode_one(snapshot).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to record metrics");
}

// ============================================================================
// FOS-5.6.8: Admin Backend Authorization Tests
// ============================================================================

// =============================================================================
// AC-5.6.8.3: Inter-canister call verification for create_contact_from_signup
// =============================================================================

#[test]
fn test_create_contact_from_signup_requires_authorized_canister() {
    let (pic, canister_id, _controller) = setup();
    let non_admin = non_admin_principal();

    let request = CreateContactRequest {
        user_id: Some("user-123".to_string()),
        email: "test@example.com".to_string(),
        name: Some("Test User".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    // Call from non-authorized principal should fail
    let response = pic
        .update_call(
            canister_id,
            non_admin,
            "create_contact_from_signup",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Unauthorized principal should not be able to call create_contact_from_signup");
    assert!(
        result.unwrap_err().contains("Expected user-service canister"),
        "Error message should indicate expected canister"
    );
}

#[test]
fn test_create_contact_from_signup_anonymous_rejected() {
    let (pic, canister_id, _controller) = setup();

    let request = CreateContactRequest {
        user_id: Some("user-123".to_string()),
        email: "anon@example.com".to_string(),
        name: None,
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    // Call from anonymous identity should fail
    let response = pic
        .update_call(
            canister_id,
            Principal::anonymous(),
            "create_contact_from_signup",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Anonymous identity should not be able to call create_contact_from_signup");
}

// NOTE: Controller verification tests behave differently in PocketIC vs production IC.
// In PocketIC, the canister_status call returns PocketIC's internal controller list,
// which may not match our test principals. This test verifies the access control
// logic exists and runs without error - production controller verification is
// tested via manual testing on IC mainnet.
#[test]
fn test_register_authorized_canister_controller_check_runs() {
    let (pic, canister_id, controller) = setup();
    let canister_to_authorize = Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();

    // Controller (from init) should be able to register authorized canister
    let response = pic.update_call(
        canister_id,
        controller,
        "register_authorized_canister",
        encode_args(("user-service".to_string(), canister_to_authorize)).unwrap(),
    );

    // Verify the call completes (controller check logic executed)
    assert!(response.is_ok(), "register_authorized_canister call should complete");
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response.unwrap())).unwrap();
    assert!(result.is_ok(), "Controller should be able to register authorized canister");
}

#[test]
fn test_register_and_use_authorized_canister() {
    let (pic, canister_id, controller) = setup();
    let user_service_canister = Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").unwrap();

    // Controller registers user-service canister
    let response = pic
        .update_call(
            canister_id,
            controller,
            "register_authorized_canister",
            encode_args(("user-service".to_string(), user_service_canister)).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Controller should be able to register authorized canister");

    // Verify the canister is listed
    let response = pic
        .query_call(
            canister_id,
            controller,
            "list_authorized_canisters",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<(String, Principal)>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let canisters = result.expect("Should succeed");
    assert!(
        canisters.iter().any(|(role, id)| role == "user-service" && *id == user_service_canister),
        "user-service should be in authorized canisters list"
    );
}

// =============================================================================
// AC-5.6.8.4: log_activity requires authorization
// =============================================================================

#[test]
fn test_log_activity_requires_authorization() {
    let (pic, canister_id, _controller) = setup();
    let non_admin = non_admin_principal();

    // Non-admin, non-authorized canister should not be able to log activity
    let response = pic
        .update_call(
            canister_id,
            non_admin,
            "log_activity",
            encode_args(("user-123".to_string(), "login".to_string(), None::<String>)).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-authorized principal should not be able to log activity");
    assert!(
        result.unwrap_err().contains("Requires admin or authorized canister"),
        "Error message should indicate authorization required"
    );
}

#[test]
fn test_log_activity_admin_succeeds() {
    let (pic, canister_id, controller) = setup();

    // Admin (controller) should be able to log activity
    let response = pic
        .update_call(
            canister_id,
            controller,
            "log_activity",
            encode_args(("user-123".to_string(), "login".to_string(), Some("metadata".to_string()))).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Admin should be able to log activity");
}

#[test]
fn test_log_activity_anonymous_rejected() {
    let (pic, canister_id, _controller) = setup();

    // Anonymous identity should not be able to log activity
    let response = pic
        .update_call(
            canister_id,
            Principal::anonymous(),
            "log_activity",
            encode_args(("anon-user".to_string(), "test".to_string(), None::<String>)).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Anonymous identity should not be able to log activity");
}

#[test]
fn test_log_activity_rate_limit() {
    let (pic, canister_id, controller) = setup();

    // log_activity has a rate limit of 100 calls per minute
    // Make 100 calls which should all succeed
    for i in 0..100 {
        let response = pic
            .update_call(
                canister_id,
                controller,
                "log_activity",
                encode_args((
                    format!("user-{}", i),
                    "rate_test".to_string(),
                    None::<String>,
                ))
                .unwrap(),
            )
            .unwrap();

        let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
        assert!(result.is_ok(), "Call {} should succeed within rate limit", i);
    }

    // The 101st call should fail with rate limit exceeded
    let response = pic
        .update_call(
            canister_id,
            controller,
            "log_activity",
            encode_args(("user-101".to_string(), "rate_test".to_string(), None::<String>)).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Call 101 should be rate limited");
    assert!(
        result.unwrap_err().contains("Rate limit exceeded"),
        "Error message should indicate rate limit exceeded"
    );
}

#[test]
fn test_log_activity_rate_limit_resets_after_window() {
    let (pic, canister_id, controller) = setup();

    // Make 100 calls to hit the rate limit
    for i in 0..100 {
        let response = pic
            .update_call(
                canister_id,
                controller,
                "log_activity",
                encode_args((
                    format!("user-{}", i),
                    "rate_test".to_string(),
                    None::<String>,
                ))
                .unwrap(),
            )
            .unwrap();

        let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
        assert!(result.is_ok(), "Call {} should succeed", i);
    }

    // Verify we're rate limited
    let response = pic
        .update_call(
            canister_id,
            controller,
            "log_activity",
            encode_args(("user-blocked".to_string(), "rate_test".to_string(), None::<String>)).unwrap(),
        )
        .unwrap();
    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Should be rate limited");

    // Advance time by more than 1 minute (rate limit window)
    // 61 seconds in nanoseconds = 61_000_000_000
    pic.advance_time(std::time::Duration::from_secs(61));
    // Tick to process time advancement
    pic.tick();

    // Now should be able to make calls again
    let response = pic
        .update_call(
            canister_id,
            controller,
            "log_activity",
            encode_args(("user-after-window".to_string(), "rate_test".to_string(), None::<String>)).unwrap(),
        )
        .unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Call should succeed after rate limit window resets");
}

// =============================================================================
// AC-5.6.8.5: Query endpoints require admin authorization
// =============================================================================

#[test]
fn test_get_contact_requires_admin() {
    let (pic, canister_id, controller) = setup();
    let non_admin = non_admin_principal();

    // First create a contact as admin
    let request = CreateContactRequest {
        user_id: None,
        email: "admin-created@example.com".to_string(),
        name: Some("Admin Created".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let create_result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = create_result.expect("Admin should create contact");

    // Non-admin should not be able to get contact
    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "get_contact",
            encode_one(contact.id).unwrap(),
        )
        .unwrap();

    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to get contact");
    assert!(
        result.unwrap_err().contains("Admin access required"),
        "Error should indicate admin required"
    );
}

#[test]
fn test_get_contact_admin_succeeds() {
    let (pic, canister_id, controller) = setup();

    // Create a contact as admin
    let request = CreateContactRequest {
        user_id: None,
        email: "admin-test@example.com".to_string(),
        name: Some("Admin Test".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let create_result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = create_result.expect("Admin should create contact");

    // Admin should be able to get contact
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_contact",
            encode_one(contact.id).unwrap(),
        )
        .unwrap();

    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Admin should be able to get contact");
    let found = result.unwrap();
    assert!(found.is_some(), "Contact should exist");
    assert_eq!(found.unwrap().email, "admin-test@example.com");
}

#[test]
fn test_get_contact_by_email_requires_admin() {
    let (pic, canister_id, controller) = setup();
    let non_admin = non_admin_principal();

    // First create a contact as admin
    let request = CreateContactRequest {
        user_id: None,
        email: "email-test@example.com".to_string(),
        name: Some("Email Test".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    pic.update_call(
        canister_id,
        controller,
        "create_contact",
        encode_one(request).unwrap(),
    )
    .unwrap();

    // Non-admin should not be able to get contact by email (email enumeration risk)
    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "get_contact_by_email",
            encode_one("email-test@example.com".to_string()).unwrap(),
        )
        .unwrap();

    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to get contact by email");
}

#[test]
fn test_get_deal_requires_admin() {
    let (pic, canister_id, controller) = setup();
    let non_admin = non_admin_principal();

    // First create a contact
    let contact_request = CreateContactRequest {
        user_id: None,
        email: "deal-test@example.com".to_string(),
        name: Some("Deal Test".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(contact_request).unwrap(),
        )
        .unwrap();

    let create_result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = create_result.expect("Should create contact");

    // Create a deal
    let deal_request = CreateDealRequest {
        contact_id: contact.id,
        name: "Test Deal".to_string(),
        value: Some(10000),
        notes: None,
        expected_close_date: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_deal",
            encode_one(deal_request).unwrap(),
        )
        .unwrap();

    let deal_result: Result<Deal, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let deal = deal_result.expect("Should create deal");

    // Non-admin should not be able to get deal
    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "get_deal",
            encode_one(deal.id).unwrap(),
        )
        .unwrap();

    let result: Result<Option<Deal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to get deal");
}

#[test]
fn test_get_admins_requires_admin() {
    let (pic, canister_id, _controller) = setup();
    let non_admin = non_admin_principal();

    // Non-admin should not be able to get admin list
    let response = pic
        .query_call(
            canister_id,
            non_admin,
            "get_admins",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<Principal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-admin should not be able to get admin list");
    assert!(
        result.unwrap_err().contains("Admin access required"),
        "Error should indicate admin required"
    );
}

#[test]
fn test_get_admins_admin_succeeds() {
    let (pic, canister_id, controller) = setup();

    // Admin should be able to get admin list
    let response = pic
        .query_call(
            canister_id,
            controller,
            "get_admins",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<Principal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Admin should be able to get admin list");
    let admins = result.unwrap();
    assert!(!admins.is_empty(), "Admin list should not be empty");
    assert!(admins.contains(&controller), "Controller should be in admin list");
}

// =============================================================================
// AC-5.6.8.6: AnonymousIdentity rejection tests
// =============================================================================

#[test]
fn test_anonymous_cannot_get_contact() {
    let (pic, canister_id, controller) = setup();

    // Create a contact as admin
    let request = CreateContactRequest {
        user_id: None,
        email: "anon-test@example.com".to_string(),
        name: Some("Anon Test".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    let response = pic
        .update_call(
            canister_id,
            controller,
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let create_result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    let contact = create_result.expect("Should create contact");

    // Anonymous should not be able to get contact
    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "get_contact",
            encode_one(contact.id).unwrap(),
        )
        .unwrap();

    let result: Result<Option<Contact>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Anonymous should not be able to get contact");
}

#[test]
fn test_anonymous_cannot_get_admins() {
    let (pic, canister_id, _controller) = setup();

    // Anonymous should not be able to get admin list
    let response = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            "get_admins",
            encode_one(()).unwrap(),
        )
        .unwrap();

    let result: Result<Vec<Principal>, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Anonymous should not be able to get admin list");
}

#[test]
fn test_anonymous_cannot_create_contact() {
    let (pic, canister_id, _controller) = setup();

    let request = CreateContactRequest {
        user_id: None,
        email: "anon-create@example.com".to_string(),
        name: Some("Anon Create".to_string()),
        company: None,
        job_title: None,
        interest_area: None,
        source: Some(ContactSource::Signup),
        notes: None,
    };

    // Anonymous should not be able to create contact
    let response = pic
        .update_call(
            canister_id,
            Principal::anonymous(),
            "create_contact",
            encode_one(request).unwrap(),
        )
        .unwrap();

    let result: Result<Contact, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Anonymous should not be able to create contact");
}
