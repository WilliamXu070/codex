use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct IndexDirectoryParams {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct IndexDirectoryResponse {
    pub started: bool,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct QueryContextParams {
    pub query: String,
    pub max_results: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct QueryContextResponse {
    pub nodes: Vec<ContextNodeSummary>,
    pub processing_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct GetNodeContextParams {
    pub node_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct GetNodeContextResponse {
    pub node: ContextNodeSummary,
    pub ancestry: Vec<ContextNodeSummary>,
    pub related: Vec<ContextNodeSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListDomainsParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListDomainsResponse {
    pub domains: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ContextNodeSummary {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub path: Option<String>,
    pub summary: String,
    pub depth: u32,
    pub keywords: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type", rename_all = "camelCase", export_to = "v2/")]
pub enum IndexStatus {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Starting { path: String },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Processing {
        file: String,
        progress: usize,
        total: usize,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Analyzing { stage: String },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Complete {
        nodes_created: usize,
        files_processed: usize,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct IndexProgressNotification {
    pub status: IndexStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct IndexCompleteNotification {
    pub domain: String,
    pub files_processed: usize,
    pub nodes_created: usize,
    pub entities_extracted: usize,
    pub processing_time_ms: u64,
}
