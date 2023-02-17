// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;

impl KaniSession {
    pub fn synthesize_loop_contracts(&self, output: &Path) -> Result<()> {
        let args: Vec<OsString> = vec![
            "--loop-contracts-no-unwind".into(),
            output.to_owned().into_os_string(), // input
            output.to_owned().into_os_string(), // output
        ];

        self.call_goto_synthesizer(args)?;

        Ok(())
    }

    /// Non-public helper function to actually do the run of goto-synthesizer
    fn call_goto_synthesizer(&self, args: Vec<OsString>) -> Result<()> {
        let mut cmd = Command::new("goto-synthesizer");
        cmd.args(args);

        self.run_suppress(cmd)
    }
}
