use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct LetsStartForm {
    #[validate(
        email(
            message = "Incorrect @mail address"
        )
    )]
    pub email: String,

    #[validate(
        range(
            min = 1_000,
            exclusive_max = 50_000,
            message = "The value of the field goes out of range"
        )
    )]
    pub min_budget: u16,

    #[validate(
        range(
            min = 1_000,
            max = 50_000,
            message = "The value of the field goes out of range"
        )
    )]
    pub max_budget: u16,

    #[validate(
        length(
            min = 2,
            max = 32,
            message = "The length of the field goes out of bounds"
        )
    )]
    pub name: String,

    #[validate(
        length(
            min = 64,
            max = 512,
            message = "The length of the field goes out of bounds"
        )
    )]
    pub project_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_form() {
        let form = LetsStartForm {
            email: "test@example.com".to_string(),
            min_budget: 2_000,
            max_budget: 3_000,
            name: "Valid Name".to_string(),
            project_description:
                "This is a valid project description with enough length to meet the minimum requirement."
                    .to_string(),
        };
        assert!(form.validate().is_ok());
    }

    #[test]
    fn test_invalid_email() {
        let form = LetsStartForm {
            email: "invalid-email".to_string(),
            min_budget: 2_000,
            max_budget: 3_000,
            name: "Valid Name".to_string(),
            project_description:
                "This is a valid project description with enough length to meet the minimum requirement."
                    .to_string(),
        };
        assert!(form.validate().is_err());
    }

    #[test]
    fn test_min_budget_out_of_range() {
        let form = LetsStartForm {
            email: "test@example.com".to_string(),
            min_budget: 500,
            max_budget: 3_000,
            name: "Valid Name".to_string(),
            project_description:
                "This is a valid project description with enough length to meet the minimum requirement."
                    .to_string(),
        };
        assert!(form.validate().is_err());
    }

    #[test]
    fn test_max_budget_out_of_range() {
        let form = LetsStartForm {
            email: "test@example.com".to_string(),
            min_budget: 2_000,
            max_budget: 60_000,
            name: "Valid Name".to_string(),
            project_description:
                "This is a valid project description with enough length to meet the minimum requirement."
                    .to_string(),
        };
        assert!(form.validate().is_err());
    }

    #[test]
    fn test_name_length_out_of_bounds() {
        let form = LetsStartForm {
            email: "test@example.com".to_string(),
            min_budget: 2_000,
            max_budget: 3_000,
            name: "A".to_string(),
            project_description:
                "This is a valid project description with enough length to meet the minimum requirement."
                    .to_string(),
        };
        assert!(form.validate().is_err());
    }

    #[test]
    fn test_project_description_length_out_of_bounds() {
        let form = LetsStartForm {
            email: "test@example.com".to_string(),
            min_budget: 2_000,
            max_budget: 3_000,
            name: "Valid Name".to_string(),
            project_description: "This is a valid project description with enough length."
                .to_string(),
        };
        assert!(form.validate().is_err());
    }
}
