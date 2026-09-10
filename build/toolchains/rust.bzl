# Hermetic Rust toolchain: the compiler is a downloaded, hash-pinned build input.
#
# prelude//toolchains:rust.bzl `system_rust_toolchain` hardcodes the tools as bare
# argv names (`rustc`, `rustdoc`, `clippy-driver`) and exposes no attr that can point
# at a binary, so rustc is resolved from PATH inside every action. On a runner PATH
# is a rustup shim, so hundreds of concurrent actions each drive rustup against one
# shared ~/.rustup, racing on the component install. That is what reds the weekly
# smoke. Here rustc, rustdoc and clippy-driver are artifacts instead, so they land in
# the action key and no rustc action consults PATH or rustup.

load("@prelude//rust:rust_toolchain.bzl", "PanicRuntime", "RustToolchainInfo")

# `.buckconfig` pulls in rust-toolchain.toml with `<file:rust-toolchain.toml>`, which makes
# `[toolchain] channel` readable as buckconfig, so the channel is read here, not declared.
def _channel() -> str:
    raw = read_root_config("toolchain", "channel", "")
    return raw.replace("\"", "").replace("'", "").strip()

RUST_CHANNEL = _channel()

_NIGHTLY_PREFIX = "nightly-"

_DIST_ROOT = "https://static.rust-lang.org/dist/"

# rustc carries bin/ + lib/; rust-std carries lib/rustlib/<triple>/lib; clippy carries
# bin/clippy-driver. Unpacked over one another at --strip-components=2 they reproduce
# the directory shape rustup installs, so rustc finds its own sysroot with no
# --sysroot flag and the prelude's clippy wrapper resolves the same one.
_COMPONENTS = ["rustc", "rust-std", "clippy"]

# Two dist layouts, both keyed off the channel string rust-toolchain.toml already uses:
#   stable "<channel>"        -> dist/<component>-<channel>-<triple>.tar.gz
#   dated  "nightly-<date>"   -> dist/<date>/<component>-nightly-<triple>.tar.gz
def _dist_url(component: str, triple: str) -> str:
    if RUST_CHANNEL.startswith(_NIGHTLY_PREFIX):
        date = RUST_CHANNEL[len(_NIGHTLY_PREFIX):]
        return "{}{}/{}-nightly-{}.tar.gz".format(_DIST_ROOT, date, component, triple)
    return "{}{}-{}-{}.tar.gz".format(_DIST_ROOT, component, RUST_CHANNEL, triple)

# sha256 per channel/triple/component, from https://static.rust-lang.org/<path>.sha256.
# A bump in rust-toolchain.toml needs a matching entry here; a channel with no entry
# fails analysis in _digests_for below rather than drifting silently.
_DIGESTS = {
    "1.98.0": {
        "aarch64-apple-darwin": {
            "clippy": "41f050d3b10488bd4910c5acbf063df2fb0d80800bf1f89a8fbd4f91df27bccf",
            "rust-std": "49dc82ff5b6fb033baa6e85635ed9cf07a27e5297a22314808ab4cf65a81335a",
            "rustc": "7b39cbbb11995584741b5821511b8706ab8ecab3bc7e12b11fb1ab50cab9d079",
        },
        "x86_64-unknown-linux-gnu": {
            "clippy": "22a5d1ed834c3b94cd5f5b0a7d1c804ca1124b7c1fadaf25583e452e6426ebd7",
            "rust-std": "8aa6405356392ce50160d1b286e86091c5e14adae3061115699c84ed4394d546",
            "rustc": "18ed6559de1b8ea6b77474ea86992b9a507d3a3d134d9ee017d30cf3f406e3ee",
        },
    },
}

# Hosts this repo builds on. An unlisted host gets "no condition matched" at
# configuration time, which is deliberate: silently falling back to PATH is the
# failure mode this file exists to remove.
RUST_TARGET_TRIPLE = select({
    "prelude//os:linux": select({
        "prelude//cpu:x86_64": "x86_64-unknown-linux-gnu",
    }),
    "prelude//os:macos": select({
        "prelude//cpu:arm64": "aarch64-apple-darwin",
    }),
})

def _digests_for(triple: str) -> dict[str, str]:
    if not RUST_CHANNEL:
        fail("no `[toolchain] channel` in buckconfig; .buckconfig must keep `<file:rust-toolchain.toml>`")
    by_triple = _DIGESTS.get(RUST_CHANNEL)
    if by_triple == None:
        fail("rust-toolchain.toml pins channel '{}', which has no _DIGESTS entry in build/toolchains/rust.bzl. Add one: sha256 of {} at {}<...>.sha256".format(
            RUST_CHANNEL,
            _COMPONENTS,
            _DIST_ROOT,
        ))
    digests = by_triple.get(triple)
    if digests == None:
        fail("channel '{}' has no pinned digests for triple '{}'".format(RUST_CHANNEL, triple))
    return digests

