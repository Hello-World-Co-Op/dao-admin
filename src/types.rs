use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

pub type ContactId = u64;
pub type DealId = u64;
pub type TransactionId = u64;
pub type Timestamp = u64;

// =============================================================================
// CRM - Contact Types
// =============================================================================

/// Contact source - how the contact was acquired
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Default)]
pub enum ContactSource {
    #[default]
    Signup,
    Referral,
    Marketing,
    Event,
    Partner,
    Other,
}

/// Contact status
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Default)]
pub enum ContactStatus {
    #[default]
    Active,
    Inactive,
    Churned,
}

/// Contact record
/// @see AC-5.6.10.1 - Row-level security: contacts have owner_id for filtering
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Contact {
    pub id: ContactId,
    pub user_id: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub interest_area: Option<String>,
    pub source: ContactSource,
    pub notes: Option<String>,
    pub status: ContactStatus,
    /// Owner of this contact record (admin who created it)
    /// @see FOS-5.6.10 - Row-level security
    #[serde(default)]
    pub owner_id: Option<Principal>,
    /// Team ID for future team-based filtering
    #[serde(default)]
    pub team_id: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a contact
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateContactRequest {
    pub user_id: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub interest_area: Option<String>,
    pub source: Option<ContactSource>,
    pub notes: Option<String>,
}

/// Request to update a contact
/// @see AC-5.6.10.3 - Granular CRUD permissions
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateContactRequest {
    pub id: ContactId,
    pub name: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub interest_area: Option<String>,
    pub notes: Option<String>,
    pub status: Option<ContactStatus>,
}

// =============================================================================
// CRM - Deal Types
// =============================================================================

/// Deal stage in pipeline
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Default)]
pub enum DealStage {
    #[default]
    Lead,
    Qualified,
    Proposal,
    Negotiation,
    ClosedWon,
    ClosedLost,
}

/// Deal record
/// @see AC-5.6.10.1 - Row-level security: deals have owner_id for filtering
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Deal {
    pub id: DealId,
    pub contact_id: ContactId,
    pub name: String,
    pub value: Option<u64>,
    pub stage: DealStage,
    pub notes: Option<String>,
    pub expected_close_date: Option<Timestamp>,
    /// Owner of this deal record (admin who created it)
    /// @see FOS-5.6.10 - Row-level security
    #[serde(default)]
    pub owner_id: Option<Principal>,
    /// Principal who created the deal (for audit trail)
    #[serde(default)]
    pub created_by: Option<Principal>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a deal
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateDealRequest {
    pub contact_id: ContactId,
    pub name: String,
    pub value: Option<u64>,
    pub notes: Option<String>,
    pub expected_close_date: Option<Timestamp>,
}

/// Request to update a deal
/// @see AC-5.6.10.3 - Granular CRUD permissions
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateDealRequest {
    pub id: DealId,
    pub name: Option<String>,
    pub value: Option<u64>,
    pub stage: Option<DealStage>,
    pub notes: Option<String>,
    pub expected_close_date: Option<Timestamp>,
}

// =============================================================================
// Finance - Transaction Types
// =============================================================================

/// Transaction type
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum TransactionType {
    Income,
    Expense,
}

/// Transaction category
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Default)]
pub enum TransactionCategory {
    Subscription,
    Donation,
    Service,
    Infrastructure,
    Marketing,
    Payroll,
    Legal,
    #[default]
    Other,
}

/// Transaction record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub transaction_type: TransactionType,
    pub category: TransactionCategory,
    pub amount: u64,
    pub currency: String,
    pub description: String,
    pub reference: Option<String>,
    pub date: Timestamp,
    pub created_at: Timestamp,
}

/// Request to create a transaction
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateTransactionRequest {
    pub transaction_type: TransactionType,
    pub category: TransactionCategory,
    pub amount: u64,
    pub currency: Option<String>,
    pub description: String,
    pub reference: Option<String>,
    pub date: Option<Timestamp>,
}

// =============================================================================
// Analytics - Metrics Types
// =============================================================================

/// User activity record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct UserActivity {
    pub user_id: String,
    pub action: String,
    pub metadata: Option<String>,
    pub timestamp: Timestamp,
}

/// Platform metrics snapshot
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct MetricsSnapshot {
    pub total_users: u64,
    pub active_users_24h: u64,
    pub active_users_7d: u64,
    pub active_users_30d: u64,
    pub total_captures: u64,
    pub total_sprints: u64,
    pub total_workspaces: u64,
    pub timestamp: Timestamp,
}

// =============================================================================
// Feature Flags
// =============================================================================

/// Feature flag record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct FeatureFlag {
    pub key: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub percentage: Option<u8>,
    pub allowed_principals: Vec<Principal>,
    pub updated_at: Timestamp,
}

/// Request to set feature flag
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct SetFeatureFlagRequest {
    pub key: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub percentage: Option<u8>,
    pub allowed_principals: Option<Vec<Principal>>,
}

// =============================================================================
// Query Types
// =============================================================================

/// Contact filter
#[derive(Clone, Debug, CandidType, Deserialize, Default)]
pub struct ContactFilter {
    pub status: Option<ContactStatus>,
    pub source: Option<ContactSource>,
    pub search: Option<String>,
}

/// Deal filter
#[derive(Clone, Debug, CandidType, Deserialize, Default)]
pub struct DealFilter {
    pub stage: Option<DealStage>,
    pub contact_id: Option<ContactId>,
}

/// Transaction filter
#[derive(Clone, Debug, CandidType, Deserialize, Default)]
pub struct TransactionFilter {
    pub transaction_type: Option<TransactionType>,
    pub category: Option<TransactionCategory>,
    pub from_date: Option<Timestamp>,
    pub to_date: Option<Timestamp>,
}

/// Pagination params
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct PaginationParams {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(50),
        }
    }
}

/// Paginated response wrapper
#[derive(Clone, Debug, CandidType, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

/// Financial summary
#[derive(Clone, Debug, CandidType, Serialize)]
pub struct FinancialSummary {
    pub total_income: u64,
    pub total_expenses: u64,
    pub net: i64,
    pub mrr: u64,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
}

// =============================================================================
// Admin Permissions (FOS-5.6.10)
// =============================================================================

/// Granular admin permissions for row-level security
/// @see AC-5.6.10.1 - Row-level security filtering
/// @see AC-5.6.10.3 - Granular CRUD permissions
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum AdminPermission {
    // Contact permissions
    ViewOwnContacts,
    ViewAllContacts,
    EditOwnContacts,
    EditAllContacts,
    DeleteOwnContacts,
    DeleteAllContacts,
    // Deal permissions
    ViewOwnDeals,
    ViewAllDeals,
    EditOwnDeals,
    EditAllDeals,
    DeleteOwnDeals,
    DeleteAllDeals,
    // Other admin permissions
    ManageFeatureFlags,
    ViewAuditLogs,
}

/// Audit log entry for tracking admin actions
/// @see AC-5.6.10.4 - CRM audit logging
/// @see AC-5.6.10.5 - Feature flag audit logging
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct AuditLogEntry {
    pub id: u64,
    pub timestamp: Timestamp,
    pub actor: Principal,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub details: Option<String>,
}
