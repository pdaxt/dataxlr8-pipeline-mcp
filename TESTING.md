# Testing Guide — dataxlr8-pipeline-mcp

Manual test cases for every tool. Tests use `jq` piped JSON over stdio.

## Prerequisites

```bash
# Start PostgreSQL (if not running)
pg_isready || pg_ctl start

# Set env
export DATABASE_URL="postgres://localhost/dataxlr8"
export MCP_SERVER_NAME="dataxlr8-pipeline-mcp"
export LOG_LEVEL="info"
```

---

## Tool: `create_pipeline`

### CP-1: Happy path — create pipeline with valid stages
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"sales-q1","stages":["lead","qualified","proposal","closed_won","closed_lost"]}}}
```
**Expected:** Pipeline created, returns id, name, stages array, created_at.

### CP-2: Reject empty name
```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"","stages":["a","b"]}}}
```
**Expected:** Error `"name cannot be empty"`

### CP-3: Reject whitespace-only name
```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"   ","stages":["a","b"]}}}
```
**Expected:** Error `"name cannot be empty"`

### CP-4: Reject single stage
```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"bad","stages":["only_one"]}}}
```
**Expected:** Error `"Pipeline must have at least 2 stages"`

### CP-5: Reject duplicate stage names
```json
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"dupes","stages":["lead","qualified","Lead"]}}}
```
**Expected:** Error `"Duplicate stage name: 'Lead'"` (case-insensitive)

### CP-6: Reject empty stage string
```json
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"empty-stage","stages":["lead","","closed"]}}}
```
**Expected:** Error `"Stage at index 1 cannot be empty"`

### CP-7: Reject missing stages param
```json
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"no-stages"}}}
```
**Expected:** Error about missing stages parameter.

### CP-8: Reject duplicate pipeline name (DB unique constraint)
```json
# Run CP-1 first, then:
{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"sales-q1","stages":["a","b"]}}}
```
**Expected:** Error from DB about unique constraint violation.

### CP-9: Reject name exceeding 200 characters
**Expected:** Error `"name exceeds maximum length of 200 characters"`

### CP-10: Reject more than 50 stages
**Expected:** Error `"Pipeline cannot have more than 50 stages"`

---

## Tool: `list_pipelines`

### LP-1: List when empty
```json
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"list_pipelines","arguments":{}}}
```
**Expected:** `{"pipelines":[],"message":"No pipelines found"}`

### LP-2: List with pipelines and prospect counts
**Setup:** Create a pipeline, add prospects, then list.
**Expected:** Returns pipelines with `stage_counts` map showing prospect count per stage.

---

## Tool: `add_prospect`

### AP-1: Happy path — add prospect to first stage
```json
{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"test@example.com","company":"Acme","source":"website","lead_score":50}}}
```
**Expected:** Prospect created in first stage ("lead"), lead_score=50, stage history entry created.

### AP-2: Reject missing contact_email
```json
{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1"}}}
```
**Expected:** Error `"Missing required parameter: contact_email"`

### AP-3: Reject invalid email format (no @)
```json
{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"notanemail"}}}
```
**Expected:** Error about invalid email format.

### AP-4: Reject invalid email format (no domain)
```json
{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"user@"}}}
```
**Expected:** Error about invalid email format.

### AP-5: Reject non-existent pipeline
```json
{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"nonexistent","contact_email":"a@b.com"}}}
```
**Expected:** Error `"Pipeline 'nonexistent' not found"`

### AP-6: Clamp lead_score above 100
```json
{"jsonrpc":"2.0","id":25,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"high@score.com","lead_score":999}}}
```
**Expected:** Prospect created with lead_score=100 (clamped).

### AP-7: Clamp negative lead_score to 0
```json
{"jsonrpc":"2.0","id":26,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"low@score.com","lead_score":-50}}}
```
**Expected:** Prospect created with lead_score=0 (clamped).

### AP-8: Reject duplicate email in same pipeline (unique index)
**Setup:** Add prospect from AP-1 first.
```json
{"jsonrpc":"2.0","id":27,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"test@example.com"}}}
```
**Expected:** Error from DB about unique constraint violation.

### AP-9: Reject empty pipeline_name
```json
{"jsonrpc":"2.0","id":28,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"","contact_email":"a@b.com"}}}
```
**Expected:** Error `"pipeline_name cannot be empty"`

### AP-10: Reject notes exceeding 5000 characters
**Expected:** Error `"notes exceeds maximum length of 5000 characters"`

---

## Tool: `score_prospect`

### SP-1: Happy path — increase score
```json
{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"test@example.com","score_delta":10,"reason":"opened email"}}}
```
**Expected:** Updated prospect with new score, returns delta and reason.

### SP-2: Decrease score (negative delta)
```json
{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"test@example.com","score_delta":-5,"reason":"bounced email"}}}
```
**Expected:** Score decremented, clamped to floor of 0.

### SP-3: Score clamped at 100 (ceiling)
**Setup:** Prospect with score=95.
```json
{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"test@example.com","score_delta":50}}}
```
**Expected:** Score = 100 (not 145).

### SP-4: Score clamped at 0 (floor)
**Setup:** Prospect with score=5.
```json
{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"test@example.com","score_delta":-100}}}
```
**Expected:** Score = 0 (not -95).

### SP-5: Reject non-existent email
```json
{"jsonrpc":"2.0","id":34,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"nobody@nowhere.com","score_delta":10}}}
```
**Expected:** Error about prospect not found.

### SP-6: Reject invalid email format
```json
{"jsonrpc":"2.0","id":35,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"bad","score_delta":10}}}
```
**Expected:** Error about invalid email format.

### SP-7: Reject missing score_delta
```json
{"jsonrpc":"2.0","id":36,"method":"tools/call","params":{"name":"score_prospect","arguments":{"contact_email":"test@example.com"}}}
```
**Expected:** Error `"Missing required parameter: score_delta"`

---

## Tool: `advance_prospect`

### AV-1: Happy path — move to next stage
```json
{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":"qualified","notes":"Passed initial screening"}}}
```
**Expected:** Prospect moved, returns from_stage + to_stage, stage_history entry created.

### AV-2: Reject invalid target stage
```json
{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":"nonexistent_stage"}}}
```
**Expected:** Error listing valid stages.

### AV-3: Reject same-stage transition
```json
{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":"qualified"}}}
```
**Expected:** Error `"Prospect is already in stage 'qualified'"`

### AV-4: Reject empty to_stage
```json
{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":""}}}
```
**Expected:** Error `"to_stage cannot be empty"`

### AV-5: Reject non-existent prospect
```json
{"jsonrpc":"2.0","id":44,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"ghost@void.com","to_stage":"qualified"}}}
```
**Expected:** Error about prospect not found.

### AV-6: Allow backward transitions (e.g. proposal -> qualified)
```json
{"jsonrpc":"2.0","id":45,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":"lead","notes":"Re-evaluation needed"}}}
```
**Expected:** Succeeds. Stage history records the backward move.

---

## Tool: `pipeline_metrics`

### PM-1: Happy path — metrics with prospects
**Setup:** Pipeline with prospects at various stages, some with stage transitions.
```json
{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"pipeline_metrics","arguments":{"pipeline_name":"sales-q1"}}}
```
**Expected:** Returns `stage_metrics` (counts + avg_hours per stage) and `conversions` (rates between stages).

### PM-2: Metrics on empty pipeline (no prospects)
```json
{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"pipeline_metrics","arguments":{"pipeline_name":"sales-q1"}}}
```
**Expected:** All counts 0, conversion rates 0%, null avg_hours.

### PM-3: Reject non-existent pipeline
```json
{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":"pipeline_metrics","arguments":{"pipeline_name":"nonexistent"}}}
```
**Expected:** Error about pipeline not found.

### PM-4: Reject missing pipeline_name
```json
{"jsonrpc":"2.0","id":53,"method":"tools/call","params":{"name":"pipeline_metrics","arguments":{}}}
```
**Expected:** Error `"Missing required parameter: pipeline_name"`

---

## Tool: `stale_deals`

### SD-1: Happy path — find stale prospects
**Setup:** Pipeline with prospects whose last_activity > 14 days ago.
```json
{"jsonrpc":"2.0","id":60,"method":"tools/call","params":{"name":"stale_deals","arguments":{"pipeline_name":"sales-q1","threshold_days":14}}}
```
**Expected:** Returns stale prospects sorted by last_activity ASC.

### SD-2: Default threshold (14 days)
```json
{"jsonrpc":"2.0","id":61,"method":"tools/call","params":{"name":"stale_deals","arguments":{"pipeline_name":"sales-q1"}}}
```
**Expected:** Uses 14-day default threshold.

### SD-3: No stale deals
**Setup:** All prospects have recent activity.
**Expected:** `stale_count: 0, stale_prospects: []`

### SD-4: Reject non-existent pipeline
**Expected:** Error about pipeline not found.

### SD-5: Reject negative threshold_days
```json
{"jsonrpc":"2.0","id":63,"method":"tools/call","params":{"name":"stale_deals","arguments":{"pipeline_name":"sales-q1","threshold_days":-5}}}
```
**Expected:** Error `"threshold_days must be at least 1"`

### SD-6: Reject zero threshold_days
```json
{"jsonrpc":"2.0","id":64,"method":"tools/call","params":{"name":"stale_deals","arguments":{"pipeline_name":"sales-q1","threshold_days":0}}}
```
**Expected:** Error `"threshold_days must be at least 1"`

---

## Tool: `export_pipeline`

### EP-1: Happy path — export full pipeline
```json
{"jsonrpc":"2.0","id":70,"method":"tools/call","params":{"name":"export_pipeline","arguments":{"pipeline_name":"sales-q1"}}}
```
**Expected:** Returns pipeline definition, total_prospects count, each prospect with their stage_history array.

### EP-2: Export empty pipeline (no prospects)
**Expected:** `total_prospects: 0, prospects: []`

### EP-3: Reject non-existent pipeline
**Expected:** Error about pipeline not found.

### EP-4: Reject missing pipeline_name
**Expected:** Error `"Missing required parameter: pipeline_name"`

---

## Tool: Unknown tool

### UT-1: Call unknown tool name
```json
{"jsonrpc":"2.0","id":80,"method":"tools/call","params":{"name":"delete_everything","arguments":{}}}
```
**Expected:** Error `"Unknown tool: delete_everything"`

---

## Security Checks

### SEC-1: SQL injection in pipeline name
```json
{"jsonrpc":"2.0","id":90,"method":"tools/call","params":{"name":"create_pipeline","arguments":{"name":"'; DROP TABLE pipeline.pipelines; --","stages":["a","b"]}}}
```
**Expected:** Pipeline created with literal name (parameterized query prevents injection).

### SEC-2: SQL injection in contact_email
```json
{"jsonrpc":"2.0","id":91,"method":"tools/call","params":{"name":"add_prospect","arguments":{"pipeline_name":"sales-q1","contact_email":"' OR 1=1; --@evil.com"}}}
```
**Expected:** Rejected by email validation (invalid format), or if somehow valid, parameterized query prevents injection.

### SEC-3: SQL injection in notes field
```json
{"jsonrpc":"2.0","id":92,"method":"tools/call","params":{"name":"advance_prospect","arguments":{"contact_email":"test@example.com","to_stage":"qualified","notes":"'); DELETE FROM pipeline.prospects; --"}}}
```
**Expected:** Notes stored as literal text. No SQL execution.

### SEC-4: Extremely long string (DoS attempt)
Send a 10,000+ character company name.
**Expected:** Error `"company exceeds maximum length of 5000 characters"`

---

## Schema Integrity

### SI-1: lead_score CHECK constraint
Direct SQL: `INSERT INTO pipeline.prospects (..., lead_score, ...) VALUES (..., 150, ...)`
**Expected:** CHECK constraint violation.

### SI-2: Unique constraint on (pipeline_id, contact_email)
Direct SQL: Insert two prospects with same pipeline_id + contact_email.
**Expected:** Unique constraint violation.

### SI-3: Foreign key cascade on pipeline delete
Direct SQL: Delete a pipeline → all its prospects and their stage_history should cascade.
**Expected:** Prospects and history deleted.

### SI-4: Index on last_activity
`EXPLAIN ANALYZE SELECT * FROM pipeline.prospects WHERE last_activity < now() - interval '14 days'`
**Expected:** Uses `idx_prospects_last_activity` index.
