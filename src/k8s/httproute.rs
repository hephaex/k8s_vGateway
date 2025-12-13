//! HTTPRoute resource management
//!
//! Provides CRUD operations and builders for HTTPRoute resources.

#![allow(dead_code)]

use anyhow::{Context, Result};
use kube::api::{Api, ListParams, PostParams};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::K8sClient;

/// HTTPRoute custom resource specification
#[derive(CustomResource, Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[kube(
    group = "gateway.networking.k8s.io",
    version = "v1",
    kind = "HTTPRoute",
    namespaced
)]
#[kube(status = "HTTPRouteStatus")]
pub struct HTTPRouteSpec {
    /// Parent references (gateways)
    #[serde(rename = "parentRefs", default)]
    pub parent_refs: Vec<ParentRef>,

    /// Hostnames for this route
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<String>,

    /// Routing rules
    #[serde(default)]
    pub rules: Vec<HTTPRouteRule>,
}

/// Parent reference (gateway)
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct ParentRef {
    /// Name of the parent resource
    pub name: String,

    /// Namespace of the parent resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Kind of the parent resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Section name within the parent
    #[serde(rename = "sectionName", skip_serializing_if = "Option::is_none")]
    pub section_name: Option<String>,
}

/// HTTPRoute routing rule
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRouteRule {
    /// Match conditions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<HTTPRouteMatch>,

    /// Backend references
    #[serde(rename = "backendRefs", default, skip_serializing_if = "Vec::is_empty")]
    pub backend_refs: Vec<HTTPBackendRef>,

    /// Filters to apply
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<HTTPRouteFilter>,

    /// Timeout configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<HTTPRouteTimeouts>,
}

/// HTTPRoute match conditions
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRouteMatch {
    /// Path match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPPathMatch>,

    /// Header matches
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<HTTPHeaderMatch>,

    /// Query parameter matches
    #[serde(rename = "queryParams", default, skip_serializing_if = "Vec::is_empty")]
    pub query_params: Vec<HTTPQueryParamMatch>,

    /// HTTP method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// Path match configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPPathMatch {
    /// Match type (Exact, PathPrefix, RegularExpression)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,

    /// Path value
    pub value: String,
}

/// Header match configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPHeaderMatch {
    /// Header name
    pub name: String,

    /// Match type (Exact, RegularExpression)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,

    /// Header value
    pub value: String,
}

/// Query parameter match
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPQueryParamMatch {
    /// Parameter name
    pub name: String,

    /// Match type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,

    /// Parameter value
    pub value: String,
}

/// Backend reference
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPBackendRef {
    /// Name of the backend service
    pub name: String,

    /// Port of the backend service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Weight for traffic splitting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,

    /// Namespace of the backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Kind of the backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// HTTPRoute filter
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRouteFilter {
    /// Filter type
    #[serde(rename = "type")]
    pub filter_type: String,

    /// Request header modifier
    #[serde(
        rename = "requestHeaderModifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_header_modifier: Option<HTTPHeaderModifier>,

    /// Response header modifier
    #[serde(
        rename = "responseHeaderModifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_header_modifier: Option<HTTPHeaderModifier>,

    /// Request redirect
    #[serde(rename = "requestRedirect", skip_serializing_if = "Option::is_none")]
    pub request_redirect: Option<HTTPRequestRedirect>,

    /// URL rewrite
    #[serde(rename = "urlRewrite", skip_serializing_if = "Option::is_none")]
    pub url_rewrite: Option<HTTPURLRewrite>,
}

/// Header modifier
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPHeaderModifier {
    /// Headers to add
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<HTTPHeader>,

    /// Headers to set
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set: Vec<HTTPHeader>,

    /// Headers to remove
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
}

/// HTTP header
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPHeader {
    /// Header name
    pub name: String,

    /// Header value
    pub value: String,
}

/// Request redirect configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRequestRedirect {
    /// Redirect scheme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    /// Redirect hostname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// Redirect path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPPathModifier>,

    /// Redirect port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Status code
    #[serde(rename = "statusCode", skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

/// Path modifier for redirect/rewrite
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPPathModifier {
    /// Modifier type (ReplaceFullPath, ReplacePrefixMatch)
    #[serde(rename = "type")]
    pub modifier_type: String,

    /// Replace full path value
    #[serde(rename = "replaceFullPath", skip_serializing_if = "Option::is_none")]
    pub replace_full_path: Option<String>,

    /// Replace prefix match value
    #[serde(rename = "replacePrefixMatch", skip_serializing_if = "Option::is_none")]
    pub replace_prefix_match: Option<String>,
}

/// URL rewrite configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPURLRewrite {
    /// Hostname rewrite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// Path rewrite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPPathModifier>,
}

