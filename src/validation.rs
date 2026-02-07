//! Input Validation Module
//!
//! FOS-5.6.11: Admin Input Validation
//! Provides backend validation for all admin form inputs.
//!
//! Security Principle: Never Trust Client Input
//! All validation must be duplicated in canister (defense in depth).
//!
//! @see AC-5.6.11.1 - Zod schema validation with proper error messages
//! @see AC-5.6.11.2 - Contact forms validate email format, phone format, required fields
//! @see AC-5.6.11.3 - Deal forms validate amount >= 0 and required fields
//! @see AC-5.6.11.4 - Transaction forms validate amount with min/max limits

use regex::Regex;
use std::sync::LazyLock;

// =============================================================================
// Constants (matching frontend validation in transactionSchema.ts)
// =============================================================================

/// Maximum transaction amount in cents ($1,000,000.00 = 100,000,000 cents)
/// @see AC-5.6.11.4 - Configurable maximum amount limit
pub const MAX_TRANSACTION_AMOUNT: u64 = 100_000_000;

/// Maximum deal value in cents ($10,000,000.00 = 1,000,000,000 cents)
pub const MAX_DEAL_VALUE: u64 = 1_000_000_000;

// Field length limits (matching frontend schemas)
/// Contact name: min 2, max 100 characters
pub const CONTACT_NAME_MIN_LEN: usize = 2;
pub const CONTACT_NAME_MAX_LEN: usize = 100;

/// Contact company: max 200 characters
pub const CONTACT_COMPANY_MAX_LEN: usize = 200;

/// Contact notes: max 5000 characters
pub const CONTACT_NOTES_MAX_LEN: usize = 5000;

/// Deal name: min 3, max 200 characters
pub const DEAL_NAME_MIN_LEN: usize = 3;
pub const DEAL_NAME_MAX_LEN: usize = 200;

/// Deal notes: max 5000 characters
pub const DEAL_NOTES_MAX_LEN: usize = 5000;

/// Transaction description: max 1000 characters
pub const TRANSACTION_DESC_MAX_LEN: usize = 1000;

/// Transaction reference: max 200 characters
pub const TRANSACTION_REF_MAX_LEN: usize = 200;

// =============================================================================
// Email Validation
// =============================================================================

/// Email regex pattern (simplified but effective)
/// Matches: local@domain.tld
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

/// Validate email format
/// @see AC-5.6.11.2 - Contact forms validate email format
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("Email is required".to_string());
    }
    if email.len() > 254 {
        return Err("Email must be at most 254 characters".to_string());
    }
    if !EMAIL_REGEX.is_match(email) {
        return Err("Invalid email format".to_string());
    }
    Ok(())
}

// =============================================================================
// String Length Validation
// =============================================================================

/// Validate string length with min and max bounds
pub fn validate_string_length(
    value: &str,
    field_name: &str,
    min_len: Option<usize>,
    max_len: usize,
) -> Result<(), String> {
    let len = value.chars().count();

    if let Some(min) = min_len {
        if len < min {
            return Err(format!(
                "{} must be at least {} characters",
                field_name, min
            ));
        }
    }

    if len > max_len {
        return Err(format!(
            "{} must be at most {} characters",
            field_name, max_len
        ));
    }

    Ok(())
}

/// Validate optional string length
pub fn validate_optional_string_length(
    value: &Option<String>,
    field_name: &str,
    min_len: Option<usize>,
    max_len: usize,
) -> Result<(), String> {
    if let Some(v) = value {
        validate_string_length(v, field_name, min_len, max_len)
    } else {
        Ok(())
    }
}

// =============================================================================
// Contact Validation
// =============================================================================

use crate::types::CreateContactRequest;

