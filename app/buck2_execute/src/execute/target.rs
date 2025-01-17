/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt::Display;

use buck2_core::category::Category;
use buck2_data::ToProtoMessage;
use derivative::Derivative;
use dupe::Dupe;

use crate::base_deferred_key_dyn::BaseDeferredKeyDyn;
use crate::path::buck_out_path::BuckOutScratchPath;

/// Indicates why we are executing a given command.
#[derive(Clone, Dupe, Derivative)]
#[derivative(Debug)]
pub struct CommandExecutionTarget<'a> {
    pub owner: BaseDeferredKeyDyn,
    pub category: &'a Category,
    pub identifier: Option<&'a str>,

    // For serialization in logging.
    #[derivative(Debug = "ignore")]
    pub action_key: &'a (dyn ToProtoMessage<Message = buck2_data::ActionKey> + Sync),
}

impl<'a> CommandExecutionTarget<'a> {
    pub fn re_action_key(&self) -> String {
        self.to_string()
    }

    pub fn re_affinity_key(&self) -> String {
        self.owner.to_string()
    }

    pub fn scratch_dir(&self) -> BuckOutScratchPath {
        BuckOutScratchPath::new(self.owner.dupe(), self.category, self.identifier).unwrap()
    }
}

impl<'a> Display for CommandExecutionTarget<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.owner, self.category)?;
        if let Some(id) = self.identifier {
            write!(f, " {}", id)?;
        }
        Ok(())
    }
}
