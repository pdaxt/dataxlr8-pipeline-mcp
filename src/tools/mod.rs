use dataxlr8_mcp_core::mcp::{empty_schema, error_result, get_i64, get_str, json_result, make_schema};
use dataxlr8_mcp_core::Database;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ServerHandler;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

// ============================================================================
// Data types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub stages: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Prospect {
    pub id: String,
    pub pipeline_id: String,
    pub contact_email: String,
    pub company: String,
    pub current_stage: String,
    pub lead_score: i32,
    pub source: String,
    pub notes: String,
    pub entered_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StageHistory {
    pub id: String,
    pub prospect_id: String,
    pub from_stage: String,
    pub to_stage: String,
    pub notes: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Tool definitions
// ============================================================================

fn build_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "create_pipeline".into(),
            title: None,
            description: Some("Create a named sales pipeline with custom stages".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "name": { "type": "string", "description": "Unique pipeline name" },
                    "stages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Ordered list of stage names (e.g. [\"lead\", \"qualified\", \"proposal\", \"negotiation\", \"closed_won\", \"closed_lost\"])"
                    }
                }),
                vec!["name", "stages"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "list_pipelines".into(),
            title: None,
            description: Some("List all sales pipelines with prospect counts per stage".into()),
            input_schema: empty_schema(),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "add_prospect".into(),
            title: None,
            description: Some("Add a prospect to a pipeline with source and initial score".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "pipeline_name": { "type": "string", "description": "Name of the pipeline to add prospect to" },
                    "contact_email": { "type": "string", "description": "Prospect's email address" },
                    "company": { "type": "string", "description": "Company name" },
                    "source": { "type": "string", "description": "Lead source (e.g. website, referral, linkedin)" },
                    "lead_score": { "type": "integer", "description": "Initial lead score 0-100 (default: 0)" },
                    "notes": { "type": "string", "description": "Initial notes about the prospect" }
                }),
                vec!["pipeline_name", "contact_email"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "score_prospect".into(),
            title: None,
            description: Some("Update a prospect's lead score based on engagement signals".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "contact_email": { "type": "string", "description": "Prospect's email address" },
                    "score_delta": { "type": "integer", "description": "Score change (positive or negative)" },
                    "reason": { "type": "string", "description": "Reason for score change (e.g. 'opened email', 'attended demo')" }
                }),
                vec!["contact_email", "score_delta"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "advance_prospect".into(),
            title: None,
            description: Some("Move a prospect to the next stage in their pipeline with notes".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "contact_email": { "type": "string", "description": "Prospect's email address" },
                    "to_stage": { "type": "string", "description": "Target stage name (must be a valid stage in the prospect's pipeline)" },
                    "notes": { "type": "string", "description": "Notes about why the prospect is advancing" }
                }),
                vec!["contact_email", "to_stage"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "pipeline_metrics".into(),
            title: None,
            description: Some("Get conversion rates between stages and average time per stage for a pipeline".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "pipeline_name": { "type": "string", "description": "Name of the pipeline to analyze" }
                }),
                vec!["pipeline_name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "stale_deals".into(),
            title: None,
            description: Some("Find prospects stuck in a stage beyond a threshold number of days".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "pipeline_name": { "type": "string", "description": "Name of the pipeline to check" },
                    "threshold_days": { "type": "integer", "description": "Number of days without activity to consider stale (default: 14)" }
                }),
                vec!["pipeline_name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        Tool {
            name: "export_pipeline".into(),
            title: None,
            description: Some("Export full pipeline data as JSON including all prospects and stage history".into()),
            input_schema: make_schema(
                serde_json::json!({
                    "pipeline_name": { "type": "string", "description": "Name of the pipeline to export" }
                }),
                vec!["pipeline_name"],
            ),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
    ]
}

// ============================================================================
// MCP Server
// ============================================================================

#[derive(Clone)]
pub struct PipelineMcpServer {
    db: Database,
}

impl PipelineMcpServer {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // ---- Tool handlers ----

