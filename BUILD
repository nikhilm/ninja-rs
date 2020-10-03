# Customizations to be able to use the worker while not clashing with our deps.
load("@io_bazel_rules_rust//proto:toolchain.bzl", "rust_proto_toolchain")

rust_proto_toolchain(
    name = "proto-toolchain-impl",
    # Path to the protobuf compiler.
    protoc = "@com_google_protobuf//:protoc",
    # Protobuf compiler plugin to generate rust gRPC stubs.
    grpc_plugin = "//cargo_raze/remote:cargo_bin_protoc_gen_rust_grpc",
    # Protobuf compiler plugin to generate rust protobuf stubs.
    proto_plugin = "//cargo_raze/remote:cargo_bin_protoc_gen_rust",
)

toolchain(
    name = "proto-toolchain",
    toolchain = ":proto-toolchain-impl",
    toolchain_type = "@io_bazel_rules_rust//proto:toolchain",
)
