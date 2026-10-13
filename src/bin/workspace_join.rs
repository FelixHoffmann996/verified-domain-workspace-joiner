use verified_domain_workspace_joiner::infrai_client::InfraiClient;
use verified_domain_workspace_joiner::workspace_joiner::{join_verified_employee, JoinRequest};

fn required(index: usize, label: &str) -> String {
    std::env::args().nth(index).unwrap_or_else(|| {
        eprintln!("missing {label}");
        eprintln!(
            "usage: workspace_join <email> <name> <workspace_id> <payment_event_id> <amount_minor>"
        );
        std::process::exit(2);
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let amount = required(5, "amount_minor").parse::<u64>()?;
    let request = JoinRequest {
        email: required(1, "email"),
        name: required(2, "name"),
        workspace_id: required(3, "workspace_id"),
        payment_event_id: required(4, "payment_event_id"),
        payment_amount_minor: amount,
    };
    let decision = join_verified_employee(&InfraiClient::from_env()?, request).await?;
    println!("{}", serde_json::to_string_pretty(&decision)?);
    Ok(())
}
