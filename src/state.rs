use crate::types::*;
use candid::Principal;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// State structure for the DAO Admin canister
#[derive(Default)]
pub struct State {
    // Access control
    pub controllers: Vec<Principal>,
    pub admins: Vec<Principal>,

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
}

impl State {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            admins: Vec::new(),
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
        }
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

    // =========================================================================
    // Contact Operations
    // =========================================================================

    /// Create a new contact
    pub fn create_contact(&mut self, request: CreateContactRequest) -> Contact {
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

    /// Get contacts with filter
    pub fn get_contacts(
        &self,
        filter: Option<ContactFilter>,
        pagination: PaginationParams,
    ) -> PaginatedResponse<Contact> {
        let mut contacts: Vec<Contact> = self.contacts.values().cloned().collect();

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
    pub fn create_deal(&mut self, request: CreateDealRequest) -> Result<Deal, String> {
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

    /// Get deals with filter
    pub fn get_deals(
        &self,
        filter: Option<DealFilter>,
        pagination: PaginationParams,
    ) -> PaginatedResponse<Deal> {
        let mut deals: Vec<Deal> = self.deals.values().cloned().collect();

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
    pub contacts: Vec<(ContactId, Contact)>,
    pub next_contact_id: ContactId,
    pub deals: Vec<(DealId, Deal)>,
    pub next_deal_id: DealId,
    pub transactions: Vec<(TransactionId, Transaction)>,
    pub next_transaction_id: TransactionId,
    pub feature_flags: Vec<(String, FeatureFlag)>,
    #[serde(default)]
    pub metrics_history: Vec<MetricsSnapshot>,
}

impl From<&State> for StableState {
    fn from(state: &State) -> Self {
        StableState {
            controllers: state.controllers.clone(),
            admins: state.admins.clone(),
            contacts: state.contacts.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_contact_id: state.next_contact_id,
            deals: state.deals.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_deal_id: state.next_deal_id,
            transactions: state.transactions.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_transaction_id: state.next_transaction_id,
            feature_flags: state.feature_flags.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            metrics_history: state.metrics_history.clone(),
        }
    }
}

impl From<StableState> for State {
    fn from(stable: StableState) -> Self {
        let mut state = State {
            controllers: stable.controllers,
            admins: stable.admins,
            contacts: stable.contacts.iter().cloned().collect(),
            next_contact_id: stable.next_contact_id,
            deals: stable.deals.iter().cloned().collect(),
            next_deal_id: stable.next_deal_id,
            transactions: stable.transactions.iter().cloned().collect(),
            next_transaction_id: stable.next_transaction_id,
            feature_flags: stable.feature_flags.iter().cloned().collect(),
            metrics_history: stable.metrics_history,
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
