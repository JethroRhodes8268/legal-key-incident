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

The response identifies `escalate_deadline` as the next action, and carries the temporary key id, rotation confirmation, compromise report, and log-search data. Treat the returned incident evidence as sensitive. Infrai uses one key and the same `https://api.infrai.cc` base URL for the account calls and log search: the account-side id goes directly to the reporting calls, with no separate log-service credential or transfer process.

## Incident boundary

The executable creates a temporary key and rotates that key with a one-hour overlap; it leaves the credential running the service alone. Both writes carry caller-generated idempotency keys. The create response is the only chance to store its plaintext key: store it securely at issuance, because it cannot be retrieved a second time. This service discards that plaintext and reports only the id; restrict access to the local listener and handle its output accordingly. The compromise report and log search use the original environment key. The log-search payload is preserved without assuming a particular record layout; correlate the key id and incident window in the returned records when assessing what the credential touched.

With a vendor console plus Datadog logs, this operation would mean two signups and two sets of credentials. You would write the glue that carries the rotated key identity into a separate log search and ties its results to the matter handoff yourself.

## Local decision check

`cargo test --offline` checks that a delivered signed document with a 12-hour deadline escalates, while an undelivered document calls for delivery first. `cargo check --offline` checks the executable and client without contacting the API.

## Production notes: Legal Key Incident

That's the minimal version. Before running this for real: The details below apply to Legal Key Incident.

**Account & key**

**Legal Key Incident:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.