/// Validate a CreateContactRequest
/// @see AC-5.6.11.1, AC-5.6.11.2
pub fn validate_create_contact(request: &CreateContactRequest) -> Result<(), String> {
    // Email is required and must be valid format
    validate_email(&request.email)?;

    // Name: optional, but if provided must be 2-100 chars
    validate_optional_string_length(
        &request.name,
        "Name",
        Some(CONTACT_NAME_MIN_LEN),
        CONTACT_NAME_MAX_LEN,
    )?;

    // Company: optional, max 200 chars
    validate_optional_string_length(
        &request.company,
        "Company",
        None,
        CONTACT_COMPANY_MAX_LEN,
    )?;

    // Job title: optional, max 100 chars (reasonable limit)
    validate_optional_string_length(
        &request.job_title,
        "Job title",
        None,
        100,
    )?;

    // Interest area: optional, max 200 chars
    validate_optional_string_length(
        &request.interest_area,
        "Interest area",
        None,
        200,
    )?;

    // Notes: optional, max 5000 chars
    validate_optional_string_length(
        &request.notes,
        "Notes",
        None,
        CONTACT_NOTES_MAX_LEN,
    )?;

    Ok(())
}

// =============================================================================
// Contact Update Validation
// =============================================================================

use crate::types::UpdateContactRequest;

/// Validate an UpdateContactRequest
/// @see AC-5.6.11.1, AC-5.6.11.2
pub fn validate_update_contact(request: &UpdateContactRequest) -> Result<(), String> {
    // Name: if provided, must be 2-100 chars
    validate_optional_string_length(
        &request.name,
        "Name",
        Some(CONTACT_NAME_MIN_LEN),
        CONTACT_NAME_MAX_LEN,
    )?;

    // Company: if provided, max 200 chars
    validate_optional_string_length(
        &request.company,
        "Company",
        None,
        CONTACT_COMPANY_MAX_LEN,
    )?;

    // Job title: if provided, max 100 chars
    validate_optional_string_length(
        &request.job_title,
        "Job title",
        None,
        100,
    )?;

    // Interest area: if provided, max 200 chars
    validate_optional_string_length(
        &request.interest_area,
        "Interest area",
        None,
        200,
    )?;

    // Notes: if provided, max 5000 chars
    validate_optional_string_length(
        &request.notes,
        "Notes",
        None,
        CONTACT_NOTES_MAX_LEN,
    )?;

    Ok(())
}

// =============================================================================
// Deal Validation
// =============================================================================

use crate::types::{CreateDealRequest, UpdateDealRequest};

/// Validate a CreateDealRequest
/// @see AC-5.6.11.1, AC-5.6.11.3
pub fn validate_create_deal(request: &CreateDealRequest) -> Result<(), String> {
    // Name is required, 3-200 chars
    validate_string_length(
        &request.name,
        "Deal name",
        Some(DEAL_NAME_MIN_LEN),
        DEAL_NAME_MAX_LEN,
    )?;

    // Value: optional, but must not exceed max if provided
    // Note: u64 cannot be negative, so we only check max
    if let Some(value) = request.value {
        if value > MAX_DEAL_VALUE {
            return Err(format!(
                "Deal value cannot exceed ${}",
                MAX_DEAL_VALUE / 100
            ));
        }
    }

    // Notes: optional, max 5000 chars
    validate_optional_string_length(
        &request.notes,
        "Notes",
        None,
        DEAL_NOTES_MAX_LEN,
    )?;

    Ok(())
}

/// Validate an UpdateDealRequest
/// @see AC-5.6.11.1, AC-5.6.11.3
pub fn validate_update_deal(request: &UpdateDealRequest) -> Result<(), String> {
    // Name: if provided, must be 3-200 chars
    validate_optional_string_length(
        &request.name,
        "Deal name",
        Some(DEAL_NAME_MIN_LEN),
        DEAL_NAME_MAX_LEN,
    )?;

    // Value: if provided, must not exceed max
    if let Some(value) = request.value {
        if value > MAX_DEAL_VALUE {
            return Err(format!(
                "Deal value cannot exceed ${}",
                MAX_DEAL_VALUE / 100
            ));
        }
    }

    // Notes: if provided, max 5000 chars
    validate_optional_string_length(
        &request.notes,
        "Notes",
        None,
        DEAL_NOTES_MAX_LEN,
    )?;

    Ok(())
}

// =============================================================================
// Transaction Validation
// =============================================================================

