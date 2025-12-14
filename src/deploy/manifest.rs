//! Kubernetes manifest generation for Gateway API resources
//!
//! Generates Gateway, HTTPRoute, and related resources for testing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::GatewayImpl;

/// Gateway resource manifest
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayManifest {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: GatewaySpec,
}

/// HTTPRoute resource manifest
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRouteManifest {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: HttpRouteSpec,
}

/// Kubernetes metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

/// Gateway spec
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySpec {
    pub gateway_class_name: String,
    pub listeners: Vec<Listener>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<GatewayAddress>>,
}

/// Gateway listener
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listener {
    pub name: String,
    pub port: u16,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<ListenerTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_routes: Option<AllowedRoutes>,
}

/// Listener TLS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenerTls {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_refs: Option<Vec<SecretRef>>,
}

/// Secret reference
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

/// Allowed routes configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedRoutes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<RouteNamespaces>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<RouteGroupKind>>,
}

/// Route namespaces selector
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteNamespaces {
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<LabelSelector>,
}

/// Label selector
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub match_labels: BTreeMap<String, String>,
}

/// Route group kind
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteGroupKind {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub kind: String,
}

/// Gateway address
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAddress {
    #[serde(rename = "type")]
    pub address_type: String,
    pub value: String,
}

/// HTTPRoute spec
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRouteSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_refs: Option<Vec<ParentRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<HttpRouteRule>>,
}

/// Parent reference
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

/// HTTPRoute rule
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<Vec<HttpRouteMatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<HttpRouteFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_refs: Option<Vec<BackendRef>>,
}

/// HTTPRoute match
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRouteMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HeaderMatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<Vec<QueryParamMatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// Path match
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathMatch {
    #[serde(rename = "type")]
    pub match_type: String,
    pub value: String,
}

/// Header match
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderMatch {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,
    pub name: String,
    pub value: String,
}

/// Query parameter match
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParamMatch {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,
    pub name: String,
    pub value: String,
}

/// HTTPRoute filter
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRouteFilter {
    #[serde(rename = "type")]
    pub filter_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_header_modifier: Option<HeaderModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_header_modifier: Option<HeaderModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_redirect: Option<RequestRedirect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_rewrite: Option<UrlRewrite>,
}

/// Header modifier
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderModifier {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set: Vec<HeaderValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<HeaderValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
}

/// Header name-value pair
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderValue {
    pub name: String,
    pub value: String,
}

/// Request redirect
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestRedirect {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

/// URL rewrite
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlRewrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathRewrite>,
}

/// Path rewrite
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathRewrite {
    #[serde(rename = "type")]
    pub rewrite_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_prefix_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_full_path: Option<String>,
}

/// Backend reference
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
}

/// Manifest generator
pub struct ManifestGenerator {
    namespace: String,
    gateway_class: String,
}

impl ManifestGenerator {
    /// Create a new manifest generator
    pub fn new(gateway_impl: GatewayImpl) -> Self {
        Self {
            namespace: "default".to_string(),
            gateway_class: gateway_impl.gateway_class().to_string(),
        }
    }

