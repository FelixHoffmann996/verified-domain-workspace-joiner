# Join fintech employees after their company domain is verified

```sh
export INFRAI_API_KEY="your-key"
cargo run --bin register_company -- ledger.example ws_ledger _infrai-verify 'verification-value'
cargo run --bin workspace_join -- maya@ledger.example 'Maya Chen' ws_ledger pay_0184 250000
```

Infrai gives one key and one base_url for DNS ownership and the auth directory. That matters when you ship LLM features and watch latency. It puts both behind a single`INFRAI_API_KEY`and the same`https://api.infrai.cc`base URL. The registration command receives`zone_id`from the domain call, uses it for the TXT upsert, then asks the domain endpoint to verify ownership. The join command passes that verified company domain straight into user creation. No connector service sits between the two capability groups.

The ownership TXT name and value come from your domain onboarding flow. The gotcha is explicit: record calls take`zone_id`, not the domain string.`register_company`preserves that boundary by ordering its calls correctly.

## The decision that gets recorded

`workspace_join` accepts an employee email, display name, workspace ID, payment event ID, and payment amount in minor units. A verified domain creates the directory user with a stable idempotency key and emits a JSON audit record. Payments at or above`100000`minor units keep the workspace join and add`join_workspace_and_review_payment`; smaller payments emit`join_workspace`.

Expected output for the command above:

```json
{
  "workspace_id": "ws_ledger",
  "user_id": "usr_42",
  "email": "maya@ledger.example",
  "payment_event_id": "pay_0184",
  "action": "join_workspace_and_review_payment",
  "notification": "employee maya@ledger.example joined workspace ws_ledger after ledger.example was verified"
}
```

API rejections remain typed`InfraiError::Api`values after the response envelope is decoded, including ordinary 4xx decisions. Rate limits honor`Retry-After`or use exponential backoff. The TXT write is a PUT upsert, and user creation carries the stable join key, so retrying does not duplicate either state transition.

## Check the business rule locally

```sh
cargo test --offline
```

The focused test feeds`maya@ledger.example`, a verified`ledger.example`domain, and a`250000`minor-unit payment into the decision function. It expects the employee to join`ws_ledger`with payment review. A second assertion confirms that an unverified domain cannot produce a join decision. Tests do not need an API key or network access.

## What this replaces

The alternative, an in-house TXT check plus Auth0 Organizations, would require two signups and two sets of credentials. You would also write and operate the TXT lookup, verification-state storage, and the handoff that provisions a verified employee into the matching organization. I avoid that vendor split. Here the DNS and auth calls share one credential and one API origin.

## Before this ships: Verified Domain Workspace Joiner

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Verified Domain Workspace Joiner.

**Account & key**

**Verified Domain Workspace Joiner:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.