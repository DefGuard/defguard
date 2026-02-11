use sqlx::{PgExecutor, query_as};

pub(crate) struct MailContext {
    /// Template name.
    #[allow(unused)]
    template: String,
    /// Section name in the template.
    pub(crate) section: String,
    /// Language tag, for example "en_US".
    #[allow(unused)]
    language_tag: String,
    /// Text to be replaced.
    pub(crate) text: String,
}

impl MailContext {
    // pub async fn save<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
    // where
    //     E: PgExecutor<'e>,
    // {
    //     query_scalar!(
    //         "INSERT INTO mail_context (template, section, language_tag, text) \
    //         VALUES ($1, $2, $3, $4) \
    //         ON CONFLICT ON CONSTRAINT template_section_language DO \
    //         UPDATE SET text = $4",
    //         self.template,
    //         self.section,
    //         self.language_tag,
    //         self.text,
    //     )
    //     .execute(executor)
    //     .await?;
    //     Ok(())
    // }

    /// Fetch all context for a given template.
    pub(crate) async fn all_for_template<'e, E>(
        executor: E,
        template: &str,
        language_tag: &str,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT template, section, language_tag, text FROM mail_context \
            WHERE template = $1 AND language_tag = $2",
            template,
            language_tag
        )
        .fetch_all(executor)
        .await
    }
}
