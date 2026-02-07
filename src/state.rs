use crate::types::*;
use candid::Principal;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// Time constants
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;

/// Rate limit configuration (FOS-5.6.8)
/// - Window: 1 minute sliding window
/// - Max calls: 100 per caller per window
pub const RATE_LIMIT_WINDOW_NS: u64 = 60 * NANOSECONDS_PER_SECOND;
pub const RATE_LIMIT_MAX_CALLS: usize = 100;

/// State structure for the DAO Admin canister
#[derive(Default)]
pub struct State {
    // Access control
    pub controllers: Vec<Principal>,
    pub admins: Vec<Principal>,
    /// Authorized canisters for inter-canister calls (role -> canister_id)
    /// Roles: "user-service", "auth-service", etc.
    pub authorized_canisters: BTreeMap<String, Principal>,

    /// Granular admin permissions (FOS-5.6.10)
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub admin_permissions: BTreeMap<Principal, Vec<AdminPermission>>,

    /// Rate limiting: caller -> list of timestamps (FOS-5.6.8)
    /// NOTE: Intentionally NOT persisted in StableState - rate limits are ephemeral
    /// and time-bound (1 minute window). Stale timestamps would be invalid after upgrade.
    pub rate_limit_buckets: BTreeMap<Principal, Vec<u64>>,

    // CRM - Contacts
    pub contacts: BTreeMap<ContactId, Contact>,
    pub contacts_by_email: BTreeMap<String, ContactId>,
    pub contacts_by_user: BTreeMap<String, ContactId>,
    pub next_contact_id: ContactId,

    // CRM - Deals
    pub deals: BTreeMap<DealId, Deal>,
    pub deals_by_contact: BTreeMap<ContactId, Vec<DealId>>,
    pub next_deal_id: DealId,

    // Finance - Transactions
    pub transactions: BTreeMap<TransactionId, Transaction>,
    pub next_transaction_id: TransactionId,

    // Analytics
    pub activity_log: Vec<UserActivity>,
    pub metrics_history: Vec<MetricsSnapshot>,

    // Feature Flags
    pub feature_flags: BTreeMap<String, FeatureFlag>,

    // Audit Log (FOS-5.6.10)
    /// @see AC-5.6.10.4, AC-5.6.10.5 - Audit logging
    pub audit_log: Vec<AuditLogEntry>,
    pub next_audit_log_id: u64,
}

