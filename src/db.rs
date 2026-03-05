use anyhow::Result;
use sqlx::PgPool;

pub async fn setup_schema(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE SCHEMA IF NOT EXISTS pipeline;

        CREATE TABLE IF NOT EXISTS pipeline.pipelines (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            stages      JSONB NOT NULL DEFAULT '[]',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS pipeline.prospects (
            id              TEXT PRIMARY KEY,
            pipeline_id     TEXT NOT NULL REFERENCES pipeline.pipelines(id) ON DELETE CASCADE,
            contact_email   TEXT NOT NULL,
            company         TEXT NOT NULL DEFAULT '',
            current_stage   TEXT NOT NULL,
            lead_score      INTEGER NOT NULL DEFAULT 0 CHECK (lead_score >= 0 AND lead_score <= 100),
            source          TEXT NOT NULL DEFAULT '',
            notes           TEXT NOT NULL DEFAULT '',
            entered_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
            last_activity   TIMESTAMPTZ NOT NULL DEFAULT now(),
            created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS pipeline.stage_history (
            id          TEXT PRIMARY KEY,
            prospect_id TEXT NOT NULL REFERENCES pipeline.prospects(id) ON DELETE CASCADE,
            from_stage  TEXT NOT NULL DEFAULT '',
            to_stage    TEXT NOT NULL,
            notes       TEXT NOT NULL DEFAULT '',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE INDEX IF NOT EXISTS idx_prospects_pipeline_id ON pipeline.prospects(pipeline_id);
        CREATE INDEX IF NOT EXISTS idx_prospects_current_stage ON pipeline.prospects(current_stage);
        CREATE INDEX IF NOT EXISTS idx_prospects_contact_email ON pipeline.prospects(contact_email);
        CREATE INDEX IF NOT EXISTS idx_prospects_last_activity ON pipeline.prospects(last_activity);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_prospects_pipeline_email ON pipeline.prospects(pipeline_id, contact_email);
        CREATE INDEX IF NOT EXISTS idx_stage_history_prospect_id ON pipeline.stage_history(prospect_id);
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
