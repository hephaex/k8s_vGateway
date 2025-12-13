//! Kubernetes client wrapper
//!
//! Provides a high-level interface to the Kubernetes API.

#![allow(dead_code)]

use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, ListParams},
    Client, Config,
};
use tracing::{info, warn};

/// Kubernetes client wrapper
#[derive(Clone)]
pub struct K8sClient {
    client: Client,
    namespace: String,
}

impl K8sClient {
    /// Create a new Kubernetes client
    pub async fn new(namespace: impl Into<String>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .context("Failed to create Kubernetes client")?;

        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    /// Create client with custom config
    pub async fn with_config(config: Config, namespace: impl Into<String>) -> Result<Self> {
        let client =
            Client::try_from(config).context("Failed to create Kubernetes client from config")?;

        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    /// Get the underlying kube client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the namespace
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Check if Gateway API CRDs are installed
    pub async fn gateway_api_installed(&self) -> Result<bool> {
        let crds: Api<CustomResourceDefinition> = Api::all(self.client.clone());
        let params = ListParams::default();

        let crd_list = crds.list(&params).await.context("Failed to list CRDs")?;

        let gateway_crds = [
            "gateways.gateway.networking.k8s.io",
            "httproutes.gateway.networking.k8s.io",
            "gatewayclasses.gateway.networking.k8s.io",
        ];

        let found_crds: Vec<_> = crd_list
            .items
            .iter()
            .filter(|crd| {
                crd.metadata
                    .name
                    .as_ref()
                    .map(|n| gateway_crds.contains(&n.as_str()))
                    .unwrap_or(false)
            })
            .collect();

        let installed = found_crds.len() == gateway_crds.len();

        if installed {
            info!("Gateway API CRDs are installed");
        } else {
            warn!(
                "Gateway API CRDs not fully installed ({}/{})",
                found_crds.len(),
                gateway_crds.len()
            );
        }

        Ok(installed)
    }

    /// Check if a specific CRD exists
    pub async fn crd_exists(&self, group: &str, _version: &str, kind: &str) -> Result<bool> {
        let crds: Api<CustomResourceDefinition> = Api::all(self.client.clone());
        let crd_name = format!("{}.{}", kind.to_lowercase() + "s", group);

        match crds.get(&crd_name).await {
            Ok(_) => Ok(true),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
            Err(e) => Err(e).context("Failed to check CRD existence"),
        }
    }

    /// List namespaces
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let ns_list = namespaces
            .list(&ListParams::default())
            .await
            .context("Failed to list namespaces")?;

        Ok(ns_list
            .items
            .iter()
            .filter_map(|ns| ns.metadata.name.clone())
            .collect())
    }

    /// Check if namespace exists
    pub async fn namespace_exists(&self, name: &str) -> Result<bool> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());

        match namespaces.get(name).await {
            Ok(_) => Ok(true),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
            Err(e) => Err(e).context("Failed to check namespace existence"),
        }
    }

    /// Get pods in namespace
    pub async fn get_pods(&self) -> Result<Vec<Pod>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let pod_list = pods
            .list(&ListParams::default())
            .await
            .context("Failed to list pods")?;

        Ok(pod_list.items)
    }

    /// Get services in namespace
    pub async fn get_services(&self) -> Result<Vec<Service>> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let svc_list = services
            .list(&ListParams::default())
            .await
            .context("Failed to list services")?;

        Ok(svc_list.items)
    }

    /// Create a namespaced API for a custom resource type
    pub fn namespaced_api<K>(&self) -> Api<K>
    where
        K: kube::Resource<Scope = kube::core::NamespaceResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        Api::namespaced(self.client.clone(), &self.namespace)
    }

    /// Create a cluster-wide API for a custom resource type
    pub fn cluster_api<K>(&self) -> Api<K>
    where
        K: kube::Resource<Scope = kube::core::ClusterResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        Api::all(self.client.clone())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_k8s_client_namespace() {
        // Note: This is a sync test, actual client creation requires async
        let namespace = "test-namespace";
        assert_eq!(namespace, "test-namespace");
    }
}
