load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive", "http_file")
load("//third_party:crates.bzl", "raze_fetch_remote_crates")

http_archive(
    name = "bazel_skylib",
    urls = [
        "https://github.com/bazelbuild/bazel-skylib/releases/download/1.0.3/bazel-skylib-1.0.3.tar.gz",
        "https://mirror.bazel.build/github.com/bazelbuild/bazel-skylib/releases/download/1.0.3/bazel-skylib-1.0.3.tar.gz",
    ],
    sha256 = "1c531376ac7e5a180e0237938a2536de0c54d93f5c278634818e0efc952dd56c",
)

load("@bazel_skylib//:workspace.bzl", "bazel_skylib_workspace")
bazel_skylib_workspace()

git_repository(
    name = "io_bazel_rules_rust",
    branch = "persistentworker",
    remote = "https://github.com/nikhilm/rules_rust",
)

load("@io_bazel_rules_rust//rust:repositories.bzl", "rust_repositories")
rust_repositories(version="nightly", edition="2018", iso_date="2020-08-24")

load("@io_bazel_rules_rust//:workspace.bzl", "bazel_version")
bazel_version(name = "bazel_version")

http_file(
    name = "rustc_worker",
    urls = ["https://github.com/nikhilm/rustc-worker/releases/download/v0.1.0/rustc-worker-linux"],
    sha256 = "0e2be6198d556e4972c52ee4bcd76be2c2d8fd74c58e0bf9be195c798e2a9a4e",
    executable = True,
)

raze_fetch_remote_crates()
