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
    # Need a better way to keep updating this.
    branch = "persistentworker",
    remote = "/home/nikhil/rules_rust",
        # Master branch as of 2020-09-16.
        # "https://github.com/bazelbuild/rules_rust/archive/7d9e890c58ca00eacf8dd4a2ba991cfe9c3f44e4.tar.gz",
)

load("@io_bazel_rules_rust//rust:repositories.bzl", "rust_repositories")
rust_repositories(version="nightly", edition="2018", iso_date="2020-08-24")

load("@io_bazel_rules_rust//:workspace.bzl", "bazel_version")
bazel_version(name = "bazel_version")

http_file(
    name = "rustc_worker",
    urls = ["file:///home/nikhil/rustc-worker/target/debug/rustc-worker"],
    executable = True,
)

raze_fetch_remote_crates()
