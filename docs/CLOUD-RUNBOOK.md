# Cloud continuation

Work exclusively on next. On resume read PROGRESS.md and PLAN.md, inspect git
status/log/remote, fetch origin and compare local next with origin/next. Preserve
uncommitted work and divergent commits; do not force push or blindly reset.

On the prepared environment, activate tools with:

```sh
source /workspace/.cloud-setup/ununitypackage/env.sh
```

On fresh hosts use a normal Rust installation. The above path is an environment
convenience, not a runtime dependency. Dependency caches and target/ are expendable;
Cargo.lock, tests, source and docs must live in Git.

Use the cloud environment runtime skill to check current network readiness.
Preserve injected proxy and CA settings. If the sandbox cannot reach proxy:8080,
retry permitted network operations via the platform's network approval path.
Never print tokens, remove proxy settings or bypass a network denial.

At each milestone, or after about 10–15 minutes of sustained implementation:

1. Update PROGRESS.md with current goal, checks, limitations and exact next action.
2. Run relevant checks and git diff --check. Failed checks mean checkpoint only.
3. Commit relevant source/docs/tests; use `checkpoint:` for incomplete work.
4. Push with git push origin next (initially git push -u origin next).
5. Compare git rev-parse HEAD with git ls-remote origin refs/heads/next.

Checkpoint before long CI waits/builds. A local commit without a successful push
is not recoverable from a recycled worker; report any push failure immediately.
Do not commit binaries, target caches or private packages. No automatic master
merge, version tag, registry publication or release publication is required.