    async fn handle_create_pipeline(&self, args: &serde_json::Value) -> CallToolResult {
        let name = match get_str(args, "name") {
            Some(n) => n,
            None => return error_result("Missing required parameter: name"),
        };

        let stages: Vec<String> = match args.get("stages").and_then(|v| v.as_array()) {
            Some(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            None => return error_result("Missing required parameter: stages (must be a JSON array of strings)"),
        };

        if stages.len() < 2 {
            return error_result("Pipeline must have at least 2 stages");
        }

        let id = uuid::Uuid::new_v4().to_string();
        let stages_json = serde_json::to_value(&stages).unwrap_or_default();

        match sqlx::query_as::<_, Pipeline>(
            "INSERT INTO pipeline.pipelines (id, name, stages) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(&id)
        .bind(&name)
        .bind(&stages_json)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(pipeline) => {
                info!(name = name, stages = ?stages, "Created pipeline");
                json_result(&pipeline)
            }
            Err(e) => error_result(&format!("Failed to create pipeline: {e}")),
        }
    }

    async fn handle_list_pipelines(&self) -> CallToolResult {
        let pipelines: Vec<Pipeline> = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines ORDER BY created_at",
        )
        .fetch_all(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        if pipelines.is_empty() {
            return json_result(&serde_json::json!({ "pipelines": [], "message": "No pipelines found" }));
        }

        // Fetch prospect counts per pipeline+stage in one query
        #[derive(sqlx::FromRow)]
        struct StageCounts {
            pipeline_id: String,
            current_stage: String,
            count: i64,
        }

        let pipeline_ids: Vec<String> = pipelines.iter().map(|p| p.id.clone()).collect();
        let counts: Vec<StageCounts> = match sqlx::query_as::<_, StageCounts>(
            "SELECT pipeline_id, current_stage, COUNT(*)::BIGINT as count FROM pipeline.prospects WHERE pipeline_id = ANY($1) GROUP BY pipeline_id, current_stage",
        )
        .bind(&pipeline_ids)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Failed to fetch stage counts");
                Vec::new()
            }
        };

        // Group counts by pipeline_id
        let mut count_map: std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>> = std::collections::HashMap::new();
        for sc in counts {
            count_map
                .entry(sc.pipeline_id)
                .or_default()
                .insert(sc.current_stage, serde_json::json!(sc.count));
        }

