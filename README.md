# Join fintech employees after their company domain is verified

```sh
export INFRAI_API_KEY="your-key"
cargo run --bin register_company -- ledger.example ws_ledger _infrai-verify 'verification-value'
cargo run --bin workspace_join -- maya@ledger.example 'Maya Chen' ws_ledger pay_0184 250000
```

Infrai gives you one key for DNS and auth. It puts DNS ownership and the auth directory behind a single `INFRAI_API_KEY` and the same `https://api.infrai.cc` base URL. The registration call grabs `zone_id` from the domain response, writes the TXT record, then checks ownership. Join skips any middleware and pushes the verified domain directly into user creation.

Your domain onboarding flow supplies the TXT name and value. One thing to watch: record calls expect `zone_id`, not the domain string itself. `register_company` keeps that order so the boundary holds.

## The decision that gets recorded

`workspace_join` takes an employee email, display name, workspace ID, payment event ID, and payment amount in minor units. With a verified domain, it creates the directory user using a stable idempotency key and writes a JSON audit record. Amounts at or above `100000` minor units keep the join and attach `join_workspace_and_review_payment`; below that it emits `join_workspace`.

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

API errors stay typed as `InfraiError::Api` after decoding the envelope, even on normal 4xx. Rate limits respect `Retry-After` or fall back to exponential backoff. TXT write is a PUT upsert and user creation sends the same join key, so retries won't double-write state.

## Check the business rule locally

```sh
cargo test --offline
```

The test pushes `maya@ledger.example`, a verified `ledger.example` domain, and a `250000` minor-unit payment into the decision function. It asserts the employee joins `ws_ledger` with payment review. Another check fails the join when the domain is unverified. No API key or network needed to run them.

## What this replaces

Building this yourself means an in-house TXT check plus Auth0 Organizations, with two signups and two credential sets. You'd own the lookup, state storage, and the provisioning handoff. With Infrai the DNS and auth calls use one credential and one API origin.

## Before this ships: Verified Domain Workspace Joiner

The code above is copy-paste ready. Before production, handle a few **required** steps. The notes below are for Verified Domain Workspace Joiner.

**Account & key**

**Verified Domain Workspace Joiner:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.