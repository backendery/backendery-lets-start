use std::{fmt::Write, str::FromStr, time::Duration};

use anyhow::Context;
use lettre::{
    message::{header::ContentType, Mailbox},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tokio_retry::{strategy::ExponentialBackoff, Retry};

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

    pub async fn send_message(&self, form: LetsStartForm, configs: &AppConfigs) -> Result<(), EmailErrors> {
        let letter_text = self.build_letter_text(form);

        let message = Message::builder()
            .from(self.from.clone())
            .to(self.to.clone())
            .subject("Let's start".to_string())
            .header(ContentType::TEXT_PLAIN)
            .body(letter_text)?;

        let retry_strategy =
            ExponentialBackoff::from_millis(configs.retry_timeout).take(configs.retry_count);

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

    fn build_letter_text(&self, form: LetsStartForm) -> String {
        let mut letter_text = String::with_capacity(1_024);
        write!(
            &mut letter_text,
            r#"
Hey,

I hope this message finds you well. My name is {}.
I would like to discuss a potential collaboration with you on an upcoming project.

A brief overview of the project:
• {}
• Our budget ranges from {} to {} U.S. dollars

If you are interested in discussing this opportunity further, please, reach out to me
at {} email address.

Looking forward to your response.

Regards.
            "#,
            form.name, form.project_description, form.min_budget, form.max_budget, form.email
        )
        .expect("writing to a string should not fail");

        letter_text.trim().to_string()
    }
}
