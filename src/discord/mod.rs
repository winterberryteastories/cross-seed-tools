use serenity::builder::ExecuteWebhook;
use serenity::http::Http;
use serenity::model::webhook::Webhook;

pub(crate) async fn discord_webhook(
    webhook_url: &str,
    content: &str,
) -> anyhow::Result<()> {
    let http = Http::new("");
    let webhook = Webhook::from_url(&http, webhook_url).await?;

    let builder = ExecuteWebhook::new().content(content).username("cross-seed-tools");
    webhook.execute(&http, false, builder).await.expect("Could not execute webhook.");

    Ok(())
}