/// Timeout configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRouteTimeouts {
    /// Request timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,

    /// Backend request timeout
    #[serde(rename = "backendRequest", skip_serializing_if = "Option::is_none")]
    pub backend_request: Option<String>,
}

/// HTTPRoute status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct HTTPRouteStatus {
    /// Parent statuses
    #[serde(default)]
    pub parents: Vec<RouteParentStatus>,
}

/// Route parent status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct RouteParentStatus {
    /// Parent reference
    #[serde(rename = "parentRef")]
    pub parent_ref: ParentRef,

    /// Controller name
    #[serde(rename = "controllerName")]
    pub controller_name: String,

    /// Conditions
    #[serde(default)]
    pub conditions: Vec<RouteCondition>,
}

/// Route condition
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct RouteCondition {
    /// Condition type
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status
    pub status: String,

    /// Reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// HTTPRoute builder
pub struct HTTPRouteBuilder {
    name: String,
    namespace: String,
    parent_refs: Vec<ParentRef>,
    hostnames: Vec<String>,
    rules: Vec<HTTPRouteRule>,
}

impl HTTPRouteBuilder {
    /// Create a new HTTPRoute builder
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            parent_refs: Vec::new(),
            hostnames: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Add parent reference (gateway)
    pub fn parent_ref(mut self, gateway_name: impl Into<String>) -> Self {
        self.parent_refs.push(ParentRef {
            name: gateway_name.into(),
            ..Default::default()
        });
        self
    }

    /// Add parent reference with namespace
    pub fn parent_ref_namespaced(
        mut self,
        gateway_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        self.parent_refs.push(ParentRef {
            name: gateway_name.into(),
            namespace: Some(namespace.into()),
            ..Default::default()
        });
        self
    }

    /// Add hostname
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostnames.push(hostname.into());
        self
    }

    /// Add a rule
    pub fn rule(mut self, rule: HTTPRouteRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Build the HTTPRoute
    pub fn build(self) -> HTTPRoute {
        HTTPRoute::new(
            &self.name,
            HTTPRouteSpec {
                parent_refs: self.parent_refs,
                hostnames: self.hostnames,
                rules: self.rules,
            },
        )
    }
}

/// Rule builder for HTTPRoute rules
pub struct RuleBuilder {
    matches: Vec<HTTPRouteMatch>,
    backend_refs: Vec<HTTPBackendRef>,
    filters: Vec<HTTPRouteFilter>,
    timeouts: Option<HTTPRouteTimeouts>,
}

impl RuleBuilder {
    /// Create a new rule builder
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
            backend_refs: Vec::new(),
            filters: Vec::new(),
            timeouts: None,
        }
    }

    /// Add path prefix match
    pub fn path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.matches.push(HTTPRouteMatch {
            path: Some(HTTPPathMatch {
                match_type: Some("PathPrefix".to_string()),
                value: prefix.into(),
            }),
            ..Default::default()
        });
        self
    }

    /// Add exact path match
    pub fn path_exact(mut self, path: impl Into<String>) -> Self {
        self.matches.push(HTTPRouteMatch {
            path: Some(HTTPPathMatch {
                match_type: Some("Exact".to_string()),
                value: path.into(),
            }),
            ..Default::default()
        });
        self
    }

    /// Add header match
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let match_entry = self.matches.last_mut();
        match match_entry {
            Some(m) => {
                m.headers.push(HTTPHeaderMatch {
                    name: name.into(),
                    match_type: Some("Exact".to_string()),
                    value: value.into(),
                });
            }
            None => {
                self.matches.push(HTTPRouteMatch {
                    headers: vec![HTTPHeaderMatch {
                        name: name.into(),
                        match_type: Some("Exact".to_string()),
                        value: value.into(),
                    }],
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Add backend service
    pub fn backend(mut self, name: impl Into<String>, port: u16) -> Self {
        self.backend_refs.push(HTTPBackendRef {
            name: name.into(),
            port: Some(port),
            ..Default::default()
        });
        self
    }

    /// Add backend with weight
    pub fn backend_with_weight(mut self, name: impl Into<String>, port: u16, weight: i32) -> Self {
        self.backend_refs.push(HTTPBackendRef {
            name: name.into(),
            port: Some(port),
            weight: Some(weight),
            ..Default::default()
        });
        self
    }

    /// Add HTTPS redirect
    pub fn redirect_https(mut self) -> Self {
        self.filters.push(HTTPRouteFilter {
            filter_type: "RequestRedirect".to_string(),
            request_redirect: Some(HTTPRequestRedirect {
                scheme: Some("https".to_string()),
                status_code: Some(301),
                ..Default::default()
            }),
            ..Default::default()
        });
        self
    }

    /// Add URL rewrite
    pub fn url_rewrite(mut self, path: impl Into<String>) -> Self {
        self.filters.push(HTTPRouteFilter {
            filter_type: "URLRewrite".to_string(),
            url_rewrite: Some(HTTPURLRewrite {
                path: Some(HTTPPathModifier {
                    modifier_type: "ReplaceFullPath".to_string(),
                    replace_full_path: Some(path.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, duration: impl Into<String>) -> Self {
        self.timeouts = Some(HTTPRouteTimeouts {
            request: Some(duration.into()),
            ..Default::default()
        });
        self
    }

    /// Build the rule
    pub fn build(self) -> HTTPRouteRule {
        HTTPRouteRule {
            matches: self.matches,
            backend_refs: self.backend_refs,
            filters: self.filters,
            timeouts: self.timeouts,
        }
    }
}

impl Default for RuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTPRoute manager
pub struct HTTPRouteManager {
    client: K8sClient,
}

impl HTTPRouteManager {
    pub fn new(client: K8sClient) -> Self {
        Self { client }
    }

    fn api(&self, namespace: &str) -> Api<HTTPRoute> {
        Api::namespaced(self.client.client().clone(), namespace)
    }

    pub async fn create(&self, route: &HTTPRoute, namespace: &str) -> Result<HTTPRoute> {
        let api = self.api(namespace);
        api.create(&PostParams::default(), route)
            .await
            .context("Failed to create HTTPRoute")
    }

    pub async fn get(&self, name: &str, namespace: &str) -> Result<HTTPRoute> {
        let api = self.api(namespace);
        api.get(name).await.context("Failed to get HTTPRoute")
    }

    pub async fn list(&self, namespace: &str) -> Result<Vec<HTTPRoute>> {
        let api = self.api(namespace);
        let list = api
            .list(&ListParams::default())
            .await
            .context("Failed to list HTTPRoutes")?;
        Ok(list.items)
    }

    pub async fn delete(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        api.delete(name, &Default::default())
            .await
            .context("Failed to delete HTTPRoute")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_httproute_builder() {
        let route = HTTPRouteBuilder::new("test-route", "default")
            .parent_ref("my-gateway")
            .hostname("example.com")
            .rule(
                RuleBuilder::new()
                    .path_prefix("/api")
                    .backend("api-service", 8080)
                    .build(),
            )
            .build();

        assert_eq!(route.metadata.name, Some("test-route".to_string()));
        assert_eq!(route.spec.hostnames, vec!["example.com"]);
    }

    #[test]
    fn test_rule_builder() {
        let rule = RuleBuilder::new()
            .path_prefix("/v1")
            .backend_with_weight("svc-a", 80, 90)
            .backend_with_weight("svc-b", 80, 10)
            .build();

        assert_eq!(rule.backend_refs.len(), 2);
        assert_eq!(rule.backend_refs[0].weight, Some(90));
    }
}
