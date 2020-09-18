"""
@generated
cargo-raze crate workspace functions

DO NOT EDIT! Replaced on runs of cargo-raze
"""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "new_git_repository")  # buildifier: disable=load
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")  # buildifier: disable=load
load("@bazel_tools//tools/build_defs/repo:utils.bzl", "maybe")  # buildifier: disable=load

def raze_fetch_remote_crates():
    """This function defines a collection of repos and should be called in a WORKSPACE file"""
    maybe(
        http_archive,
        name = "raze__anyhow__1_0_32",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/anyhow/anyhow-1.0.32.crate",
        type = "tar.gz",
        strip_prefix = "anyhow-1.0.32",
        build_file = Label("//third_party/remote:anyhow-1.0.32.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__arc_swap__0_4_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/arc-swap/arc-swap-0.4.7.crate",
        type = "tar.gz",
        strip_prefix = "arc-swap-0.4.7",
        build_file = Label("//third_party/remote:arc-swap-0.4.7.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__async_trait__0_1_40",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/async-trait/async-trait-0.1.40.crate",
        type = "tar.gz",
        strip_prefix = "async-trait-0.1.40",
        build_file = Label("//third_party/remote:async-trait-0.1.40.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__autocfg__1_0_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/autocfg/autocfg-1.0.1.crate",
        type = "tar.gz",
        strip_prefix = "autocfg-1.0.1",
        build_file = Label("//third_party/remote:autocfg-1.0.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__bit_set__0_5_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/bit-set/bit-set-0.5.2.crate",
        type = "tar.gz",
        strip_prefix = "bit-set-0.5.2",
        build_file = Label("//third_party/remote:bit-set-0.5.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__bit_vec__0_6_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/bit-vec/bit-vec-0.6.2.crate",
        type = "tar.gz",
        strip_prefix = "bit-vec-0.6.2",
        build_file = Label("//third_party/remote:bit-vec-0.6.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__bitflags__1_2_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/bitflags/bitflags-1.2.1.crate",
        type = "tar.gz",
        strip_prefix = "bitflags-1.2.1",
        build_file = Label("//third_party/remote:bitflags-1.2.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__byteorder__1_3_4",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/byteorder/byteorder-1.3.4.crate",
        type = "tar.gz",
        strip_prefix = "byteorder-1.3.4",
        build_file = Label("//third_party/remote:byteorder-1.3.4.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__bytes__0_5_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/bytes/bytes-0.5.6.crate",
        type = "tar.gz",
        strip_prefix = "bytes-0.5.6",
        build_file = Label("//third_party/remote:bytes-0.5.6.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__cfg_if__0_1_10",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/cfg-if/cfg-if-0.1.10.crate",
        type = "tar.gz",
        strip_prefix = "cfg-if-0.1.10",
        build_file = Label("//third_party/remote:cfg-if-0.1.10.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__console__0_11_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/console/console-0.11.3.crate",
        type = "tar.gz",
        strip_prefix = "console-0.11.3",
        build_file = Label("//third_party/remote:console-0.11.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__difference__2_0_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/difference/difference-2.0.0.crate",
        type = "tar.gz",
        strip_prefix = "difference-2.0.0",
        build_file = Label("//third_party/remote:difference-2.0.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__dtoa__0_4_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/dtoa/dtoa-0.4.6.crate",
        type = "tar.gz",
        strip_prefix = "dtoa-0.4.6",
        build_file = Label("//third_party/remote:dtoa-0.4.6.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__encode_unicode__0_3_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/encode_unicode/encode_unicode-0.3.6.crate",
        type = "tar.gz",
        strip_prefix = "encode_unicode-0.3.6",
        build_file = Label("//third_party/remote:encode_unicode-0.3.6.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__fixedbitset__0_2_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/fixedbitset/fixedbitset-0.2.0.crate",
        type = "tar.gz",
        strip_prefix = "fixedbitset-0.2.0",
        build_file = Label("//third_party/remote:fixedbitset-0.2.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__fnv__1_0_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/fnv/fnv-1.0.7.crate",
        type = "tar.gz",
        strip_prefix = "fnv-1.0.7",
        build_file = Label("//third_party/remote:fnv-1.0.7.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__fuchsia_zircon__0_3_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/fuchsia-zircon/fuchsia-zircon-0.3.3.crate",
        type = "tar.gz",
        strip_prefix = "fuchsia-zircon-0.3.3",
        build_file = Label("//third_party/remote:fuchsia-zircon-0.3.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__fuchsia_zircon_sys__0_3_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/fuchsia-zircon-sys/fuchsia-zircon-sys-0.3.3.crate",
        type = "tar.gz",
        strip_prefix = "fuchsia-zircon-sys-0.3.3",
        build_file = Label("//third_party/remote:fuchsia-zircon-sys-0.3.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures/futures-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-0.3.5",
        build_file = Label("//third_party/remote:futures-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_channel__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-channel/futures-channel-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-channel-0.3.5",
        build_file = Label("//third_party/remote:futures-channel-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_core__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-core/futures-core-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-core-0.3.5",
        build_file = Label("//third_party/remote:futures-core-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_executor__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-executor/futures-executor-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-executor-0.3.5",
        build_file = Label("//third_party/remote:futures-executor-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_io__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-io/futures-io-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-io-0.3.5",
        build_file = Label("//third_party/remote:futures-io-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_macro__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-macro/futures-macro-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-macro-0.3.5",
        build_file = Label("//third_party/remote:futures-macro-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_sink__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-sink/futures-sink-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-sink-0.3.5",
        build_file = Label("//third_party/remote:futures-sink-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_task__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-task/futures-task-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-task-0.3.5",
        build_file = Label("//third_party/remote:futures-task-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__futures_util__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/futures-util/futures-util-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "futures-util-0.3.5",
        build_file = Label("//third_party/remote:futures-util-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__getrandom__0_1_15",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/getrandom/getrandom-0.1.15.crate",
        type = "tar.gz",
        strip_prefix = "getrandom-0.1.15",
        build_file = Label("//third_party/remote:getrandom-0.1.15.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__hashbrown__0_9_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/hashbrown/hashbrown-0.9.0.crate",
        type = "tar.gz",
        strip_prefix = "hashbrown-0.9.0",
        build_file = Label("//third_party/remote:hashbrown-0.9.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__hermit_abi__0_1_15",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/hermit-abi/hermit-abi-0.1.15.crate",
        type = "tar.gz",
        strip_prefix = "hermit-abi-0.1.15",
        build_file = Label("//third_party/remote:hermit-abi-0.1.15.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__indexmap__1_6_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/indexmap/indexmap-1.6.0.crate",
        type = "tar.gz",
        strip_prefix = "indexmap-1.6.0",
        build_file = Label("//third_party/remote:indexmap-1.6.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__insta__0_16_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/insta/insta-0.16.1.crate",
        type = "tar.gz",
        strip_prefix = "insta-0.16.1",
        build_file = Label("//third_party/remote:insta-0.16.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__iovec__0_1_4",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/iovec/iovec-0.1.4.crate",
        type = "tar.gz",
        strip_prefix = "iovec-0.1.4",
        build_file = Label("//third_party/remote:iovec-0.1.4.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__itoa__0_4_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/itoa/itoa-0.4.6.crate",
        type = "tar.gz",
        strip_prefix = "itoa-0.4.6",
        build_file = Label("//third_party/remote:itoa-0.4.6.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__kernel32_sys__0_2_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/kernel32-sys/kernel32-sys-0.2.2.crate",
        type = "tar.gz",
        strip_prefix = "kernel32-sys-0.2.2",
        build_file = Label("//third_party/remote:kernel32-sys-0.2.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__lazy_static__1_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/lazy_static/lazy_static-1.4.0.crate",
        type = "tar.gz",
        strip_prefix = "lazy_static-1.4.0",
        build_file = Label("//third_party/remote:lazy_static-1.4.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__libc__0_2_77",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/libc/libc-0.2.77.crate",
        type = "tar.gz",
        strip_prefix = "libc-0.2.77",
        build_file = Label("//third_party/remote:libc-0.2.77.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__linked_hash_map__0_5_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/linked-hash-map/linked-hash-map-0.5.3.crate",
        type = "tar.gz",
        strip_prefix = "linked-hash-map-0.5.3",
        build_file = Label("//third_party/remote:linked-hash-map-0.5.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__log__0_4_11",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/log/log-0.4.11.crate",
        type = "tar.gz",
        strip_prefix = "log-0.4.11",
        build_file = Label("//third_party/remote:log-0.4.11.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__memchr__2_3_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/memchr/memchr-2.3.3.crate",
        type = "tar.gz",
        strip_prefix = "memchr-2.3.3",
        build_file = Label("//third_party/remote:memchr-2.3.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__mio__0_6_22",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/mio/mio-0.6.22.crate",
        type = "tar.gz",
        strip_prefix = "mio-0.6.22",
        build_file = Label("//third_party/remote:mio-0.6.22.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__mio_named_pipes__0_1_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/mio-named-pipes/mio-named-pipes-0.1.7.crate",
        type = "tar.gz",
        strip_prefix = "mio-named-pipes-0.1.7",
        build_file = Label("//third_party/remote:mio-named-pipes-0.1.7.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__mio_uds__0_6_8",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/mio-uds/mio-uds-0.6.8.crate",
        type = "tar.gz",
        strip_prefix = "mio-uds-0.6.8",
        build_file = Label("//third_party/remote:mio-uds-0.6.8.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__miow__0_2_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/miow/miow-0.2.1.crate",
        type = "tar.gz",
        strip_prefix = "miow-0.2.1",
        build_file = Label("//third_party/remote:miow-0.2.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__miow__0_3_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/miow/miow-0.3.5.crate",
        type = "tar.gz",
        strip_prefix = "miow-0.3.5",
        build_file = Label("//third_party/remote:miow-0.3.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__net2__0_2_35",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/net2/net2-0.2.35.crate",
        type = "tar.gz",
        strip_prefix = "net2-0.2.35",
        build_file = Label("//third_party/remote:net2-0.2.35.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__num_traits__0_2_12",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num-traits/num-traits-0.2.12.crate",
        type = "tar.gz",
        strip_prefix = "num-traits-0.2.12",
        build_file = Label("//third_party/remote:num-traits-0.2.12.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__num_cpus__1_13_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/num_cpus/num_cpus-1.13.0.crate",
        type = "tar.gz",
        strip_prefix = "num_cpus-1.13.0",
        build_file = Label("//third_party/remote:num_cpus-1.13.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__once_cell__1_4_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/once_cell/once_cell-1.4.1.crate",
        type = "tar.gz",
        strip_prefix = "once_cell-1.4.1",
        build_file = Label("//third_party/remote:once_cell-1.4.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__petgraph__0_5_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/petgraph/petgraph-0.5.1.crate",
        type = "tar.gz",
        strip_prefix = "petgraph-0.5.1",
        build_file = Label("//third_party/remote:petgraph-0.5.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pico_args__0_3_4",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/pico-args/pico-args-0.3.4.crate",
        type = "tar.gz",
        strip_prefix = "pico-args-0.3.4",
        build_file = Label("//third_party/remote:pico-args-0.3.4.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pin_project__0_4_23",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/pin-project/pin-project-0.4.23.crate",
        type = "tar.gz",
        strip_prefix = "pin-project-0.4.23",
        build_file = Label("//third_party/remote:pin-project-0.4.23.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pin_project_internal__0_4_23",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/pin-project-internal/pin-project-internal-0.4.23.crate",
        type = "tar.gz",
        strip_prefix = "pin-project-internal-0.4.23",
        build_file = Label("//third_party/remote:pin-project-internal-0.4.23.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pin_project_lite__0_1_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/pin-project-lite/pin-project-lite-0.1.7.crate",
        type = "tar.gz",
        strip_prefix = "pin-project-lite-0.1.7",
        build_file = Label("//third_party/remote:pin-project-lite-0.1.7.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__pin_utils__0_1_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/pin-utils/pin-utils-0.1.0.crate",
        type = "tar.gz",
        strip_prefix = "pin-utils-0.1.0",
        build_file = Label("//third_party/remote:pin-utils-0.1.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__ppv_lite86__0_2_9",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ppv-lite86/ppv-lite86-0.2.9.crate",
        type = "tar.gz",
        strip_prefix = "ppv-lite86-0.2.9",
        build_file = Label("//third_party/remote:ppv-lite86-0.2.9.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__proc_macro_hack__0_5_18",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/proc-macro-hack/proc-macro-hack-0.5.18.crate",
        type = "tar.gz",
        strip_prefix = "proc-macro-hack-0.5.18",
        build_file = Label("//third_party/remote:proc-macro-hack-0.5.18.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__proc_macro_nested__0_1_6",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/proc-macro-nested/proc-macro-nested-0.1.6.crate",
        type = "tar.gz",
        strip_prefix = "proc-macro-nested-0.1.6",
        build_file = Label("//third_party/remote:proc-macro-nested-0.1.6.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__proc_macro2__1_0_21",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/proc-macro2/proc-macro2-1.0.21.crate",
        type = "tar.gz",
        strip_prefix = "proc-macro2-1.0.21",
        build_file = Label("//third_party/remote:proc-macro2-1.0.21.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__proptest__0_10_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/proptest/proptest-0.10.1.crate",
        type = "tar.gz",
        strip_prefix = "proptest-0.10.1",
        build_file = Label("//third_party/remote:proptest-0.10.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__quick_error__1_2_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/quick-error/quick-error-1.2.3.crate",
        type = "tar.gz",
        strip_prefix = "quick-error-1.2.3",
        build_file = Label("//third_party/remote:quick-error-1.2.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__quote__1_0_7",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/quote/quote-1.0.7.crate",
        type = "tar.gz",
        strip_prefix = "quote-1.0.7",
        build_file = Label("//third_party/remote:quote-1.0.7.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rand__0_7_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand/rand-0.7.3.crate",
        type = "tar.gz",
        strip_prefix = "rand-0.7.3",
        build_file = Label("//third_party/remote:rand-0.7.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rand_chacha__0_2_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand_chacha/rand_chacha-0.2.2.crate",
        type = "tar.gz",
        strip_prefix = "rand_chacha-0.2.2",
        build_file = Label("//third_party/remote:rand_chacha-0.2.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rand_core__0_5_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand_core/rand_core-0.5.1.crate",
        type = "tar.gz",
        strip_prefix = "rand_core-0.5.1",
        build_file = Label("//third_party/remote:rand_core-0.5.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rand_hc__0_2_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand_hc/rand_hc-0.2.0.crate",
        type = "tar.gz",
        strip_prefix = "rand_hc-0.2.0",
        build_file = Label("//third_party/remote:rand_hc-0.2.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rand_xorshift__0_2_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rand_xorshift/rand_xorshift-0.2.0.crate",
        type = "tar.gz",
        strip_prefix = "rand_xorshift-0.2.0",
        build_file = Label("//third_party/remote:rand_xorshift-0.2.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__redox_syscall__0_1_57",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/redox_syscall/redox_syscall-0.1.57.crate",
        type = "tar.gz",
        strip_prefix = "redox_syscall-0.1.57",
        build_file = Label("//third_party/remote:redox_syscall-0.1.57.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__regex_syntax__0_6_18",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/regex-syntax/regex-syntax-0.6.18.crate",
        type = "tar.gz",
        strip_prefix = "regex-syntax-0.6.18",
        build_file = Label("//third_party/remote:regex-syntax-0.6.18.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__remove_dir_all__0_5_3",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/remove_dir_all/remove_dir_all-0.5.3.crate",
        type = "tar.gz",
        strip_prefix = "remove_dir_all-0.5.3",
        build_file = Label("//third_party/remote:remove_dir_all-0.5.3.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__rusty_fork__0_3_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/rusty-fork/rusty-fork-0.3.0.crate",
        type = "tar.gz",
        strip_prefix = "rusty-fork-0.3.0",
        build_file = Label("//third_party/remote:rusty-fork-0.3.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__ryu__1_0_5",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ryu/ryu-1.0.5.crate",
        type = "tar.gz",
        strip_prefix = "ryu-1.0.5",
        build_file = Label("//third_party/remote:ryu-1.0.5.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__serde__1_0_116",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde/serde-1.0.116.crate",
        type = "tar.gz",
        strip_prefix = "serde-1.0.116",
        build_file = Label("//third_party/remote:serde-1.0.116.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__serde_derive__1_0_116",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde_derive/serde_derive-1.0.116.crate",
        type = "tar.gz",
        strip_prefix = "serde_derive-1.0.116",
        build_file = Label("//third_party/remote:serde_derive-1.0.116.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__serde_json__1_0_57",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde_json/serde_json-1.0.57.crate",
        type = "tar.gz",
        strip_prefix = "serde_json-1.0.57",
        build_file = Label("//third_party/remote:serde_json-1.0.57.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__serde_yaml__0_8_13",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/serde_yaml/serde_yaml-0.8.13.crate",
        type = "tar.gz",
        strip_prefix = "serde_yaml-0.8.13",
        build_file = Label("//third_party/remote:serde_yaml-0.8.13.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__signal_hook_registry__1_2_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/signal-hook-registry/signal-hook-registry-1.2.1.crate",
        type = "tar.gz",
        strip_prefix = "signal-hook-registry-1.2.1",
        build_file = Label("//third_party/remote:signal-hook-registry-1.2.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__slab__0_4_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/slab/slab-0.4.2.crate",
        type = "tar.gz",
        strip_prefix = "slab-0.4.2",
        build_file = Label("//third_party/remote:slab-0.4.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__socket2__0_3_15",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/socket2/socket2-0.3.15.crate",
        type = "tar.gz",
        strip_prefix = "socket2-0.3.15",
        build_file = Label("//third_party/remote:socket2-0.3.15.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__syn__1_0_41",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/syn/syn-1.0.41.crate",
        type = "tar.gz",
        strip_prefix = "syn-1.0.41",
        build_file = Label("//third_party/remote:syn-1.0.41.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__tempfile__3_1_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/tempfile/tempfile-3.1.0.crate",
        type = "tar.gz",
        strip_prefix = "tempfile-3.1.0",
        build_file = Label("//third_party/remote:tempfile-3.1.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__terminal_size__0_1_13",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/terminal_size/terminal_size-0.1.13.crate",
        type = "tar.gz",
        strip_prefix = "terminal_size-0.1.13",
        build_file = Label("//third_party/remote:terminal_size-0.1.13.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__termios__0_3_2",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/termios/termios-0.3.2.crate",
        type = "tar.gz",
        strip_prefix = "termios-0.3.2",
        build_file = Label("//third_party/remote:termios-0.3.2.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__thiserror__1_0_20",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/thiserror/thiserror-1.0.20.crate",
        type = "tar.gz",
        strip_prefix = "thiserror-1.0.20",
        build_file = Label("//third_party/remote:thiserror-1.0.20.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__thiserror_impl__1_0_20",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/thiserror-impl/thiserror-impl-1.0.20.crate",
        type = "tar.gz",
        strip_prefix = "thiserror-impl-1.0.20",
        build_file = Label("//third_party/remote:thiserror-impl-1.0.20.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__tokio__0_2_22",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/tokio/tokio-0.2.22.crate",
        type = "tar.gz",
        strip_prefix = "tokio-0.2.22",
        build_file = Label("//third_party/remote:tokio-0.2.22.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__unicode_xid__0_2_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/unicode-xid/unicode-xid-0.2.1.crate",
        type = "tar.gz",
        strip_prefix = "unicode-xid-0.2.1",
        build_file = Label("//third_party/remote:unicode-xid-0.2.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__wait_timeout__0_2_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/wait-timeout/wait-timeout-0.2.0.crate",
        type = "tar.gz",
        strip_prefix = "wait-timeout-0.2.0",
        build_file = Label("//third_party/remote:wait-timeout-0.2.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__wasi__0_9_0_wasi_snapshot_preview1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/wasi/wasi-0.9.0+wasi-snapshot-preview1.crate",
        type = "tar.gz",
        strip_prefix = "wasi-0.9.0+wasi-snapshot-preview1",
        build_file = Label("//third_party/remote:wasi-0.9.0+wasi-snapshot-preview1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__winapi__0_2_8",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi/winapi-0.2.8.crate",
        type = "tar.gz",
        strip_prefix = "winapi-0.2.8",
        build_file = Label("//third_party/remote:winapi-0.2.8.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__winapi__0_3_9",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi/winapi-0.3.9.crate",
        type = "tar.gz",
        strip_prefix = "winapi-0.3.9",
        build_file = Label("//third_party/remote:winapi-0.3.9.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__winapi_build__0_1_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-build/winapi-build-0.1.1.crate",
        type = "tar.gz",
        strip_prefix = "winapi-build-0.1.1",
        build_file = Label("//third_party/remote:winapi-build-0.1.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__winapi_i686_pc_windows_gnu__0_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-i686-pc-windows-gnu/winapi-i686-pc-windows-gnu-0.4.0.crate",
        type = "tar.gz",
        strip_prefix = "winapi-i686-pc-windows-gnu-0.4.0",
        build_file = Label("//third_party/remote:winapi-i686-pc-windows-gnu-0.4.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__winapi_x86_64_pc_windows_gnu__0_4_0",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/winapi-x86_64-pc-windows-gnu/winapi-x86_64-pc-windows-gnu-0.4.0.crate",
        type = "tar.gz",
        strip_prefix = "winapi-x86_64-pc-windows-gnu-0.4.0",
        build_file = Label("//third_party/remote:winapi-x86_64-pc-windows-gnu-0.4.0.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__ws2_32_sys__0_2_1",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/ws2_32-sys/ws2_32-sys-0.2.1.crate",
        type = "tar.gz",
        strip_prefix = "ws2_32-sys-0.2.1",
        build_file = Label("//third_party/remote:ws2_32-sys-0.2.1.BUILD.bazel"),
    )

    maybe(
        http_archive,
        name = "raze__yaml_rust__0_4_4",
        url = "https://crates-io.s3-us-west-1.amazonaws.com/crates/yaml-rust/yaml-rust-0.4.4.crate",
        type = "tar.gz",
        strip_prefix = "yaml-rust-0.4.4",
        build_file = Label("//third_party/remote:yaml-rust-0.4.4.BUILD.bazel"),
    )