use crate::types::CreateTransactionRequest;

/// Validate a CreateTransactionRequest
/// @see AC-5.6.11.1, AC-5.6.11.4
pub fn validate_create_transaction(request: &CreateTransactionRequest) -> Result<(), String> {
    // Amount validation
    // Note: u64 cannot be negative, so we only check max
    if request.amount > MAX_TRANSACTION_AMOUNT {
        return Err(format!(
            "Transaction amount cannot exceed ${}",
            MAX_TRANSACTION_AMOUNT / 100
        ));
    }

    // Description is required and has max length
    if request.description.is_empty() {
        return Err("Description is required".to_string());
    }
    validate_string_length(
        &request.description,
        "Description",
        None,
        TRANSACTION_DESC_MAX_LEN,
    )?;

    // Reference: optional, max 200 chars
    validate_optional_string_length(
        &request.reference,
        "Reference",
        None,
        TRANSACTION_REF_MAX_LEN,
    )?;

    // Currency: if provided, should be valid ISO 4217 code (3 uppercase letters)
    if let Some(ref currency) = request.currency {
        if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err("Currency must be a valid 3-letter ISO 4217 code (e.g., USD, EUR)".to_string());
        }
    }

    Ok(())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TransactionType, TransactionCategory};

    // -------------------------------------------------------------------------
    // Email Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_emails() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name@domain.org").is_ok());
        assert!(validate_email("user+tag@example.co.uk").is_ok());
        assert!(validate_email("a@b.co").is_ok());
    }

    #[test]
    fn test_invalid_emails() {
        assert!(validate_email("").is_err());
        assert!(validate_email("notanemail").is_err());
        assert!(validate_email("missing@domain").is_err());
        assert!(validate_email("@nodomain.com").is_err());
        assert!(validate_email("spaces in@email.com").is_err());
    }

    #[test]
    fn test_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        assert!(validate_email(&long_email).is_err());
    }

    // -------------------------------------------------------------------------
    // String Length Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_string_length_valid() {
        assert!(validate_string_length("Hello", "Field", Some(2), 10).is_ok());
        assert!(validate_string_length("AB", "Field", Some(2), 10).is_ok());
        assert!(validate_string_length("ABCDEFGHIJ", "Field", Some(2), 10).is_ok());
    }

    #[test]
    fn test_string_length_too_short() {
        let result = validate_string_length("A", "Name", Some(2), 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 2"));
    }

    #[test]
    fn test_string_length_too_long() {
        let result = validate_string_length("ABCDEFGHIJK", "Name", Some(2), 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at most 10"));
    }

    // -------------------------------------------------------------------------
    // Contact Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_contact_request() {
        let request = CreateContactRequest {
            user_id: None,
            email: "test@example.com".to_string(),
            name: Some("John Doe".to_string()),
            company: Some("Acme Corp".to_string()),
            job_title: Some("Engineer".to_string()),
            interest_area: Some("Technology".to_string()),
            source: None,
            notes: None,
        };
        assert!(validate_create_contact(&request).is_ok());
    }

    #[test]
    fn test_contact_invalid_email() {
        let request = CreateContactRequest {
            user_id: None,
            email: "invalid-email".to_string(),
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            source: None,
            notes: None,
        };
        let result = validate_create_contact(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("email"));
    }

    #[test]
    fn test_contact_name_too_short() {
        let request = CreateContactRequest {
            user_id: None,
            email: "test@example.com".to_string(),
            name: Some("A".to_string()), // Too short
            company: None,
            job_title: None,
            interest_area: None,
            source: None,
            notes: None,
        };
        let result = validate_create_contact(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Name"));
    }

    #[test]
    fn test_contact_notes_too_long() {
        let request = CreateContactRequest {
            user_id: None,
            email: "test@example.com".to_string(),
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            source: None,
            notes: Some("x".repeat(5001)), // Too long
        };
        let result = validate_create_contact(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Notes"));
    }

    // -------------------------------------------------------------------------
    // Update Contact Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_update_contact_request() {
        let request = UpdateContactRequest {
            id: 1,
            name: Some("John Doe".to_string()),
            company: Some("Acme Corp".to_string()),
            job_title: Some("Engineer".to_string()),
            interest_area: Some("Technology".to_string()),
            notes: Some("Updated notes".to_string()),
            status: None,
        };
        assert!(validate_update_contact(&request).is_ok());
    }

    #[test]
    fn test_update_contact_name_too_short() {
        let request = UpdateContactRequest {
            id: 1,
            name: Some("A".to_string()), // Too short
            company: None,
            job_title: None,
            interest_area: None,
            notes: None,
            status: None,
        };
        let result = validate_update_contact(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Name"));
    }

    #[test]
    fn test_update_contact_notes_too_long() {
        let request = UpdateContactRequest {
            id: 1,
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            notes: Some("x".repeat(5001)), // Too long
            status: None,
        };
        let result = validate_update_contact(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Notes"));
    }

    #[test]
    fn test_update_contact_all_none_is_valid() {
        // Empty update request is valid (no changes)
        let request = UpdateContactRequest {
            id: 1,
            name: None,
            company: None,
            job_title: None,
            interest_area: None,
            notes: None,
            status: None,
        };
        assert!(validate_update_contact(&request).is_ok());
    }

    // -------------------------------------------------------------------------
    // Deal Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_deal_request() {
        let request = CreateDealRequest {
            contact_id: 1,
            name: "New Deal".to_string(),
            value: Some(100_000), // $1,000.00
            notes: Some("Important deal".to_string()),
            expected_close_date: None,
        };
        assert!(validate_create_deal(&request).is_ok());
    }

    #[test]
    fn test_deal_name_too_short() {
        let request = CreateDealRequest {
            contact_id: 1,
            name: "AB".to_string(), // Too short (min 3)
            value: None,
            notes: None,
            expected_close_date: None,
        };
        let result = validate_create_deal(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Deal name"));
    }

    #[test]
    fn test_deal_value_exceeds_max() {
        let request = CreateDealRequest {
            contact_id: 1,
            name: "Big Deal".to_string(),
            value: Some(MAX_DEAL_VALUE + 1), // Exceeds max
            notes: None,
            expected_close_date: None,
        };
        let result = validate_create_deal(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot exceed"));
    }

    // -------------------------------------------------------------------------
    // Transaction Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_transaction_request() {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Subscription,
            amount: 10_000, // $100.00
            currency: Some("USD".to_string()),
            description: "Monthly subscription".to_string(),
            reference: Some("INV-001".to_string()),
            date: None,
        };
        assert!(validate_create_transaction(&request).is_ok());
    }

    #[test]
    fn test_transaction_amount_exceeds_max() {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Expense,
            category: TransactionCategory::Other,
            amount: MAX_TRANSACTION_AMOUNT + 1, // Exceeds max
            currency: None,
            description: "Large expense".to_string(),
            reference: None,
            date: None,
        };
        let result = validate_create_transaction(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot exceed"));
    }

    #[test]
    fn test_transaction_empty_description() {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Other,
            amount: 1000,
            currency: None,
            description: "".to_string(), // Empty
            reference: None,
            date: None,
        };
        let result = validate_create_transaction(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Description"));
    }

    #[test]
    fn test_transaction_invalid_currency() {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Other,
            amount: 1000,
            currency: Some("usd".to_string()), // lowercase
            description: "Test".to_string(),
            reference: None,
            date: None,
        };
        let result = validate_create_transaction(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ISO 4217"));

        // Also test wrong length
        let request2 = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Other,
            amount: 1000,
            currency: Some("US".to_string()), // Too short
            description: "Test".to_string(),
            reference: None,
            date: None,
        };
        assert!(validate_create_transaction(&request2).is_err());
    }

    #[test]
    fn test_transaction_description_too_long() {
        let request = CreateTransactionRequest {
            transaction_type: TransactionType::Income,
            category: TransactionCategory::Other,
            amount: 1000,
            currency: None,
            description: "x".repeat(1001), // Too long
            reference: None,
            date: None,
        };
        let result = validate_create_transaction(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Description"));
    }
}
