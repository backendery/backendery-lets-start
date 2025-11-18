use serde::Deserialize;
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[validate(schema(function = "validate_budget_bounds"))]
#[must_use]
pub struct LetsStartForm {
    #[validate(email(message = "The @mail must be a valid email address"))]
    pub email: String,

    #[validate(range(
        min = 1_000,
        exclusive_max = 50_000,
        message = "The budget must range from 1,000 to 50,000 USD"
    ))]
    pub min_budget: u16,
    #[validate(range(
        min = 1_000,
        max = 50_000,
        message = "The budget must range from 1,000 to 50,000 USD"
    ))]
    pub max_budget: u16,

    #[validate(length(min = 2, max = 32, message = "The name must be between 2 and 32 chars"))]
    pub name: String,

    #[validate(length(
        min = 64,
        max = 512,
        message = "The project description must be between 64 and 512 chars"
    ))]
    pub project_description: String,
}

fn validate_budget_bounds(form: &LetsStartForm) -> Result<(), ValidationError> {
    if form.max_budget < form.min_budget {
        let mut err = ValidationError::new("budget_bounds");
        err.message = Some("max_budget must be >= min_budget".into());
        return Err(err);
    }

    Ok(())
}
