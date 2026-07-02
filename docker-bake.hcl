// Bake definitions for the production Cassandra image, built by
// .github/workflows/docker.yml. Each target here corresponds 1:1 to a
// `bake_target` entry in that workflow's build matrix and a same-named
// stage in ./Dockerfile.
//
// The workflow invokes bake per (variant, arch) with `--set` overrides
// for `platform`, `output`, `cache-from`, and `cache-to` -- the
// `platforms` default below only matters for local/manual
// `docker buildx bake` runs.

variable "REGISTRY_IMAGE" {
  default = ""
}

group "default" {
  targets = [
    "runtime",
    "runtime-nonroot",
    "runtime-code",
    "runtime-code-nonroot",
    "runtime-slim",
    "runtime-slim-nonroot",
    "runtime-code-slim",
    "runtime-code-slim-nonroot",
  ]
}

target "_common" {
  context    = "."
  dockerfile = "Dockerfile"
  platforms  = ["linux/amd64", "linux/arm64"]
}

target "runtime" {
  inherits = ["_common"]
  target   = "runtime"
}

target "runtime-nonroot" {
  inherits = ["_common"]
  target   = "runtime-nonroot"
}

target "runtime-code" {
  inherits = ["_common"]
  target   = "runtime-code"
}

target "runtime-code-nonroot" {
  inherits = ["_common"]
  target   = "runtime-code-nonroot"
}

target "runtime-slim" {
  inherits = ["_common"]
  target   = "runtime-slim"
}

target "runtime-slim-nonroot" {
  inherits = ["_common"]
  target   = "runtime-slim-nonroot"
}

target "runtime-code-slim" {
  inherits = ["_common"]
  target   = "runtime-code-slim"
}

target "runtime-code-slim-nonroot" {
  inherits = ["_common"]
  target   = "runtime-code-slim-nonroot"
}
