# Hermetic C toolchain: clang, its resource headers and llvm-ar are downloaded,
# hash-pinned build inputs.
#
# prelude//toolchains:cxx.bzl `system_cxx_toolchain` declares `compiler`, `linker` and
# `archiver` as `attrs.option(attrs.string())` and wraps each in `RunInfo(args = [<string>])`.
# The C toolchain is therefore an argv name, never an artifact, so it is absent from the
# input merkle tree of every action that uses it: each link of a `rust_test` or
# `rust_binary`, and every cc-rs build script. Those actions are cacheable but unkeyed on
# the compiler that produced them, so a shared action cache can serve an artifact built by
# a different clang. That is the same defect the Rust toolchain had, one layer down.
#
# The prelude takes the same tools a second way — `cxx_tools_info_toolchain` reads them
# from a `CxxToolsInfo` exec_dep whose fields are `typing.Any` — and then runs the identical
# `_cxx_toolchain_from_cxx_tools_info` body. Artifact-backed `cmd_args` pass through that
# path, so the tools land in the key with no other toolchain knob changed.

load("@prelude//cxx:cxx_toolchain_types.bzl", "LinkerType")
load("@prelude//toolchains:cxx.bzl", "CxxToolsInfo")

# The one statement of the C toolchain version. Nothing else in the repo pins a clang, so
# unlike the Rust channel there is no other file to derive it from; the digests below are
# derived data that a bump must refresh.
CLANG_VERSION = "19.1.7"

_CLANG_MAJOR = CLANG_VERSION.split(".")[0]

_RELEASE_ROOT = "https://github.com/llvm/llvm-project/releases/download/llvmorg-{}/".format(CLANG_VERSION)

# sha256 of the release asset, from streaming it through sha256. An unlisted host fails
# loudly below: silently falling back to a PATH clang is the failure this file removes.
_DISTRIBUTIONS = {
    "aarch64-apple-darwin": {
        "asset": "LLVM-{}-macOS-ARM64.tar.xz",
        "sha256": "d93bf12952d89fe4ec7501c40475718b722407da6a8d651f05c995863468e570",
    },
    "x86_64-unknown-linux-gnu": {
        "asset": "LLVM-{}-Linux-X64.tar.xz",
        "sha256": "4a5ec53951a584ed36f80240f6fbf8fdd46b4cf6c7ee87cc2d5018dc37caf679",
    },
}

# Hosts this repo builds on, keyed like build/toolchains/rust.bzl's RUST_TARGET_TRIPLE so
# the two toolchains admit the same machines. Windows never reaches here: BUCK routes it to
# the prelude's MSVC tools, which is what it used before.
CLANG_HOST = select({
    "prelude//os:linux": select({
        "prelude//cpu:x86_64": "x86_64-unknown-linux-gnu",
    }),
    "prelude//os:macos": select({
        "prelude//cpu:arm64": "aarch64-apple-darwin",
    }),
})

def _distribution_for(host: str) -> dict[str, str]:
    dist = _DISTRIBUTIONS.get(host)
    if dist == None:
        fail("no pinned clang {} distribution for host '{}'; add one to _DISTRIBUTIONS in build/toolchains/cxx.bzl".format(
            CLANG_VERSION,
            host,
        ))
    return dist

def _clang_distribution_impl(ctx: AnalysisContext) -> list[Provider]:
    dist = _distribution_for(ctx.attrs.host)
    asset = dist["asset"].format(CLANG_VERSION)

    archive = ctx.actions.declare_output(asset)
    ctx.actions.download_file(
        archive.as_output(),
        _RELEASE_ROOT + asset,
        sha256 = dist["sha256"],
    )

    # Only these members are extracted: ~300 files of the release's ~9,000, and 0.5 GB of
    # its 5 GB. The rest is tools this build never invokes, and every byte of it would be
    # hashed into the input merkle tree of every action that uses the toolchain. lld is
    # kept because a cc-rs build-script link has no `-B` of its own and takes
    # `-fuse-ld=lld` from the prelude, resolving it out of clang's own bin dir; a rustc
    # link instead resolves its linker from rustc's `-B <sysroot>/.../bin/gcc-ld`, which
    # #2430 already declared. A member absent from a future release fails the unpack
    # loudly rather than silently dropping a tool.
    #
    # --strip-components=1 drops the `clang+llvm-<ver>-<host>/` prefix, so the output dir is
    # the distribution root: bin/clang finds lib/clang/<major>/include next to it, exactly as
    # an unpacked release does. clang resolves its resource dir from the real path of argv[0],
    # so the tree has to be one artifact rather than a projection of individual files.
    prefix = asset.removesuffix(".tar.xz")
    members = [
        "{}/bin/{}".format(prefix, tool)
        for tool in ["clang-" + _CLANG_MAJOR, "clang", "clang++", "llvm-ar", "lld", "ld.lld"]
    ] + ["{}/lib/clang/{}".format(prefix, _CLANG_MAJOR)]

    root = ctx.actions.declare_output("clang", dir = True)
    script, _ = ctx.actions.write(
        "unpack_clang.sh",
        cmd_args(
            "set -eu",
            cmd_args(root.as_output(), format = "mkdir -p {}"),
            cmd_args(
                "tar",
                "-xJf",
                archive,
                "-C",
                root.as_output(),
                "--strip-components=1",
                members,
                delimiter = " ",
            ),
        ),
        is_executable = True,
        allow_args = True,
    )
    ctx.actions.run(
        cmd_args(["/bin/sh", script], hidden = [archive, root.as_output()]),
        category = "clang_distribution",
        identifier = ctx.attrs.host,
    )
    return [DefaultInfo(default_output = root)]

clang_distribution = rule(
    impl = _clang_distribution_impl,
    attrs = {
        "host": attrs.string(default = CLANG_HOST),
    },
)

def _tool(root: Artifact, relative_path: str) -> cmd_args:
    return cmd_args(root.project(relative_path), hidden = [root])

def _clang_tools_impl(ctx: AnalysisContext) -> list[Provider]:
    root = ctx.attrs.distribution[DefaultInfo].default_outputs[0]
    clang = _tool(root, "bin/clang")
    return [
        DefaultInfo(),
        # Field-for-field what prelude//toolchains/cxx/clang:path_clang_tools returns, with
        # every tool an artifact instead of a bare argv name. `linker` is the C driver, not
        # clang++, matching what build/toolchains/BUCK passed before.
        CxxToolsInfo(
            compiler = clang,
            compiler_type = "clang",
            cxx_compiler = _tool(root, "bin/clang++"),
            asm_compiler = clang,
            asm_compiler_type = "clang",
            rc_compiler = None,
            cvtres_compiler = None,
            archiver = _tool(root, "bin/llvm-ar"),
            archiver_type = "gnu",
            linker = clang,
            linker_type = LinkerType(ctx.attrs.linker_type),
        ),
    ]

clang_tools = rule(
    impl = _clang_tools_impl,
    attrs = {
        "distribution": attrs.dep(providers = [DefaultInfo]),
        "linker_type": attrs.string(default = select({
            "DEFAULT": "gnu",
            "prelude//os:macos": "darwin",
        })),
    },
)
