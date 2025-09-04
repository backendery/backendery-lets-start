use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct LetsStartForm {
    #[validate(email(message = "The `email` is expected"))]
    pub email: String,

    #[validate(range(
        min = 1_000,
        exclusive_max = 50_000,
        message = "The `budget` is expected to be between 1,000 and 50,000 c.u."
    ))]
    pub min_budget: u16,
    #[validate(range(
        min = 1_000,
        max = 50_000,
        message = "The `budget` is expected to be between 1,000 and 50,000 c.u."
    ))]
    pub max_budget: u16,

    #[validate(length(
        min = 2,
        max = 32,
        message = "The `name` is expected to be between 2 and 32 chars long"
    ))]
    pub name: String,

    #[validate(length(
        min = 64,
        max = 512,
        message = "The `project description` is expected to be between 64 and 512 chars long"
    ))]
    pub project_description: String,
}
