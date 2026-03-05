# dataxlr8-pipeline-mcp

Sales pipeline automation MCP server for the DataXLR8 platform.

## What It Does

Manages named sales pipelines with customizable stages. Add prospects, score them, advance them through stages, track stage history, detect stale deals, and export pipeline data. Each pipeline has its own stage definitions and metrics.

## Tools

| Tool | Description |
|------|-------------|
| `create_pipeline` | Create a new pipeline with ordered stages |
| `list_pipelines` | List all pipelines |
| `add_prospect` | Add a prospect to a pipeline |
| `score_prospect` | Score a prospect with fit/intent/engagement values |
| `advance_prospect` | Move a prospect to a different stage |
| `pipeline_metrics` | Get conversion rates and stage distribution |
| `stale_deals` | Find deals stuck in a stage beyond a threshold |
| `export_pipeline` | Export full pipeline data as JSON |

## Quick Start

```bash
export DATABASE_URL=postgres://user:pass@localhost:5432/dataxlr8

cargo build
cargo run
```

## Schema

Creates a `pipeline` schema with:

| Table | Purpose |
|-------|---------|
| `pipeline.pipelines` | Pipeline definitions (name, ordered stages) |
| `pipeline.prospects` | Prospects with current stage, score, and contact info |
| `pipeline.stage_history` | Stage transition log with timestamps |

## Part of the [DataXLR8](https://github.com/pdaxt) Platform