    /// Set namespace
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = ns.into();
        self
    }

    /// Generate a basic Gateway resource
    pub fn gateway(&self, name: &str) -> GatewayManifest {
        GatewayManifest {
            api_version: "gateway.networking.k8s.io/v1".to_string(),
            kind: "Gateway".to_string(),
            metadata: Metadata {
                name: name.to_string(),
                namespace: Some(self.namespace.clone()),
                labels: self.default_labels(),
                annotations: BTreeMap::new(),
            },
            spec: GatewaySpec {
                gateway_class_name: self.gateway_class.clone(),
                listeners: vec![
                    Listener {
                        name: "http".to_string(),
                        port: 80,
                        protocol: "HTTP".to_string(),
                        hostname: None,
                        tls: None,
                        allowed_routes: Some(AllowedRoutes {
                            namespaces: Some(RouteNamespaces {
                                from: "All".to_string(),
                                selector: None,
                            }),
                            kinds: None,
                        }),
                    },
                ],
                addresses: None,
            },
        }
    }

    /// Generate Gateway with HTTPS listener
    pub fn gateway_with_tls(&self, name: &str, secret_name: &str) -> GatewayManifest {
        let mut gateway = self.gateway(name);
        gateway.spec.listeners.push(Listener {
            name: "https".to_string(),
            port: 443,
            protocol: "HTTPS".to_string(),
            hostname: None,
            tls: Some(ListenerTls {
                mode: "Terminate".to_string(),
                certificate_refs: Some(vec![SecretRef {
                    name: secret_name.to_string(),
                    namespace: None,
                }]),
            }),
            allowed_routes: Some(AllowedRoutes {
                namespaces: Some(RouteNamespaces {
                    from: "All".to_string(),
                    selector: None,
                }),
                kinds: None,
            }),
        });
        gateway
    }

    /// Generate a basic HTTPRoute
    pub fn http_route(&self, name: &str, gateway_name: &str) -> HttpRouteManifest {
        HttpRouteManifest {
            api_version: "gateway.networking.k8s.io/v1".to_string(),
            kind: "HTTPRoute".to_string(),
            metadata: Metadata {
                name: name.to_string(),
                namespace: Some(self.namespace.clone()),
                labels: self.default_labels(),
                annotations: BTreeMap::new(),
            },
            spec: HttpRouteSpec {
                parent_refs: Some(vec![ParentRef {
                    name: gateway_name.to_string(),
                    namespace: Some(self.namespace.clone()),
                    section_name: None,
                    port: None,
                }]),
                hostnames: None,
                rules: None,
            },
        }
    }

    /// Generate HTTPRoute with path routing
    pub fn http_route_path(
        &self,
        name: &str,
        gateway_name: &str,
        path: &str,
        backend: &str,
        port: u16,
    ) -> HttpRouteManifest {
        let mut route = self.http_route(name, gateway_name);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: Some(vec![HttpRouteMatch {
                path: Some(PathMatch {
                    match_type: "PathPrefix".to_string(),
                    value: path.to_string(),
                }),
                headers: None,
                query_params: None,
                method: None,
            }]),
            filters: None,
            backend_refs: Some(vec![BackendRef {
                name: backend.to_string(),
                namespace: None,
                port: Some(port),
                weight: None,
            }]),
        }]);
        route
    }

    /// Generate HTTPRoute with host routing
    pub fn http_route_host(
        &self,
        name: &str,
        gateway_name: &str,
        hostname: &str,
        backend: &str,
        port: u16,
    ) -> HttpRouteManifest {
        let mut route = self.http_route(name, gateway_name);
        route.spec.hostnames = Some(vec![hostname.to_string()]);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: None,
            filters: None,
            backend_refs: Some(vec![BackendRef {
                name: backend.to_string(),
                namespace: None,
                port: Some(port),
                weight: None,
            }]),
        }]);
        route
    }

    /// Generate HTTPRoute with header routing
    pub fn http_route_header(
        &self,
        name: &str,
        gateway_name: &str,
        header_name: &str,
        header_value: &str,
        backend: &str,
        port: u16,
    ) -> HttpRouteManifest {
        let mut route = self.http_route(name, gateway_name);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: Some(vec![HttpRouteMatch {
                path: None,
                headers: Some(vec![HeaderMatch {
                    match_type: Some("Exact".to_string()),
                    name: header_name.to_string(),
                    value: header_value.to_string(),
                }]),
                query_params: None,
                method: None,
            }]),
            filters: None,
            backend_refs: Some(vec![BackendRef {
                name: backend.to_string(),
                namespace: None,
                port: Some(port),
                weight: None,
            }]),
        }]);
        route
    }

    /// Generate HTTPRoute with traffic split (canary)
    pub fn http_route_canary(
        &self,
        name: &str,
        gateway_name: &str,
        stable_backend: &str,
        canary_backend: &str,
        canary_weight: u32,
        port: u16,
    ) -> HttpRouteManifest {
        let stable_weight = 100 - canary_weight;
        let mut route = self.http_route(name, gateway_name);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: None,
            filters: None,
            backend_refs: Some(vec![
                BackendRef {
                    name: stable_backend.to_string(),
                    namespace: None,
                    port: Some(port),
                    weight: Some(stable_weight),
                },
                BackendRef {
                    name: canary_backend.to_string(),
                    namespace: None,
                    port: Some(port),
                    weight: Some(canary_weight),
                },
            ]),
        }]);
        route
    }

    /// Generate HTTPRoute with HTTPS redirect
    pub fn http_route_redirect_https(
        &self,
        name: &str,
        gateway_name: &str,
    ) -> HttpRouteManifest {
        let mut route = self.http_route(name, gateway_name);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: None,
            filters: Some(vec![HttpRouteFilter {
                filter_type: "RequestRedirect".to_string(),
                request_header_modifier: None,
                response_header_modifier: None,
                request_redirect: Some(RequestRedirect {
                    scheme: Some("https".to_string()),
                    hostname: None,
                    port: Some(443),
                    status_code: Some(301),
                }),
                url_rewrite: None,
            }]),
            backend_refs: None,
        }]);
        route
    }

    /// Generate HTTPRoute with URL rewrite
    pub fn http_route_rewrite(
        &self,
        name: &str,
        gateway_name: &str,
        path_prefix: &str,
        replace_with: &str,
        backend: &str,
        port: u16,
    ) -> HttpRouteManifest {
        let mut route = self.http_route(name, gateway_name);
        route.spec.rules = Some(vec![HttpRouteRule {
            matches: Some(vec![HttpRouteMatch {
                path: Some(PathMatch {
                    match_type: "PathPrefix".to_string(),
                    value: path_prefix.to_string(),
                }),
                headers: None,
                query_params: None,
                method: None,
            }]),
            filters: Some(vec![HttpRouteFilter {
                filter_type: "URLRewrite".to_string(),
                request_header_modifier: None,
                response_header_modifier: None,
                request_redirect: None,
                url_rewrite: Some(UrlRewrite {
                    hostname: None,
                    path: Some(PathRewrite {
                        rewrite_type: "ReplacePrefixMatch".to_string(),
                        replace_prefix_match: Some(replace_with.to_string()),
                        replace_full_path: None,
                    }),
                }),
            }]),
            backend_refs: Some(vec![BackendRef {
                name: backend.to_string(),
                namespace: None,
                port: Some(port),
                weight: None,
            }]),
        }]);
        route
    }

    /// Convert manifest to YAML
    pub fn to_yaml<T: Serialize>(manifest: &T) -> String {
        serde_yaml::to_string(manifest).unwrap_or_default()
    }

    /// Convert manifest to JSON
    pub fn to_json<T: Serialize>(manifest: &T) -> String {
        serde_json::to_string_pretty(manifest).unwrap_or_default()
    }

    fn default_labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("app.kubernetes.io/managed-by".to_string(), "gateway-poc".to_string());
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_manifest() {
        let gen = ManifestGenerator::new(GatewayImpl::Nginx);
        let gateway = gen.gateway("test-gateway");

        assert_eq!(gateway.metadata.name, "test-gateway");
        assert_eq!(gateway.spec.gateway_class_name, "nginx");
        assert_eq!(gateway.spec.listeners.len(), 1);
    }

    #[test]
    fn test_http_route_manifest() {
        let gen = ManifestGenerator::new(GatewayImpl::Envoy);
        let route = gen.http_route_path("test-route", "test-gateway", "/api", "backend-svc", 8080);

        assert_eq!(route.metadata.name, "test-route");
        assert!(route.spec.rules.is_some());
    }

    #[test]
    fn test_canary_route() {
        let gen = ManifestGenerator::new(GatewayImpl::Istio);
        let route = gen.http_route_canary("canary-route", "gateway", "stable", "canary", 20, 80);

        let rules = route.spec.rules.unwrap();
        let backends = rules[0].backend_refs.as_ref().unwrap();
        assert_eq!(backends.len(), 2);
        assert_eq!(backends[0].weight, Some(80));
        assert_eq!(backends[1].weight, Some(20));
    }

    #[test]
    fn test_to_yaml() {
        let gen = ManifestGenerator::new(GatewayImpl::Nginx);
        let gateway = gen.gateway("test");
        let yaml = ManifestGenerator::to_yaml(&gateway);

        assert!(yaml.contains("apiVersion:"));
        assert!(yaml.contains("kind: Gateway"));
    }
}
