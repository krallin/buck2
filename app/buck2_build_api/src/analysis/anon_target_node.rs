/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use allocative::Allocative;
use buck2_core::collections::sorted_map::SortedMap;
use buck2_core::configuration::pair::ConfigurationNoExec;
use buck2_core::configuration::ConfigurationData;
use buck2_core::fs::paths::forward_rel_path::ForwardRelativePath;
use buck2_core::fs::project_rel_path::ProjectRelativePath;
use buck2_core::fs::project_rel_path::ProjectRelativePathBuf;
use buck2_core::target::label::ConfiguredTargetLabel;
use buck2_core::target::label::TargetLabel;
use buck2_data::ToProtoMessage;
use buck2_execute::base_deferred_key_dyn::BaseDeferredKeyDynImpl;
use buck2_node::attrs::configured_attr::ConfiguredAttr;
use buck2_node::rule_type::StarlarkRuleType;
use gazebo::cmp::PartialEqAny;

#[derive(Hash, Eq, PartialEq, Clone, Debug, Allocative)]
pub struct AnonTarget {
    /// Not necessarily a "real" target label that actually exists, but could be.
    name: TargetLabel,
    /// The type of the rule we are running.
    rule_type: Arc<StarlarkRuleType>,
    /// The attributes the target was defined with.
    /// We use a sorted map since we want to iterate in a defined order.
    attrs: SortedMap<String, ConfiguredAttr>,
    /// The hash of the `rule_type` and `attrs`
    hash: String,
    /// The execution configuration - same as the parent.
    exec_cfg: ConfigurationNoExec,
}

impl fmt::Display for AnonTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (anon: {}) ({})", self.name, self.hash, self.exec_cfg)
    }
}

impl ToProtoMessage for AnonTarget {
    type Message = buck2_data::AnonTarget;

    fn as_proto(&self) -> Self::Message {
        buck2_data::AnonTarget {
            name: Some(self.name.as_proto()),
            execution_configuration: Some(self.exec_cfg.cfg().as_proto()),
            hash: self.hash.clone(),
        }
    }
}

impl AnonTarget {
    fn mk_hash(rule_type: &StarlarkRuleType, attrs: &SortedMap<String, ConfiguredAttr>) -> String {
        // This is the same hasher as we use for Configuration, so is probably fine.
        // But quite possibly should be a crypto hasher in future.
        let mut hasher = DefaultHasher::new();
        rule_type.hash(&mut hasher);
        attrs.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn new(
        rule_type: Arc<StarlarkRuleType>,
        name: TargetLabel,
        attrs: SortedMap<String, ConfiguredAttr>,
        exec_cfg: ConfigurationNoExec,
    ) -> Self {
        let hash = Self::mk_hash(&rule_type, &attrs);
        Self {
            name,
            rule_type,
            attrs,
            hash,
            exec_cfg,
        }
    }

    pub fn rule_type(&self) -> &Arc<StarlarkRuleType> {
        &self.rule_type
    }

    pub fn name(&self) -> &TargetLabel {
        &self.name
    }

    pub fn attrs(&self) -> &SortedMap<String, ConfiguredAttr> {
        &self.attrs
    }

    pub fn rule_type_attrs_hash(&self) -> &str {
        &self.hash
    }

    pub fn exec_cfg(&self) -> &ConfigurationNoExec {
        &self.exec_cfg
    }

    pub fn configured_label(&self) -> ConfiguredTargetLabel {
        // We need a configured label, but we don't have a real configuration (because it doesn't make sense),
        // so create a dummy version
        self.name().configure(ConfigurationData::unspecified())
    }
}

impl BaseDeferredKeyDynImpl for AnonTarget {
    fn eq_token(&self) -> PartialEqAny {
        PartialEqAny::new(self)
    }

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(self, &mut hasher);
        hasher.finish()
    }

    fn make_hashed_path(
        &self,
        base: &ProjectRelativePath,
        prefix: &ForwardRelativePath,
        action_key: Option<&str>,
        path: &ForwardRelativePath,
    ) -> ProjectRelativePathBuf {
        let cell_relative_path = self.name().pkg().cell_relative_path().as_str();

        // It is performance critical that we use slices and allocate via `join` instead of
        // repeated calls to `join` on the path object because `join` allocates on each call,
        // which has a significant impact.
        let parts = [
            base.as_str(),
            "/",
            prefix.as_str(),
            "-anon/",
            self.name().pkg().cell_name().as_str(),
            "/",
            self.exec_cfg().cfg().output_hash(),
            cell_relative_path,
            if cell_relative_path.is_empty() {
                ""
            } else {
                "/"
            },
            self.rule_type_attrs_hash(),
            "/__",
            self.name().name().as_str(),
            "__",
            action_key.unwrap_or_default(),
            if action_key.is_none() { "" } else { "__" },
            "/",
            path.as_str(),
        ];

        ProjectRelativePathBuf::unchecked_new(parts.concat())
    }
}
