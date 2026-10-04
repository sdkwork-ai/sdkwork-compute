//! Internal HTTP surface for the compute domain (`/internal/v3/api/compute/*`).
//!
//! Internal APIs are consumed by the platform gateway, which applies IAM
//! upstream; the tenant scope arrives as the `X-Tenant-Id` header. The App
//! API surface (with the IAM adapter) mounts in the standalone gateway at
//! phase 1.5 (ADR-20261004 roadmap).

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::model::{ComputeAllocation, ComputeInstance, ComputeNode};
use crate::service::ComputeService;

/// Shared application state for the compute HTTP surface.
pub type ComputeHttpState = Arc<ComputeService>;

// ---------- Envelope (API_SPEC sections 14/15) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemData<T> {
    item: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageData<T> {
    items: Vec<T>,
    page_info: PageInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_items: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiEnvelope<T: Serialize> {
    code: i32,
    data: T,
    trace_id: String,
}

/// ProblemDetail per API_SPEC section 15 (`application/problem+json`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemDetail {
    #[serde(rename = "type")]
    type_uri: String,
    title: String,
    status: u16,
    detail: String,
    code: i32,
    trace_id: String,
}

fn envelope<T: Serialize>(data: T) -> ApiEnvelope<T> {
    ApiEnvelope {
        code: 0,
        data,
        trace_id: "00000000-0000-0000-0000-000000000000".to_string(),
    }
}

fn problem(status: StatusCode, code: i32, detail: &str) -> ProblemDetail {
    ProblemDetail {
        type_uri: format!("https://docs.sdkwork.com/problems/{code}"),
        title: "validation failed".to_string(),
        status: status.as_u16(),
        detail: detail.to_string(),
        code,
        trace_id: "00000000-0000-0000-0000-000000000000".to_string(),
    }
}

struct ApiProblem {
    status: StatusCode,
    body: ProblemDetail,
}

impl IntoResponse for ApiProblem {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

fn from_validation(detail: &str) -> ApiProblem {
    ApiProblem {
        status: StatusCode::BAD_REQUEST,
        body: problem(StatusCode::BAD_REQUEST, 40001, detail),
    }
}

fn internal_error(detail: &str) -> ApiProblem {
    ApiProblem {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: problem(StatusCode::INTERNAL_SERVER_ERROR, 50001, detail),
    }
}

fn not_found(detail: &str) -> ApiProblem {
    ApiProblem {
        status: StatusCode::NOT_FOUND,
        body: problem(StatusCode::NOT_FOUND, 40401, detail),
    }
}

fn require_tenant(header: Option<&str>) -> Result<String, ApiProblem> {
    // Fail closed: the gateway must inject the tenant scope; a missing
    // header must never silently widen to the platform sentinel pool.
    let Some(tenant) = header.map(str::trim) else {
        return Err(from_validation("tenant scope is required"));
    };
    if tenant.is_empty() || tenant.len() > 128 {
        return Err(from_validation("tenant scope is invalid"));
    }
    Ok(tenant.to_string())
}

fn now_stamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

// ---------- Request bodies ----------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NodeUpsertBody {
    #[serde(default)]
    display_name: Option<String>,
    node_kind: String,
    dispatch_endpoint: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    assurance_capabilities: Vec<String>,
    #[serde(default)]
    capacity_vcpu: i32,
    #[serde(default)]
    capacity_memory_mb: i64,
    #[serde(default)]
    capacity_disk_mb: i64,
    #[serde(default = "default_max_units")]
    max_units: i32,
    #[serde(default = "default_node_status")]
    status: String,
}

fn default_max_units() -> i32 {
    1
}

