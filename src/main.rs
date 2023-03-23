use anyhow::{bail, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::*;
use validator::Validate;

use k8s_openapi::{apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition, api::core::v1::Pod};
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
    core::crd::CustomResourceExt,
    runtime::{reflector, watcher, WatchStreamExt},
    Client, CustomResource,
};
use futures::{StreamExt, TryStreamExt};

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(group = "pk.dev", version = "v1", kind = "CleanerRs", namespaced)]
pub struct CleanerRsSpec {
    ttl: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    // delete old versions
    // info!("Deleting old CRD");
    // let dp = DeleteParams::default();
    // let _ = crds.delete("cleanerrses.pk.dev", &dp).await;
    // sleep(Duration::from_secs(2)).await;

    // create new crd
    let crd = CleanerRs::crd();
    info!("Creating CRD: {}", serde_json::to_string_pretty(&crd)?);
    let pp = PostParams::default();
    match crds.create(&pp, &crd).await {
        Ok(_) => info!("Created CRD"),
        Err(e) => {
            if e.to_string().contains("already exists") {
                info!("CRD already exists");
            } else {
                bail!(e);
            }
        }
    }

    sleep(Duration::from_secs(1)).await;

    // get cleaners
    let cleaners: Api<CleanerRs> = Api::all(client);
    watcher(cleaners, ListParams::default())
        .applied_objects()
        .try_for_each(|p| async move {
            info!("item: {}, with ttl: {}", p.name_any(), p.spec.ttl);
            Ok(())
        })
        .await?;

    Ok(())
}
