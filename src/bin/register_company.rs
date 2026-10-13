use verified_domain_workspace_joiner::infrai_client::InfraiClient;

fn required(index: usize, label: &str) -> String {
    std::env::args().nth(index).unwrap_or_else(|| {
        eprintln!("missing {label}");
        eprintln!("usage: register_company <domain> <workspace_id> <txt_name> <txt_content>");
        std::process::exit(2);
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let domain = required(1, "domain");
    let workspace_id = required(2, "workspace_id");
    let txt_name = required(3, "txt_name");
    let txt_content = required(4, "txt_content");
    let client = InfraiClient::from_env()?;

    let zone = client.add_domain(&domain, &workspace_id).await?;
    client
        .upsert_ownership_txt(&zone.zone_id, &txt_name, &txt_content)
        .await?;
    let verification = client.verify_domain(&domain).await?;
    println!(
        "{}",
        serde_json::json!({
            "domain": domain,
            "zone_id": zone.zone_id,
            "verified": verification.verified
        })
    );
    Ok(())
}
