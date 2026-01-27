# DAO Admin Canister

Administrative canister for the Hello World Co-Op DAO platform, providing CRM, Finance, Analytics, and Feature Flag management.

## Overview

This canister handles internal administrative operations:

- **CRM** - Contact and deal pipeline management
- **Finance** - Transaction tracking and financial reporting
- **Analytics** - User activity logging and metrics
- **Feature Flags** - Feature rollout controls

## Features

### CRM

#### Contacts
- Create and manage contacts from user signups
- Track source, company, job title, and interest areas
- Search and filter contacts
- Link contacts to user accounts

#### Deal Pipeline
- 6-stage pipeline: Lead → Qualified → Proposal → Negotiation → ClosedWon/ClosedLost
- Auto-create deals from signup
- Track deal value and expected close date

### Finance

- Record income and expense transactions
- Categories: Subscription, Donation, Service, Infrastructure, Marketing, Payroll, Legal, Other
- Financial summary with MRR calculation
- Period-based reporting

### Analytics

- User activity logging
- Platform metrics snapshots
- Historical trend data

### Feature Flags

- Enable/disable features globally
- Percentage-based rollouts
- Principal whitelist for beta access

## Building

```bash
cargo build --release --target wasm32-unknown-unknown
```

## Testing

```bash
cargo test
```

## API Reference

### Admin Management

| Method | Type | Description |
|--------|------|-------------|
| `add_admin` | Update | Add an admin principal |
| `remove_admin` | Update | Remove an admin principal |
| `get_admins` | Query | List admin principals |

### Contact API

| Method | Type | Description |
|--------|------|-------------|
| `create_contact` | Update | Create a new contact |
| `create_contact_from_signup` | Update | Create contact from user signup |
| `get_contact` | Query | Get contact by ID |
| `get_contact_by_email` | Query | Get contact by email |
| `get_contacts` | Query | List contacts with filters |

### Deal API

| Method | Type | Description |
|--------|------|-------------|
| `create_deal` | Update | Create a new deal |
| `get_deal` | Query | Get deal by ID |
| `update_deal_stage` | Update | Move deal to new stage |
| `get_deals` | Query | List deals with filters |

### Transaction API

| Method | Type | Description |
|--------|------|-------------|
| `create_transaction` | Update | Record a transaction |
| `get_transactions` | Query | List transactions with filters |
| `get_financial_summary` | Query | Get financial summary for period |

### Feature Flag API

| Method | Type | Description |
|--------|------|-------------|
| `set_feature_flag` | Update | Set a feature flag |
| `get_feature_flag` | Query | Get feature flag by key |
| `is_feature_enabled` | Query | Check if feature is enabled for caller |
| `list_feature_flags` | Query | List all feature flags |

### Analytics API

| Method | Type | Description |
|--------|------|-------------|
| `log_activity` | Update | Log user activity |
| `record_metrics` | Update | Record metrics snapshot |

### Health & Stats

| Method | Type | Description |
|--------|------|-------------|
| `health` | Query | Health check (returns "ok") |
| `get_admin_stats` | Query | Get admin dashboard stats |

## Access Control

- **Controllers** - Full access to all operations
- **Admins** - Access to CRM, Finance, Analytics, Feature Flags
- **Public** - `is_feature_enabled` only

## Integration

### User Signup Flow

When a user signs up via `user-service`:
1. User-service creates the user account
2. User-service calls `create_contact_from_signup` on dao-admin
3. A Contact is created with source=Signup
4. A Deal is auto-created in Lead stage

### Feature Flag Usage

```typescript
// Check if feature is enabled for current user
const enabled = await dao_admin.is_feature_enabled("new_chat_ui");
if (enabled) {
  // Show new UI
}
```

## License

Part of the Hello World Co-Op DAO platform.