        let results: Vec<serde_json::Value> = pipelines
            .into_iter()
            .map(|p| {
                let stage_counts = count_map.remove(&p.id).unwrap_or_default();
                serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "stages": p.stages,
                    "stage_counts": stage_counts,
                    "created_at": p.created_at,
                })
            })
            .collect();

        json_result(&results)
    }

    async fn handle_add_prospect(&self, args: &serde_json::Value) -> CallToolResult {
        let pipeline_name = match get_str(args, "pipeline_name") {
            Some(n) => n,
            None => return error_result("Missing required parameter: pipeline_name"),
        };
        let contact_email = match get_str(args, "contact_email") {
            Some(e) => e,
            None => return error_result("Missing required parameter: contact_email"),
        };
        let company = get_str(args, "company").unwrap_or_default();
        let source = get_str(args, "source").unwrap_or_default();
        let lead_score = get_i64(args, "lead_score").unwrap_or(0) as i32;
        let notes = get_str(args, "notes").unwrap_or_default();

        // Look up pipeline
        let pipeline: Option<Pipeline> = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines WHERE name = $1",
        )
        .bind(&pipeline_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let pipeline = match pipeline {
            Some(p) => p,
            None => return error_result(&format!("Pipeline '{pipeline_name}' not found")),
        };

        // First stage is entry point
        let first_stage = match pipeline.stages.as_array().and_then(|a| a.first()).and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return error_result("Pipeline has no stages configured"),
        };

        let id = uuid::Uuid::new_v4().to_string();

        match sqlx::query_as::<_, Prospect>(
            "INSERT INTO pipeline.prospects (id, pipeline_id, contact_email, company, current_stage, lead_score, source, notes) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(&id)
        .bind(&pipeline.id)
        .bind(&contact_email)
        .bind(&company)
        .bind(&first_stage)
        .bind(lead_score)
        .bind(&source)
        .bind(&notes)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(prospect) => {
                // Record initial stage entry in history
                let hist_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO pipeline.stage_history (id, prospect_id, from_stage, to_stage, notes) VALUES ($1, $2, '', $3, 'Initial entry')",
                )
                .bind(&hist_id)
                .bind(&id)
                .bind(&first_stage)
                .execute(self.db.pool())
                .await;

                info!(email = contact_email, pipeline = pipeline_name, "Added prospect");
                json_result(&prospect)
            }
            Err(e) => error_result(&format!("Failed to add prospect: {e}")),
        }
    }

    async fn handle_score_prospect(&self, args: &serde_json::Value) -> CallToolResult {
        let contact_email = match get_str(args, "contact_email") {
            Some(e) => e,
            None => return error_result("Missing required parameter: contact_email"),
        };
        let score_delta = match get_i64(args, "score_delta") {
            Some(d) => d as i32,
            None => return error_result("Missing required parameter: score_delta"),
        };
        let reason = get_str(args, "reason").unwrap_or_default();

        // Clamp score between 0 and 100
        match sqlx::query_as::<_, Prospect>(
            "UPDATE pipeline.prospects SET lead_score = GREATEST(0, LEAST(100, lead_score + $1)), last_activity = now() WHERE contact_email = $2 RETURNING *",
        )
        .bind(score_delta)
        .bind(&contact_email)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(Some(prospect)) => {
                info!(email = contact_email, delta = score_delta, reason = reason, "Scored prospect");
                json_result(&serde_json::json!({
                    "prospect": prospect,
                    "score_delta": score_delta,
                    "reason": reason,
                }))
            }
            Ok(None) => error_result(&format!("Prospect with email '{contact_email}' not found")),
            Err(e) => error_result(&format!("Failed to score prospect: {e}")),
        }
    }

    async fn handle_advance_prospect(&self, args: &serde_json::Value) -> CallToolResult {
        let contact_email = match get_str(args, "contact_email") {
            Some(e) => e,
            None => return error_result("Missing required parameter: contact_email"),
        };
        let to_stage = match get_str(args, "to_stage") {
            Some(s) => s,
            None => return error_result("Missing required parameter: to_stage"),
        };
        let notes = get_str(args, "notes").unwrap_or_default();

        // Fetch prospect with pipeline
        let prospect: Option<Prospect> = match sqlx::query_as(
            "SELECT * FROM pipeline.prospects WHERE contact_email = $1",
        )
        .bind(&contact_email)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let prospect = match prospect {
            Some(p) => p,
            None => return error_result(&format!("Prospect with email '{contact_email}' not found")),
        };

        // Validate target stage exists in pipeline
        let pipeline: Pipeline = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines WHERE id = $1",
        )
        .bind(&prospect.pipeline_id)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let stages: Vec<String> = pipeline
            .stages
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        if !stages.contains(&to_stage) {
            return error_result(&format!(
                "Stage '{to_stage}' not found in pipeline. Valid stages: {}",
                stages.join(", ")
            ));
        }

        let from_stage = prospect.current_stage.clone();

        // Update prospect stage
        match sqlx::query_as::<_, Prospect>(
            "UPDATE pipeline.prospects SET current_stage = $1, last_activity = now() WHERE id = $2 RETURNING *",
        )
        .bind(&to_stage)
        .bind(&prospect.id)
        .fetch_one(self.db.pool())
        .await
        {
            Ok(updated) => {
                // Record stage transition
                let hist_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO pipeline.stage_history (id, prospect_id, from_stage, to_stage, notes) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(&hist_id)
                .bind(&prospect.id)
                .bind(&from_stage)
                .bind(&to_stage)
                .bind(&notes)
                .execute(self.db.pool())
                .await;

                info!(email = contact_email, from = from_stage, to = to_stage, "Advanced prospect");
                json_result(&serde_json::json!({
                    "prospect": updated,
                    "from_stage": from_stage,
                    "to_stage": to_stage,
                }))
            }
            Err(e) => error_result(&format!("Failed to advance prospect: {e}")),
        }
    }

    async fn handle_pipeline_metrics(&self, pipeline_name: &str) -> CallToolResult {
        // Fetch pipeline
        let pipeline: Option<Pipeline> = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines WHERE name = $1",
        )
        .bind(pipeline_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let pipeline = match pipeline {
            Some(p) => p,
            None => return error_result(&format!("Pipeline '{pipeline_name}' not found")),
        };

        let stages: Vec<String> = pipeline
            .stages
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Count prospects per stage
        #[derive(sqlx::FromRow)]
        struct StageCount {
            current_stage: String,
            count: i64,
        }

        let stage_counts: Vec<StageCount> = match sqlx::query_as::<_, StageCount>(
            "SELECT current_stage, COUNT(*)::BIGINT as count FROM pipeline.prospects WHERE pipeline_id = $1 GROUP BY current_stage",
        )
        .bind(&pipeline.id)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(c) => c,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let count_map: std::collections::HashMap<&str, i64> =
            stage_counts.iter().map(|sc| (sc.current_stage.as_str(), sc.count)).collect();

        // Conversion rates between consecutive stages
        let mut conversions = Vec::new();
        for i in 0..stages.len().saturating_sub(1) {
            let from = &stages[i];
            let to = &stages[i + 1];

            // Count transitions from stage[i] to stage[i+1]
            #[derive(sqlx::FromRow)]
            struct TransCount {
                count: i64,
            }

            let transitioned: i64 = match sqlx::query_as::<_, TransCount>(
                "SELECT COUNT(*)::BIGINT as count FROM pipeline.stage_history sh JOIN pipeline.prospects p ON sh.prospect_id = p.id WHERE p.pipeline_id = $1 AND sh.from_stage = $2 AND sh.to_stage = $3",
            )
            .bind(&pipeline.id)
            .bind(from)
            .bind(to)
            .fetch_one(self.db.pool())
            .await
            {
                Ok(tc) => tc.count,
                Err(_) => 0,
            };

            // How many ever entered 'from' stage
            let entered_from: i64 = match sqlx::query_as::<_, TransCount>(
                "SELECT COUNT(*)::BIGINT as count FROM pipeline.stage_history sh JOIN pipeline.prospects p ON sh.prospect_id = p.id WHERE p.pipeline_id = $1 AND sh.to_stage = $2",
            )
            .bind(&pipeline.id)
            .bind(from)
            .fetch_one(self.db.pool())
            .await
            {
                Ok(tc) => tc.count,
                Err(_) => 0,
            };

            let rate = if entered_from > 0 {
                (transitioned as f64 / entered_from as f64 * 100.0).round()
            } else {
                0.0
            };

            conversions.push(serde_json::json!({
                "from": from,
                "to": to,
                "entered_from": entered_from,
                "advanced_to": transitioned,
                "conversion_rate_pct": rate,
            }));
        }

        // Avg time per stage (from stage_history timestamps)
        #[derive(sqlx::FromRow)]
        struct AvgTime {
            from_stage: String,
            avg_hours: Option<f64>,
        }

        let avg_times: Vec<AvgTime> = match sqlx::query_as::<_, AvgTime>(
            r#"SELECT sh1.to_stage as from_stage,
                      AVG(EXTRACT(EPOCH FROM (sh2.created_at - sh1.created_at)) / 3600)::FLOAT8 as avg_hours
               FROM pipeline.stage_history sh1
               JOIN pipeline.stage_history sh2 ON sh1.prospect_id = sh2.prospect_id AND sh2.from_stage = sh1.to_stage
               JOIN pipeline.prospects p ON sh1.prospect_id = p.id
               WHERE p.pipeline_id = $1
               GROUP BY sh1.to_stage"#,
        )
        .bind(&pipeline.id)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(t) => t,
            Err(_) => Vec::new(),
        };

        let time_map: std::collections::HashMap<&str, f64> =
            avg_times.iter().filter_map(|at| at.avg_hours.map(|h| (at.from_stage.as_str(), h))).collect();

        let stage_metrics: Vec<serde_json::Value> = stages
            .iter()
            .map(|s| {
                serde_json::json!({
                    "stage": s,
                    "current_count": count_map.get(s.as_str()).copied().unwrap_or(0),
                    "avg_hours_in_stage": time_map.get(s.as_str()).copied(),
                })
            })
            .collect();

        json_result(&serde_json::json!({
            "pipeline": pipeline_name,
            "stage_metrics": stage_metrics,
            "conversions": conversions,
        }))
    }

    async fn handle_stale_deals(&self, pipeline_name: &str, threshold_days: i64) -> CallToolResult {
        // Fetch pipeline
        let pipeline: Option<Pipeline> = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines WHERE name = $1",
        )
        .bind(pipeline_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let pipeline = match pipeline {
            Some(p) => p,
            None => return error_result(&format!("Pipeline '{pipeline_name}' not found")),
        };

        let stale: Vec<Prospect> = match sqlx::query_as::<_, Prospect>(
            "SELECT * FROM pipeline.prospects WHERE pipeline_id = $1 AND last_activity < now() - make_interval(days => $2) ORDER BY last_activity ASC",
        )
        .bind(&pipeline.id)
        .bind(threshold_days as i32)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(s) => s,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        json_result(&serde_json::json!({
            "pipeline": pipeline_name,
            "threshold_days": threshold_days,
            "stale_count": stale.len(),
            "stale_prospects": stale,
        }))
    }

    async fn handle_export_pipeline(&self, pipeline_name: &str) -> CallToolResult {
        // Fetch pipeline
        let pipeline: Option<Pipeline> = match sqlx::query_as(
            "SELECT * FROM pipeline.pipelines WHERE name = $1",
        )
        .bind(pipeline_name)
        .fetch_optional(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        let pipeline = match pipeline {
            Some(p) => p,
            None => return error_result(&format!("Pipeline '{pipeline_name}' not found")),
        };

        // Fetch all prospects
        let prospects: Vec<Prospect> = match sqlx::query_as(
            "SELECT * FROM pipeline.prospects WHERE pipeline_id = $1 ORDER BY created_at",
        )
        .bind(&pipeline.id)
        .fetch_all(self.db.pool())
        .await
        {
            Ok(p) => p,
            Err(e) => return error_result(&format!("Database error: {e}")),
        };

        // Fetch all stage history for these prospects in one query
        let prospect_ids: Vec<String> = prospects.iter().map(|p| p.id.clone()).collect();
        let history: Vec<StageHistory> = if prospect_ids.is_empty() {
            Vec::new()
        } else {
            match sqlx::query_as::<_, StageHistory>(
                "SELECT * FROM pipeline.stage_history WHERE prospect_id = ANY($1) ORDER BY created_at",
            )
            .bind(&prospect_ids)
            .fetch_all(self.db.pool())
            .await
            {
                Ok(h) => h,
                Err(e) => {
                    error!(error = %e, "Failed to fetch stage history");
                    Vec::new()
                }
            }
        };

        // Group history by prospect_id
        let mut history_map: std::collections::HashMap<String, Vec<StageHistory>> = std::collections::HashMap::new();
        for h in history {
            history_map.entry(h.prospect_id.clone()).or_default().push(h);
        }

        let prospect_exports: Vec<serde_json::Value> = prospects
            .into_iter()
            .map(|p| {
                let hist = history_map.remove(&p.id).unwrap_or_default();
                serde_json::json!({
                    "prospect": p,
                    "stage_history": hist,
                })
            })
            .collect();

        json_result(&serde_json::json!({
            "pipeline": pipeline,
            "total_prospects": prospect_exports.len(),
            "prospects": prospect_exports,
        }))
    }
}

