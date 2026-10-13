use crate::infrai_client::{CreatedUser, InfraiClient, InfraiError};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct JoinRequest {
    pub email: String,
    pub name: String,
    pub workspace_id: String,
    pub payment_event_id: String,
    pub payment_amount_minor: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskAction {
    JoinWorkspace,
    JoinWorkspaceAndReviewPayment,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct JoinDecision {
    pub workspace_id: String,
    pub user_id: String,
    pub email: String,
    pub payment_event_id: String,
    pub action: RiskAction,
    pub notification: String,
}

#[derive(Debug, Error)]
pub enum JoinError {
    #[error("email has no company domain")]
    InvalidEmail,
    #[error("company domain is not verified: {0}")]
    DomainNotVerified(String),
    #[error(transparent)]
    Infrai(#[from] InfraiError),
}

pub fn decide_join(
    request: &JoinRequest,
    domain: &str,
    verified: bool,
    user: CreatedUser,
) -> Result<JoinDecision, JoinError> {
    if !verified {
        return Err(JoinError::DomainNotVerified(domain.to_owned()));
    }
    let action = if request.payment_amount_minor >= 100_000 {
        RiskAction::JoinWorkspaceAndReviewPayment
    } else {
        RiskAction::JoinWorkspace
    };
    Ok(JoinDecision {
        workspace_id: request.workspace_id.clone(),
        user_id: user.id,
        email: user.email,
        payment_event_id: request.payment_event_id.clone(),
        action,
        notification: format!(
            "employee {} joined workspace {} after {} was verified",
            request.email, request.workspace_id, domain
        ),
    })
}

pub async fn join_verified_employee(
    client: &InfraiClient,
    request: JoinRequest,
) -> Result<JoinDecision, JoinError> {
    let domain = request
        .email
        .rsplit_once('@')
        .map(|(_, domain)| domain)
        .filter(|domain| !domain.is_empty())
        .ok_or(JoinError::InvalidEmail)?;
    let verification = client.verify_domain(domain).await?;
    if !verification.verified {
        return Err(JoinError::DomainNotVerified(domain.to_owned()));
    }
    let idempotency_key = format!("workspace-join:{}:{}", request.workspace_id, request.email);
    let user = client
        .create_user(
            &request.email,
            &request.name,
            &request.workspace_id,
            &idempotency_key,
        )
        .await?;
    decide_join(&request, domain, true, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payment_request(amount: u64) -> JoinRequest {
        JoinRequest {
            email: "maya@ledger.example".to_owned(),
            name: "Maya Chen".to_owned(),
            workspace_id: "ws_ledger".to_owned(),
            payment_event_id: "pay_0184".to_owned(),
            payment_amount_minor: amount,
        }
    }

    #[test]
    fn verified_employee_with_large_payment_joins_with_review() {
        let decision = decide_join(
            &payment_request(250_000),
            "ledger.example",
            true,
            CreatedUser {
                id: "usr_42".to_owned(),
                email: "maya@ledger.example".to_owned(),
            },
        )
        .expect("verified employee should join");
        assert_eq!(decision.action, RiskAction::JoinWorkspaceAndReviewPayment);
        assert_eq!(decision.payment_event_id, "pay_0184");
    }

    #[test]
    fn unverified_domain_never_produces_a_join_decision() {
        let result = decide_join(
            &payment_request(500),
            "ledger.example",
            false,
            CreatedUser {
                id: "usr_unused".to_owned(),
                email: "maya@ledger.example".to_owned(),
            },
        );
        assert!(matches!(result, Err(JoinError::DomainNotVerified(_))));
    }
}
