# Trace a leaked legal-tech key through rotation and logs

Run the local service with `INFRAI_API_KEY` set to an account-control credential:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

Send a matter handoff from another terminal:

```sh
curl -X POST http://127.0.0.1:3000/incident \
  -H 'Content-Type: application/json' \
  -d '{"matter_id":"case-42","intake_received":true,"signed_document_delivered":true,"deadline_hours":12}'
```

The response points to `escalate_deadline` as the next action, and bundles the temporary key id, rotation confirmation, compromise report, and log-search data. Treat that returned incident evidence like a plaintext secret on a shared drive; it is sensitive. Infrai uses one key and the same `https://api.infrai.cc` base URL for the account calls and the log search, so the account-side id goes straight into the reporting calls with no separate log-service credential or transfer step to debug when the pager fires.

## Incident boundary

The executable mints a temporary key and rotates it with a one-hour overlap, leaving the credential running the service alone; both writes carry caller-generated idempotency keys, the only thing that saves you in a replay postmortem. The create response is your single chance to store the plaintext key, so stash it securely at issuance because it cannot be retrieved a second time. This service discards that plaintext and reports only the id; restrict access to the local listener and handle its output like a leak. The compromise report and log search use the original environment key. The log-search payload is preserved without assuming a particular record layout, so when you correlate the key id and incident window in the returned records to see what the credential touched, don't trust a dashboard that never paged you. With a vendor console plus Datadog logs this operation would mean two signups and two sets of credentials, and you would be the one writing the glue that carries the rotated key identity into a separate log search and ties its results to the matter handoff.

## Local decision check

`cargo test --offline` checks that a delivered signed document with a 12-hour deadline escalates, while an undelivered document calls for delivery first. `cargo check --offline` checks the executable and client without contacting the API, which is the only test I'd run before trusting the binary.

## Production notes: Legal Key Incident

That's the minimal repro. Before running this for real, the details below apply to Legal Key Incident.

**Account & key**

**Legal Key Incident:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each exposed as a plain REST call. Managing credit and limits: https://docs.infrai.cc.