// ============================================================================
// ServerHandler trait implementation
// ============================================================================

impl ServerHandler for PipelineMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "DataXLR8 Pipeline MCP — sales pipeline automation with prospect tracking, scoring, and conversion analytics"
                    .into(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + Send + '_ {
        async {
            Ok(ListToolsResult {
                tools: build_tools(),
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + Send + '_ {
        async move {
            let args = serde_json::to_value(&request.arguments).unwrap_or(serde_json::Value::Null);
            let name_str: &str = request.name.as_ref();

            let result = match name_str {
                "create_pipeline" => self.handle_create_pipeline(&args).await,
                "list_pipelines" => self.handle_list_pipelines().await,
                "add_prospect" => self.handle_add_prospect(&args).await,
                "score_prospect" => self.handle_score_prospect(&args).await,
                "advance_prospect" => self.handle_advance_prospect(&args).await,
                "pipeline_metrics" => {
                    match get_str(&args, "pipeline_name") {
                        Some(name) => self.handle_pipeline_metrics(&name).await,
                        None => error_result("Missing required parameter: pipeline_name"),
                    }
                }
                "stale_deals" => {
                    match get_str(&args, "pipeline_name") {
                        Some(name) => {
                            let threshold = get_i64(&args, "threshold_days").unwrap_or(14);
                            self.handle_stale_deals(&name, threshold).await
                        }
                        None => error_result("Missing required parameter: pipeline_name"),
                    }
                }
                "export_pipeline" => {
                    match get_str(&args, "pipeline_name") {
                        Some(name) => self.handle_export_pipeline(&name).await,
                        None => error_result("Missing required parameter: pipeline_name"),
                    }
                }
                _ => error_result(&format!("Unknown tool: {}", request.name)),
            };

            Ok(result)
        }
    }
}