fn default_node_status() -> String {
    "registering".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstanceCreateBody {
    instance_id: String,
    owned_by: String,
    purpose: String,
    #[serde(default)]
    offering_id: Option<String>,
    vcpu: i32,
    memory_mb: i64,
    disk_mb: i64,
    assurance_level: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AllocationUpsertBody {
    consumer_domain: String,
    consumer_ref: String,
    #[serde(default)]
    instance_id: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    requested_target: Option<String>,
    effective_target: String,
    #[serde(default)]
    lease_owner: Option<String>,
    #[serde(default)]
    lease_expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListNodesQuery {
    #[serde(default)]
    node_kind: Option<String>,
}

// ---------- Handlers ----------

async fn put_node(
    State(state): State<ComputeHttpState>,
    Path(node_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<NodeUpsertBody>,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    let node = ComputeNode {
        tenant_id: tenant_id.clone(),
        node_id: node_id.clone(),
        display_name: body.display_name,
        node_kind: body.node_kind,
        dispatch_endpoint: body.dispatch_endpoint,
        region: body.region,
        assurance_capabilities: body.assurance_capabilities,
        capacity_vcpu: body.capacity_vcpu,
        capacity_memory_mb: body.capacity_memory_mb,
        capacity_disk_mb: body.capacity_disk_mb,
        max_units: body.max_units,
        status: body.status,
        last_heartbeat_at: None,
        health_state: None,
        metadata: Value::Object(serde_json::Map::new()),
        version: 1,
        created_at: now_stamp(),
        updated_at: now_stamp(),
        deleted_at: None,
    };
    state.register_node(node);
    let Some(stored) = state.get_node(&tenant_id, &node_id) else {
        return internal_error("node vanished immediately after registration").into_response();
    };
    (StatusCode::OK, Json(envelope(ItemData { item: stored }))).into_response()
}

async fn get_node(
    State(state): State<ComputeHttpState>,
    Path(node_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    match state.get_node(&tenant_id, &node_id) {
        Some(node) => (StatusCode::OK, Json(envelope(ItemData { item: node }))).into_response(),
        None => not_found("compute node not found").into_response(),
    }
}

async fn list_nodes(
    State(state): State<ComputeHttpState>,
    headers: HeaderMap,
    Query(query): Query<ListNodesQuery>,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    let items = state.list_eligible_nodes(&tenant_id, query.node_kind.as_deref());
    let total = items.len();
    (
        StatusCode::OK,
        Json(envelope(PageData {
            items,
            page_info: PageInfo {
                mode: "offset",
                total_items: Some(total.to_string()),
            },
        })),
    )
        .into_response()
}

async fn post_instance(
    State(state): State<ComputeHttpState>,
    headers: HeaderMap,
    Json(body): Json<InstanceCreateBody>,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    let instance = ComputeInstance {
        tenant_id: tenant_id.clone(),
        instance_id: body.instance_id.clone(),
        owned_by: body.owned_by,
        purpose: body.purpose,
        offering_id: body.offering_id,
        node_id: None,
        vcpu: body.vcpu,
        memory_mb: body.memory_mb,
        disk_mb: body.disk_mb,
        assurance_level: body.assurance_level,
        state: "requested".to_string(),
        expires_at: None,
        renewed_until: None,
        endpoints: Vec::new(),
        metrics: Value::Object(serde_json::Map::new()),
        workspace_ref: None,
        last_failure: None,
        version: 1,
        created_at: now_stamp(),
        updated_at: now_stamp(),
        deleted_at: None,
    };
    state.create_instance(instance);
    let Some(stored) = state.get_instance(&tenant_id, &body.instance_id) else {
        return internal_error("instance vanished immediately after creation").into_response();
    };
    (
        StatusCode::CREATED,
        Json(envelope(ItemData { item: stored })),
    )
        .into_response()
}

async fn get_instance(
    State(state): State<ComputeHttpState>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    match state.get_instance(&tenant_id, &instance_id) {
        Some(instance) => {
            (StatusCode::OK, Json(envelope(ItemData { item: instance }))).into_response()
        }
        None => not_found("compute instance not found").into_response(),
    }
}

async fn put_allocation(
    State(state): State<ComputeHttpState>,
    Path(allocation_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AllocationUpsertBody>,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    let allocation = ComputeAllocation {
        tenant_id: tenant_id.clone(),
        allocation_id: allocation_id.clone(),
        consumer_domain: body.consumer_domain,
        consumer_ref: body.consumer_ref,
        instance_id: body.instance_id,
        node_id: body.node_id,
        requested_target: body.requested_target,
        effective_target: body.effective_target,
        allocation_state: "active".to_string(),
        lease_owner: body.lease_owner,
        lease_expires_at: body.lease_expires_at,
        fencing_generation: 0,
        is_current: true,
        version: 1,
        created_at: now_stamp(),
        updated_at: now_stamp(),
    };
    state.record_allocation(allocation);
    let Some(stored) = state.get_allocation(&tenant_id, &allocation_id) else {
        return internal_error("allocation vanished immediately after recording").into_response();
    };
    (StatusCode::OK, Json(envelope(ItemData { item: stored }))).into_response()
}

async fn get_allocation(
    State(state): State<ComputeHttpState>,
    Path(allocation_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    match state.get_allocation(&tenant_id, &allocation_id) {
        Some(allocation) => (
            StatusCode::OK,
            Json(envelope(ItemData { item: allocation })),
        )
            .into_response(),
        None => not_found("compute allocation not found").into_response(),
    }
}

async fn post_reconcile(State(state): State<ComputeHttpState>, headers: HeaderMap) -> Response {
    let tenant_id = match tenant_scope(&headers) {
        Ok(value) => value,
        Err(problem) => return problem.into_response(),
    };
    let reconciled = state.reconcile_expired_allocations(&tenant_id, &now_stamp(), 100);
    (
        StatusCode::OK,
        Json(envelope(ItemData {
            item: serde_json::json!({ "reconciled": reconciled }),
        })),
    )
        .into_response()
}

fn tenant_scope(headers: &HeaderMap) -> Result<String, ApiProblem> {
    require_tenant(
        headers
            .get("x-tenant-id")
            .and_then(|value| value.to_str().ok()),
    )
}

/// Builds the internal compute router (`/internal/v3/api/compute/*`).
pub fn build_internal_routes(state: ComputeHttpState) -> Router {
    Router::new()
        .route(
            "/internal/v3/api/compute/nodes/{nodeId}",
            put(put_node).get(get_node),
        )
        .route("/internal/v3/api/compute/nodes", get(list_nodes))
        .route("/internal/v3/api/compute/instances", post(post_instance))
        .route(
            "/internal/v3/api/compute/instances/{instanceId}",
            get(get_instance),
        )
        .route(
            "/internal/v3/api/compute/allocations/{allocationId}",
            put(put_allocation).get(get_allocation),
        )
        .route(
            "/internal/v3/api/compute/allocations/reconcile",
            post(post_reconcile),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryComputeRepository;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> (Router, Arc<ComputeService>) {
        let service = Arc::new(ComputeService::new(Arc::new(
            InMemoryComputeRepository::new(),
        )));
        (build_internal_routes(service.clone()), service)
    }

    async fn put_node(app: &Router, node_id: &str, status: &str) -> StatusCode {
        let body = serde_json::json!({
            "nodeKind": "docker",
            "dispatchEndpoint": "unix:///run/docker.sock",
            "assuranceCapabilities": ["container"],
            "maxUnits": 4,
            "status": status
        });
        let request = Request::builder()
            .method("PUT")
            .uri(format!("/internal/v3/api/compute/nodes/{node_id}"))
            .header("content-type", "application/json")
            .header("x-tenant-id", "10")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        response.status()
    }

    #[tokio::test]
    async fn node_upsert_get_list_roundtrip() {
        let (app, _service) = test_app();
        assert_eq!(
            put_node(&app, "node.docker-01", "active").await,
            StatusCode::OK
        );

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes/node.docker-01")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes?nodeKind=docker")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn draining_node_drops_out_of_the_eligible_list() {
        let (app, _service) = test_app();
        assert_eq!(put_node(&app, "node.1", "active").await, StatusCode::OK);

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let body = to_bytes(response).await;
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);

        assert_eq!(put_node(&app, "node.1", "draining").await, StatusCode::OK);

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let body = to_bytes(response).await;
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn missing_tenant_scope_is_rejected() {
        let (app, _service) = test_app();
        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes/node.missing")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_node_reads_as_404_problem() {
        let (app, _service) = test_app();
        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/nodes/node.missing")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn instance_create_get_roundtrip() {
        let (app, _service) = test_app();
        let body = serde_json::json!({
            "instanceId": "instance.1",
            "ownedBy": "user.100",
            "purpose": "sandbox",
            "vcpu": 2,
            "memoryMb": 2048,
            "diskMb": 20480,
            "assuranceLevel": "container"
        });
        let request = Request::builder()
            .method("POST")
            .uri("/internal/v3/api/compute/instances")
            .header("content-type", "application/json")
            .header("x-tenant-id", "10")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/instances/instance.1")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn allocation_put_get_and_reconcile_roundtrip() {
        let (app, service) = test_app();
        let body = serde_json::json!({
            "consumerDomain": "agents",
            "consumerRef": "session.test",
            "requestedTarget": "cloud",
            "effectiveTarget": "cloud",
            "leaseOwner": "worker-1",
            "leaseExpiresAt": "2026-10-03T00:00:00Z"
        });
        let request = Request::builder()
            .method("PUT")
            .uri("/internal/v3/api/compute/allocations/allocation.1")
            .header("content-type", "application/json")
            .header("x-tenant-id", "10")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/allocations/allocation.1")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // A future as_of finds nothing expired; a past as_of expires it.
        assert_eq!(
            service.reconcile_expired_allocations("10", "2026-10-02T00:00:00Z", 10),
            0
        );
        assert_eq!(
            service.reconcile_expired_allocations("10", "2026-10-04T00:00:00Z", 10),
            1
        );

        let request = Request::builder()
            .method("GET")
            .uri("/internal/v3/api/compute/allocations/allocation.1")
            .header("x-tenant-id", "10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let body = to_bytes(response).await;
        assert_eq!(body["data"]["item"]["allocationState"], "expired");
    }

    async fn to_bytes(response: Response) -> serde_json::Value {
        let bytes = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        serde_json::from_slice(&bytes).expect("json body")
    }
}
