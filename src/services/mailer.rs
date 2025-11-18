use std::{str::FromStr, time::Duration};

use anyhow::Context;
use askama::Template;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};
use tokio_retry::{
    Retry,
    strategy::{ExponentialBackoff, jitter},
};

use crate::{
    api::{errors::EmailErrors, models::LetsStartForm},
    configs::AppConfigs,
};

#[derive(Clone, Debug)]
pub struct Mailer {
    from: Mailbox,
    to: Mailbox,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl Mailer {
    pub fn new(configs: &AppConfigs) -> anyhow::Result<Self> {
        let timeout = Some(Duration::from_millis(configs.smtp_connection_timeout));
        let url = format!(
            "smtps://{}@{}",
            configs.smtp_auth.as_str(),
            configs.smtp_addr.as_str()
        );

        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url.as_str())
            .inspect_err(|err| tracing::error!("smtp error: {:?}", err))
            .context("couldn't create SMTP transport from URL")?
            .timeout(timeout)
            .build();

        let from = Mailbox::from_str(configs.from_mailbox.as_str())
            .inspect_err(|err| tracing::error!("mailbox error: {:?}", err))
            .context("invalid or incompatible <from>")?;
        let to = Mailbox::from_str(configs.to_mailbox.as_str())
            .inspect_err(|err| tracing::error!("mailbox error: {:?}", err))
            .context("invalid or incompatible <to>")?;

        Ok(Self { from, to, transport })
    }

    pub async fn send_message(
        &self,
        form: LetsStartForm,
        configs: &AppConfigs,
    ) -> Result<(), EmailErrors> {
        let letter_text = self.build_letter_text(&form)?;

        let message = Message::builder()
            .from(self.from.clone())
            .to(self.to.clone())
            .subject("Let's start".to_string())
            .header(ContentType::TEXT_PLAIN)
            .body(letter_text)?;

        let retry_strategy = ExponentialBackoff::from_millis(configs.retry_timeout)
            .take(configs.retry_count)
            .map(jitter);

        Retry::spawn(retry_strategy, || async {
            match self.transport.send(message.clone()).await {
                Ok(_) => Ok(()),
                Err(cause) => {
                    tracing::error!("Smtp transport error: {:?}", cause);
                    Err(cause)
                }
            }
        })
        .await?;

        Ok(())
    }

    fn build_letter_text(&self, form: &LetsStartForm) -> Result<String, EmailErrors> {
        let template = LetsStartEmailTemplate::from(form);
        Ok(template.render()?)
    }
}

#[derive(Template)]
#[template(
    source = r#"
Hey,

I hope this message finds you well. My name is {{ name }}.
I would like to discuss a potential collaboration with you on an upcoming project.

A brief overview of the project:
• {{ project_description }}
• Our budget ranges from {{ min_budget }} to {{ max_budget }} U.S. dollars

If you are interested in discussing this opportunity further, please, reach out to me
at {{ email }} email address.

Looking forward to your response.

Regards.
"#,
    ext = "txt",
    escape = "none"
)]
struct LetsStartEmailTemplate<'a> {
    name: &'a str,
    project_description: &'a str,
    min_budget: u16,
    max_budget: u16,
    email: &'a str,
}

impl<'a> From<&'a LetsStartForm> for LetsStartEmailTemplate<'a> {
    fn from(form: &'a LetsStartForm) -> Self {
        Self {
            name: &form.name,
            project_description: &form.project_description,
            min_budget: form.min_budget,
            max_budget: form.max_budget,
            email: &form.email,
        }
    }
}