def _rust_distribution_impl(ctx: AnalysisContext) -> list[Provider]:
    triple = ctx.attrs.rustc_target_triple
    digests = _digests_for(triple)

    sysroot = ctx.actions.declare_output("sysroot", dir = True)
    commands = [cmd_args(sysroot.as_output(), format = "mkdir -p {}")]
    archives = []
    for component in _COMPONENTS:
        archive = ctx.actions.declare_output("{}.tar.gz".format(component))
        ctx.actions.download_file(
            archive.as_output(),
            _dist_url(component, triple),
            sha256 = digests[component],
        )
        archives.append(archive)
        commands.append(cmd_args(
            "tar",
            "-xzf",
            archive,
            "-C",
            sysroot.as_output(),
            "--strip-components=2",
            delimiter = " ",
        ))

    script, _ = ctx.actions.write(
        "unpack_rust.sh",
        cmd_args("set -eu", commands),
        is_executable = True,
        allow_args = True,
    )
    ctx.actions.run(
        cmd_args(["/bin/sh", script], hidden = archives + [sysroot.as_output()]),
        category = "rust_distribution",
        identifier = triple,
    )
    return [DefaultInfo(default_output = sysroot)]

rust_distribution = rule(
    impl = _rust_distribution_impl,
    attrs = {
        "rustc_target_triple": attrs.string(default = RUST_TARGET_TRIPLE),
    },
)

def _tool(sysroot: Artifact, relative_path: str) -> RunInfo:
    return RunInfo(args = cmd_args(sysroot.project(relative_path), hidden = [sysroot]))

def _hermetic_rust_toolchain_impl(ctx: AnalysisContext) -> list[Provider]:
    sysroot = ctx.attrs.distribution[DefaultInfo].default_outputs[0]
    return [
        DefaultInfo(),
        RustToolchainInfo(
            allow_lints = ctx.attrs.allow_lints,
            clippy_driver = _tool(sysroot, "bin/clippy-driver"),
            clippy_toml = ctx.attrs.clippy_toml[DefaultInfo].default_outputs[0] if ctx.attrs.clippy_toml else None,
            compiler = _tool(sysroot, "bin/rustc"),
            default_edition = ctx.attrs.default_edition,
            panic_runtime = PanicRuntime("unwind"),
            deny_lints = ctx.attrs.deny_lints,
            doctests = ctx.attrs.doctests,
            nightly_features = ctx.attrs.nightly_features,
            report_unused_deps = ctx.attrs.report_unused_deps,
            rustc_binary_flags = ctx.attrs.rustc_binary_flags,
            rustc_flags = ctx.attrs.rustc_flags,
            rustc_target_triple = ctx.attrs.rustc_target_triple,
            rustc_test_flags = ctx.attrs.rustc_test_flags,
            rustdoc = _tool(sysroot, "bin/rustdoc"),
            rustdoc_flags = ctx.attrs.rustdoc_flags,
            warn_lints = ctx.attrs.warn_lints,
        ),
    ]

# Attr surface deliberately mirrors prelude `system_rust_toolchain`, plus `distribution`,
# so swapping the two is a one-line change in BUCK. `.buckconfig` takes the prelude bundled,
# so "the prelude" is whichever buck2 runs: these defaults mirror the buck2 pinned in
# .github/workflows/buck2-weekly-smoke.yml and need re-checking when that pin moves.
# `nightly_features` already defaults the other way in a later prelude.
hermetic_rust_toolchain = rule(
    impl = _hermetic_rust_toolchain_impl,
    attrs = {
        "allow_lints": attrs.list(attrs.string(), default = []),
        "clippy_toml": attrs.option(attrs.dep(providers = [DefaultInfo]), default = None),
        "default_edition": attrs.option(attrs.string(), default = None),
        "deny_lints": attrs.list(attrs.string(), default = []),
        "distribution": attrs.dep(providers = [DefaultInfo]),
        "doctests": attrs.bool(default = False),
        "nightly_features": attrs.bool(default = False),
        "report_unused_deps": attrs.bool(default = False),
        "rustc_binary_flags": attrs.list(attrs.arg(), default = []),
        "rustc_flags": attrs.list(attrs.arg(), default = []),
        "rustc_target_triple": attrs.string(default = RUST_TARGET_TRIPLE),
        "rustc_test_flags": attrs.list(attrs.arg(), default = []),
        "rustdoc_flags": attrs.list(attrs.arg(), default = []),
        "warn_lints": attrs.list(attrs.string(), default = []),
    },
    is_toolchain_rule = True,
)