impl State {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            admins: Vec::new(),
            authorized_canisters: BTreeMap::new(),
            admin_permissions: BTreeMap::new(),
            rate_limit_buckets: BTreeMap::new(),
            contacts: BTreeMap::new(),
            contacts_by_email: BTreeMap::new(),
            contacts_by_user: BTreeMap::new(),
            next_contact_id: 1,
            deals: BTreeMap::new(),
            deals_by_contact: BTreeMap::new(),
            next_deal_id: 1,
            transactions: BTreeMap::new(),
            next_transaction_id: 1,
            activity_log: Vec::new(),
            metrics_history: Vec::new(),
            feature_flags: BTreeMap::new(),
            audit_log: Vec::new(),
            next_audit_log_id: 1,
        }
    }

    // =========================================================================
    // Rate Limiting (FOS-5.6.8 AC-4.2)
    // =========================================================================

    /// Check rate limit for a caller, returning Ok if allowed or Err with message
    /// Also cleans up expired entries and records the new call if allowed
    pub fn check_rate_limit(&mut self, caller: &Principal) -> Result<(), String> {
        let now = ic_cdk::api::time();
        let window_start = now.saturating_sub(RATE_LIMIT_WINDOW_NS);

        // Get or create the bucket for this caller
        let bucket = self.rate_limit_buckets.entry(*caller).or_default();

        // Remove timestamps older than the window
        bucket.retain(|&ts| ts >= window_start);

        // Check if at limit
        if bucket.len() >= RATE_LIMIT_MAX_CALLS {
            return Err(format!(
                "Rate limit exceeded: {} calls per minute allowed, try again later",
                RATE_LIMIT_MAX_CALLS
            ));
        }

        // Record this call
        bucket.push(now);

        Ok(())
    }

    /// Clean up rate limit buckets for principals with no recent activity
    /// Call periodically to prevent memory bloat
    pub fn cleanup_rate_limits(&mut self) {
        let now = ic_cdk::api::time();
        let window_start = now.saturating_sub(RATE_LIMIT_WINDOW_NS);

        // Remove empty buckets and clean old entries
        self.rate_limit_buckets.retain(|_, bucket| {
            bucket.retain(|&ts| ts >= window_start);
            !bucket.is_empty()
        });
    }

    /// Check if a principal is a controller
    pub fn is_controller(&self, principal: &Principal) -> bool {
        self.controllers.contains(principal)
    }

    /// Check if a principal is an admin
    pub fn is_admin(&self, principal: &Principal) -> bool {
        self.admins.contains(principal) || self.controllers.contains(principal)
    }

    /// Add an admin
    pub fn add_admin(&mut self, principal: Principal) {
        if !self.admins.contains(&principal) {
            self.admins.push(principal);
        }
    }

    /// Remove an admin
    pub fn remove_admin(&mut self, principal: &Principal) {
        self.admins.retain(|p| p != principal);
    }

    /// Register an authorized canister for inter-canister calls
    pub fn register_authorized_canister(&mut self, role: String, canister_id: Principal) {
        self.authorized_canisters.insert(role, canister_id);
    }

    /// Unregister an authorized canister
    pub fn unregister_authorized_canister(&mut self, role: &str) {
        self.authorized_canisters.remove(role);
    }

    /// Check if a principal is an authorized canister for the given role
    pub fn is_authorized_canister(&self, role: &str, principal: &Principal) -> bool {
        self.authorized_canisters
            .get(role)
            .map_or(false, |expected| expected == principal)
    }

    /// Check if a principal is any authorized canister
    pub fn is_any_authorized_canister(&self, principal: &Principal) -> bool {
        self.authorized_canisters.values().any(|p| p == principal)
    }

    /// Get all authorized canisters
    pub fn get_authorized_canisters(&self) -> Vec<(String, Principal)> {
        self.authorized_canisters
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    // =========================================================================
    // Permission Operations (FOS-5.6.10)
    // =========================================================================

    /// Check if a principal has a specific permission
    /// Controllers have all permissions implicitly
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub fn has_permission(&self, principal: &Principal, permission: &AdminPermission) -> bool {
        // Controllers have all permissions
        if self.controllers.contains(principal) {
            return true;
        }

        // Check explicit permissions
        self.admin_permissions
            .get(principal)
            .map_or(false, |perms| perms.contains(permission))
    }

    /// Grant a permission to a principal
    pub fn grant_permission(&mut self, principal: Principal, permission: AdminPermission) {
        self.admin_permissions
            .entry(principal)
            .or_default()
            .push(permission);
    }

    /// Revoke a permission from a principal
    pub fn revoke_permission(&mut self, principal: &Principal, permission: &AdminPermission) {
        if let Some(perms) = self.admin_permissions.get_mut(principal) {
            perms.retain(|p| p != permission);
        }
    }

    /// Get all permissions for a principal
    pub fn get_permissions(&self, principal: &Principal) -> Vec<AdminPermission> {
        self.admin_permissions
            .get(principal)
            .cloned()
            .unwrap_or_default()
    }

    /// Grant default permissions to a new admin (view own + edit own)
    pub fn grant_default_permissions(&mut self, principal: Principal) {
        let default_perms = vec![
            AdminPermission::ViewOwnContacts,
            AdminPermission::EditOwnContacts,
            AdminPermission::ViewOwnDeals,
            AdminPermission::EditOwnDeals,
        ];

        for perm in default_perms {
            if !self.has_permission(&principal, &perm) {
                self.grant_permission(principal, perm);
            }
        }
    }

    // =========================================================================
    // Audit Log Operations (FOS-5.6.10)
    // =========================================================================

    /// Record an audit log entry
    /// @see AC-5.6.10.4, AC-5.6.10.5 - Audit logging
    pub fn record_audit_log(
        &mut self,
        actor: Principal,
        action: &str,
        target_type: &str,
        target_id: &str,
        details: Option<String>,
    ) {
        let entry = AuditLogEntry {
            id: self.next_audit_log_id,
            timestamp: ic_cdk::api::time(),
            actor,
            action: action.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            details,
        };

        self.next_audit_log_id += 1;
        self.audit_log.push(entry);

        // Keep only last 10000 entries
        if self.audit_log.len() > 10000 {
            self.audit_log.drain(0..1000);
        }
    }

    /// Get audit log entries with optional filtering
    pub fn get_audit_log(
        &self,
        action_filter: Option<&str>,
        target_type_filter: Option<&str>,
        actor_filter: Option<&Principal>,
        limit: Option<u64>,
    ) -> Vec<AuditLogEntry> {
        let limit = limit.unwrap_or(100) as usize;

        self.audit_log
            .iter()
            .rev()
            .filter(|entry| {
                action_filter.map_or(true, |a| entry.action == a)
                    && target_type_filter.map_or(true, |t| entry.target_type == t)
                    && actor_filter.map_or(true, |p| &entry.actor == p)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    // =========================================================================
    // Contact Operations
    // =========================================================================

    /// Create a new contact
    /// @see AC-5.6.10.1 - Sets owner_id to caller for row-level security
    pub fn create_contact(&mut self, request: CreateContactRequest, caller: Principal) -> Contact {
        let now = ic_cdk::api::time();
        let id = self.next_contact_id;
        self.next_contact_id += 1;

        let contact = Contact {
            id,
            user_id: request.user_id.clone(),
            email: request.email.clone(),
            name: request.name,
            company: request.company,
            job_title: request.job_title,
            interest_area: request.interest_area,
            source: request.source.unwrap_or_default(),
            notes: request.notes,
            status: ContactStatus::Active,
            owner_id: Some(caller),
            team_id: None,
            created_at: now,
            updated_at: now,
        };

        self.contacts.insert(id, contact.clone());
        self.contacts_by_email.insert(request.email.to_lowercase(), id);
        if let Some(ref user_id) = request.user_id {
            self.contacts_by_user.insert(user_id.clone(), id);
        }

        contact
    }

    /// Get a contact by ID
    pub fn get_contact(&self, id: ContactId) -> Option<&Contact> {
        self.contacts.get(&id)
    }

    /// Get a contact by email
    pub fn get_contact_by_email(&self, email: &str) -> Option<&Contact> {
        self.contacts_by_email
            .get(&email.to_lowercase())
            .and_then(|id| self.contacts.get(id))
    }

    /// Update a contact
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub fn update_contact(
        &mut self,
        id: ContactId,
        name: Option<String>,
        company: Option<String>,
        job_title: Option<String>,
        interest_area: Option<String>,
        notes: Option<String>,
        status: Option<ContactStatus>,
    ) -> Option<Contact> {
        let contact = self.contacts.get_mut(&id)?;

        if let Some(n) = name {
            contact.name = Some(n);
        }
        if let Some(c) = company {
            contact.company = Some(c);
        }
        if let Some(j) = job_title {
            contact.job_title = Some(j);
        }
        if let Some(i) = interest_area {
            contact.interest_area = Some(i);
        }
        if let Some(n) = notes {
            contact.notes = Some(n);
        }
        if let Some(s) = status {
            contact.status = s;
        }

        contact.updated_at = ic_cdk::api::time();
        Some(contact.clone())
    }

    /// Delete a contact
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub fn delete_contact(&mut self, id: ContactId) -> Option<Contact> {
        let contact = self.contacts.remove(&id)?;

        // Remove from indexes
        self.contacts_by_email.remove(&contact.email.to_lowercase());
        if let Some(ref user_id) = contact.user_id {
            self.contacts_by_user.remove(user_id);
        }

        // Remove associated deals
        if let Some(deal_ids) = self.deals_by_contact.remove(&id) {
            for deal_id in deal_ids {
                self.deals.remove(&deal_id);
            }
        }

        Some(contact)
    }

    /// Get contacts with filter and row-level security
    /// @see AC-5.6.10.1 - Row-level security filtering
    pub fn get_contacts(
        &self,
        filter: Option<ContactFilter>,
        pagination: PaginationParams,
        caller: &Principal,
    ) -> PaginatedResponse<Contact> {
        let has_view_all = self.has_permission(caller, &AdminPermission::ViewAllContacts);
        let has_view_own = self.has_permission(caller, &AdminPermission::ViewOwnContacts);

        // If no view permissions, return empty
        if !has_view_all && !has_view_own {
            return PaginatedResponse {
                items: Vec::new(),
                total: 0,
                offset: pagination.offset.unwrap_or(0),
                limit: pagination.limit.unwrap_or(50),
            };
        }

        let mut contacts: Vec<Contact> = self.contacts.values().cloned().collect();

        // Apply row-level security if not ViewAllContacts
        if !has_view_all {
            contacts.retain(|c| c.owner_id.as_ref() == Some(caller));
        }

        if let Some(ref f) = filter {
            if let Some(ref status) = f.status {
                contacts.retain(|c| &c.status == status);
            }
            if let Some(ref source) = f.source {
                contacts.retain(|c| &c.source == source);
            }
            if let Some(ref search) = f.search {
                let search_lower = search.to_lowercase();
                contacts.retain(|c| {
                    c.email.to_lowercase().contains(&search_lower)
                        || c.name.as_ref().map_or(false, |n| n.to_lowercase().contains(&search_lower))
                        || c.company.as_ref().map_or(false, |co| co.to_lowercase().contains(&search_lower))
                });
            }
        }

        let total = contacts.len() as u64;
        let offset = pagination.offset.unwrap_or(0);
        let limit = pagination.limit.unwrap_or(50);

        let items: Vec<Contact> = contacts
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        PaginatedResponse {
            items,
            total,
            offset,
            limit,
        }
    }

    // =========================================================================
    // Deal Operations
    // =========================================================================

    /// Create a new deal
    /// @see AC-5.6.10.1 - Sets owner_id and created_by to caller for row-level security
    pub fn create_deal(&mut self, request: CreateDealRequest, caller: Principal) -> Result<Deal, String> {
        if !self.contacts.contains_key(&request.contact_id) {
            return Err("Contact not found".to_string());
        }

        let now = ic_cdk::api::time();
        let id = self.next_deal_id;
        self.next_deal_id += 1;

        let deal = Deal {
            id,
            contact_id: request.contact_id,
            name: request.name,
            value: request.value,
            stage: DealStage::Lead,
            notes: request.notes,
            expected_close_date: request.expected_close_date,
            owner_id: Some(caller),
            created_by: Some(caller),
            created_at: now,
            updated_at: now,
        };

        self.deals.insert(id, deal.clone());
        self.deals_by_contact
            .entry(request.contact_id)
            .or_default()
            .push(id);

        Ok(deal)
    }

    /// Get a deal by ID
    pub fn get_deal(&self, id: DealId) -> Option<&Deal> {
        self.deals.get(&id)
    }

    /// Update deal stage
    pub fn update_deal_stage(&mut self, id: DealId, stage: DealStage) -> Option<Deal> {
        let deal = self.deals.get_mut(&id)?;
        deal.stage = stage;
        deal.updated_at = ic_cdk::api::time();
        Some(deal.clone())
    }

    /// Update a deal
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub fn update_deal(
        &mut self,
        id: DealId,
        name: Option<String>,
        value: Option<u64>,
        stage: Option<DealStage>,
        notes: Option<String>,
        expected_close_date: Option<Timestamp>,
    ) -> Option<Deal> {
        let deal = self.deals.get_mut(&id)?;

        if let Some(n) = name {
            deal.name = n;
        }
        if let Some(v) = value {
            deal.value = Some(v);
        }
        if let Some(s) = stage {
            deal.stage = s;
        }
        if let Some(n) = notes {
            deal.notes = Some(n);
        }
        if let Some(d) = expected_close_date {
            deal.expected_close_date = Some(d);
        }

        deal.updated_at = ic_cdk::api::time();
        Some(deal.clone())
    }

    /// Delete a deal
    /// @see AC-5.6.10.3 - Granular CRUD permissions
    pub fn delete_deal(&mut self, id: DealId) -> Option<Deal> {
        let deal = self.deals.remove(&id)?;

        // Remove from contact's deal list
        if let Some(deal_ids) = self.deals_by_contact.get_mut(&deal.contact_id) {
            deal_ids.retain(|&did| did != id);
        }

        Some(deal)
    }

    /// Get deals with filter and row-level security
    /// @see AC-5.6.10.1 - Row-level security filtering
    pub fn get_deals(
        &self,
        filter: Option<DealFilter>,
        pagination: PaginationParams,
        caller: &Principal,
    ) -> PaginatedResponse<Deal> {
        let has_view_all = self.has_permission(caller, &AdminPermission::ViewAllDeals);
        let has_view_own = self.has_permission(caller, &AdminPermission::ViewOwnDeals);

        // If no view permissions, return empty
        if !has_view_all && !has_view_own {
            return PaginatedResponse {
                items: Vec::new(),
                total: 0,
                offset: pagination.offset.unwrap_or(0),
                limit: pagination.limit.unwrap_or(50),
            };
        }

        let mut deals: Vec<Deal> = self.deals.values().cloned().collect();

        // Apply row-level security if not ViewAllDeals
        if !has_view_all {
            deals.retain(|d| d.owner_id.as_ref() == Some(caller));
        }

        if let Some(ref f) = filter {
            if let Some(ref stage) = f.stage {
                deals.retain(|d| &d.stage == stage);
            }
            if let Some(contact_id) = f.contact_id {
                deals.retain(|d| d.contact_id == contact_id);
            }
        }

        let total = deals.len() as u64;
        let offset = pagination.offset.unwrap_or(0);
        let limit = pagination.limit.unwrap_or(50);

        let items: Vec<Deal> = deals
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        PaginatedResponse {
            items,
            total,
            offset,
            limit,
        }
    }

    // =========================================================================
    // Transaction Operations
    // =========================================================================

    /// Create a new transaction
    pub fn create_transaction(&mut self, request: CreateTransactionRequest) -> Transaction {
        let now = ic_cdk::api::time();
        let id = self.next_transaction_id;
        self.next_transaction_id += 1;

        let transaction = Transaction {
            id,
            transaction_type: request.transaction_type,
            category: request.category,
            amount: request.amount,
            currency: request.currency.unwrap_or_else(|| "USD".to_string()),
            description: request.description,
            reference: request.reference,
            date: request.date.unwrap_or(now),
            created_at: now,
        };

        self.transactions.insert(id, transaction.clone());
        transaction
    }

    /// Get transactions with filter
    pub fn get_transactions(
        &self,
        filter: Option<TransactionFilter>,
        pagination: PaginationParams,
    ) -> PaginatedResponse<Transaction> {
        let mut transactions: Vec<Transaction> = self.transactions.values().cloned().collect();

        if let Some(ref f) = filter {
            if let Some(ref t_type) = f.transaction_type {
                transactions.retain(|t| &t.transaction_type == t_type);
            }
            if let Some(ref category) = f.category {
                transactions.retain(|t| &t.category == category);
            }
            if let Some(from) = f.from_date {
                transactions.retain(|t| t.date >= from);
            }
            if let Some(to) = f.to_date {
                transactions.retain(|t| t.date <= to);
            }
        }

        let total = transactions.len() as u64;
        let offset = pagination.offset.unwrap_or(0);
        let limit = pagination.limit.unwrap_or(50);

        let items: Vec<Transaction> = transactions
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        PaginatedResponse {
            items,
            total,
            offset,
            limit,
        }
    }

    /// Get financial summary
    pub fn get_financial_summary(&self, from: Timestamp, to: Timestamp) -> FinancialSummary {
        let mut total_income: u64 = 0;
        let mut total_expenses: u64 = 0;
        let mut subscription_income: u64 = 0;

        for t in self.transactions.values() {
            if t.date >= from && t.date <= to {
                match t.transaction_type {
                    TransactionType::Income => {
                        total_income += t.amount;
                        if t.category == TransactionCategory::Subscription {
                            subscription_income += t.amount;
                        }
                    }
                    TransactionType::Expense => {
                        total_expenses += t.amount;
                    }
                }
            }
        }

        FinancialSummary {
            total_income,
            total_expenses,
            net: (total_income as i64) - (total_expenses as i64),
            mrr: subscription_income / 12,
            period_start: from,
            period_end: to,
        }
    }

    // =========================================================================
    // Feature Flag Operations
    // =========================================================================

    /// Set a feature flag
    pub fn set_feature_flag(&mut self, request: SetFeatureFlagRequest) {
        let now = ic_cdk::api::time();

        let flag = FeatureFlag {
            key: request.key.clone(),
            enabled: request.enabled,
            description: request.description,
            percentage: request.percentage,
            allowed_principals: request.allowed_principals.unwrap_or_default(),
            updated_at: now,
        };

        self.feature_flags.insert(request.key, flag);
    }

    /// Get a feature flag
    pub fn get_feature_flag(&self, key: &str) -> Option<&FeatureFlag> {
        self.feature_flags.get(key)
    }

    /// Check if a feature is enabled for a principal
    pub fn is_feature_enabled(&self, key: &str, principal: &Principal) -> bool {
        match self.feature_flags.get(key) {
            Some(flag) => {
                if !flag.enabled {
                    return false;
                }

                // Check allowed principals
                if !flag.allowed_principals.is_empty() {
                    return flag.allowed_principals.contains(principal);
                }

                // Check percentage rollout
                if let Some(pct) = flag.percentage {
                    // Simple hash-based rollout
                    let hash = principal.as_slice().iter().fold(0u64, |acc, b| acc.wrapping_add(*b as u64));
                    return (hash % 100) < (pct as u64);
                }

                true
            }
            None => false,
        }
    }

    /// List all feature flags
    pub fn list_feature_flags(&self) -> Vec<FeatureFlag> {
        self.feature_flags.values().cloned().collect()
    }

    // =========================================================================
    // Analytics Operations
    // =========================================================================

    // =========================================================================
    // Migration Operations (FOS-5.6.10)
    // =========================================================================

    /// Migrate existing contacts and deals without owner_id
    /// Sets owner_id to the first admin if not already set
    /// @see AC-5.6.10.1 - Migration for row-level security
    pub fn migrate_ownership(&mut self) {
        let first_admin = self.admins.first().cloned();

        if let Some(admin) = first_admin {
            // Migrate contacts
            for contact in self.contacts.values_mut() {
                if contact.owner_id.is_none() {
                    contact.owner_id = Some(admin);
                }
            }

            // Migrate deals
            for deal in self.deals.values_mut() {
                if deal.owner_id.is_none() {
                    deal.owner_id = Some(admin);
                }
                if deal.created_by.is_none() {
                    deal.created_by = Some(admin);
                }
            }

            ic_cdk::println!("Migrated ownership for {} contacts and {} deals to admin {}",
                self.contacts.len(), self.deals.len(), admin);
        } else {
            ic_cdk::println!("No admins found, skipping ownership migration");
        }
    }

    // =========================================================================
    // Analytics Operations
    // =========================================================================

    /// Log user activity
    pub fn log_activity(&mut self, user_id: String, action: String, metadata: Option<String>) {
        let activity = UserActivity {
            user_id,
            action,
            metadata,
            timestamp: ic_cdk::api::time(),
        };

        self.activity_log.push(activity);

        // Keep only last 10000 entries
        if self.activity_log.len() > 10000 {
            self.activity_log.drain(0..1000);
        }
    }

    /// Record metrics snapshot
    pub fn record_metrics(&mut self, snapshot: MetricsSnapshot) {
        self.metrics_history.push(snapshot);

        // Keep only last 365 entries
        if self.metrics_history.len() > 365 {
            self.metrics_history.drain(0..30);
        }
    }

    /// List metrics within a date range
    pub fn list_metrics(&self, from: Timestamp, to: Timestamp, limit: Option<u64>) -> Vec<MetricsSnapshot> {
        let limit = limit.unwrap_or(100) as usize;

        let mut filtered: Vec<MetricsSnapshot> = self
            .metrics_history
            .iter()
            .filter(|m| m.timestamp >= from && m.timestamp <= to)
            .cloned()
            .collect();

        // Sort by timestamp descending (newest first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        filtered.truncate(limit);
        filtered
    }

    /// Get the most recent metrics snapshot
    pub fn get_latest_metrics(&self) -> Option<MetricsSnapshot> {
        self.metrics_history.last().cloned()
    }
}

thread_local! {
    pub static STATE: RefCell<State> = RefCell::new(State::new());
}

/// Serializable state for upgrades
#[derive(candid::CandidType, serde::Deserialize, Clone)]
pub struct StableState {
    pub controllers: Vec<Principal>,
    pub admins: Vec<Principal>,
    #[serde(default)]
    pub authorized_canisters: Vec<(String, Principal)>,
    /// Admin permissions (FOS-5.6.10)
    #[serde(default)]
    pub admin_permissions: Vec<(Principal, Vec<AdminPermission>)>,
    pub contacts: Vec<(ContactId, Contact)>,
    pub next_contact_id: ContactId,
    pub deals: Vec<(DealId, Deal)>,
    pub next_deal_id: DealId,
    pub transactions: Vec<(TransactionId, Transaction)>,
    pub next_transaction_id: TransactionId,
    pub feature_flags: Vec<(String, FeatureFlag)>,
    #[serde(default)]
    pub metrics_history: Vec<MetricsSnapshot>,
    /// Audit log (FOS-5.6.10)
    #[serde(default)]
    pub audit_log: Vec<AuditLogEntry>,
    #[serde(default)]
    pub next_audit_log_id: u64,
}

impl From<&State> for StableState {
    fn from(state: &State) -> Self {
        StableState {
            controllers: state.controllers.clone(),
            admins: state.admins.clone(),
            authorized_canisters: state.authorized_canisters.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            admin_permissions: state.admin_permissions.iter().map(|(k, v)| (*k, v.clone())).collect(),
            contacts: state.contacts.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_contact_id: state.next_contact_id,
            deals: state.deals.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_deal_id: state.next_deal_id,
            transactions: state.transactions.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_transaction_id: state.next_transaction_id,
            feature_flags: state.feature_flags.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            metrics_history: state.metrics_history.clone(),
            audit_log: state.audit_log.clone(),
            next_audit_log_id: state.next_audit_log_id,
        }
    }
}

impl From<StableState> for State {
    fn from(stable: StableState) -> Self {
        let mut state = State {
            controllers: stable.controllers,
            admins: stable.admins,
            authorized_canisters: stable.authorized_canisters.iter().cloned().collect(),
            admin_permissions: stable.admin_permissions.iter().cloned().collect(),
            contacts: stable.contacts.iter().cloned().collect(),
            next_contact_id: stable.next_contact_id,
            deals: stable.deals.iter().cloned().collect(),
            next_deal_id: stable.next_deal_id,
            transactions: stable.transactions.iter().cloned().collect(),
            next_transaction_id: stable.next_transaction_id,
            feature_flags: stable.feature_flags.iter().cloned().collect(),
            metrics_history: stable.metrics_history,
            audit_log: stable.audit_log,
            next_audit_log_id: if stable.next_audit_log_id == 0 { 1 } else { stable.next_audit_log_id },
            ..Default::default()
        };

        // Rebuild indexes
        for (id, contact) in &state.contacts {
            state.contacts_by_email.insert(contact.email.to_lowercase(), *id);
            if let Some(ref user_id) = contact.user_id {
                state.contacts_by_user.insert(user_id.clone(), *id);
            }
        }

        for (id, deal) in &state.deals {
            state.deals_by_contact
                .entry(deal.contact_id)
                .or_default()
                .push(*id);
        }

        state
    }
}
