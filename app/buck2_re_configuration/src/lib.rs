/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use allocative::Allocative;
use buck2_common::legacy_configs::LegacyBuckConfig;

static BUCK2_RE_CLIENT_CFG_SECTION: &str = "buck2_re_client";

/// Metadata that doesn't change between executions
#[derive(Clone, Debug, Default, Allocative)]
pub struct RemoteExecutionStaticMetadata {
    pub cas_address: Option<String>,
    pub cas_connection_count: i32,
    pub action_cache_address: Option<String>,
    pub action_cache_connection_count: i32,
    pub engine_address: Option<String>,
    pub engine_connection_count: i32,

    pub verbose_logging: bool,

    pub use_manifold_rich_client: bool,
    pub use_zippy_rich_client: bool,
    pub use_p2p: bool,

    pub cas_thread_count: i32,

    pub rich_client_channels_per_blob: Option<i32>,
    pub rich_client_attempt_timeout_ms: Option<i32>,
    pub rich_client_retries_count: Option<i32>,
    pub force_enable_deduplicate_find_missing: Option<bool>,

    pub features_config_path: Option<String>,
}

impl RemoteExecutionStaticMetadata {
    pub fn from_legacy_config(legacy_config: &LegacyBuckConfig) -> anyhow::Result<Self> {
        Ok(Self {
            cas_address: legacy_config.parse(BUCK2_RE_CLIENT_CFG_SECTION, "cas_address")?,
            cas_connection_count: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "cas_connection_count")?
                .unwrap_or(16),
            action_cache_address: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "action_cache_address")?,
            action_cache_connection_count: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "action_cache_connection_count")?
                .unwrap_or(4),
            engine_address: legacy_config.parse(BUCK2_RE_CLIENT_CFG_SECTION, "engine_address")?,
            engine_connection_count: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "engine_connection_count")?
                .unwrap_or(4),
            verbose_logging: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "verbose_logging")?
                .unwrap_or(false),
            cas_thread_count: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "cas_thread_count")?
                .unwrap_or(4),
            use_manifold_rich_client: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "use_manifold_rich_client_new")?
                .unwrap_or(true),
            use_zippy_rich_client: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "use_zippy_rich_client")?
                .unwrap_or(false),
            use_p2p: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "use_p2p")?
                .unwrap_or(false),
            rich_client_channels_per_blob: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "rich_client_channels_per_blob")?,
            rich_client_attempt_timeout_ms: legacy_config.parse(
                BUCK2_RE_CLIENT_CFG_SECTION,
                "rich_client_attempt_timeout_ms",
            )?,
            rich_client_retries_count: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "rich_client_retries_count")?,
            force_enable_deduplicate_find_missing: legacy_config.parse(
                BUCK2_RE_CLIENT_CFG_SECTION,
                "force_enable_deduplicate_find_missing",
            )?,
            features_config_path: legacy_config
                .parse(BUCK2_RE_CLIENT_CFG_SECTION, "features_config_path")?,
        })
    }
}