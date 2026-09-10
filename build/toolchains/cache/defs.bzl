# Cache-only execution platform for the NativeLink CAS warm substrate (ADR-0560,
# consuming the ADR-0556 classification; idea basis docs/ideas/nativelink-remote-cache-first.md).
#
# Mirrors prelude//platforms:default exactly (same host cpu/os constraints, same
# exec-platform marker, local execution ONLY) and adds the two cache-first executor
# knobs the prelude hardcodes off: `remote_cache_enabled` + `allow_cache_uploads`.
# Remote *execution* stays False until the ADR-0525 D3 RE phase flips it in its own
# reviewed change.
#
# Dark-by-default invariant: the root .buckconfig never selects this platform and
# never sets the [cache] section. Only the opt-in CI overlays
# (infra/ci/buckconfig/warm-cache-{rw,ro}.buckconfig) select it and set the knobs,
# so every build that does not explicitly pass an overlay is bit-identical to today
# (the conformance gate in ci/facade/build-cache-policy asserts the root config stays
# clean; it moved there from cloud/cloud-ci/gates/ in the ADR-0562 reorg).

load("@prelude//cfg/exec_platform:marker.bzl", "get_exec_platform_marker")

def _cache_execution_platform_impl(ctx: AnalysisContext) -> list[Provider]:
    constraints = dict()
    constraints.update(ctx.attrs.cpu_configuration[ConfigurationInfo].constraints)
    constraints.update(ctx.attrs.os_configuration[ConfigurationInfo].constraints)
    cfg = ConfigurationInfo(constraints = constraints, values = {})

    name = ctx.label.raw_target()
    platform = ExecutionPlatformInfo(
        label = name,
        configuration = cfg,
        executor_config = CommandExecutorConfig(
            local_enabled = True,
            # Cache-first MVP (ADR-0556 D3 stage 3): the scheduler tier is not
            # deployed; remote execution is a later, separate door (ADR-0525 D3).
            remote_enabled = False,
            remote_cache_enabled = ctx.attrs.remote_cache_enabled,
            allow_cache_uploads = ctx.attrs.allow_cache_uploads,
            use_windows_path_separators = ctx.attrs.use_windows_path_separators,
        ),
    )

    return [
        DefaultInfo(),
        platform,
        PlatformInfo(label = str(name), configuration = cfg),
        ExecutionPlatformRegistrationInfo(
            platforms = [platform],
            exec_marker_constraint = get_exec_platform_marker(),
        ),
    ]

cache_execution_platform = rule(
    impl = _cache_execution_platform_impl,
    attrs = {
        "allow_cache_uploads": attrs.bool(default = False),
        "cpu_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "os_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "remote_cache_enabled": attrs.bool(default = False),
        "use_windows_path_separators": attrs.bool(default = False),
    },
)
