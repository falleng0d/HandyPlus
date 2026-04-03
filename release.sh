#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./release.sh [--repo OWNER/REPO] [--model MODEL]

Creates a GitHub release for the current version from `package.json`.

What it does:
- validates a clean git worktree
- infers the GitHub repo from `origin` unless `--repo` is provided
- fetches origin tags and matches the existing semver tag format (`0.8.1` vs `v0.8.1`)
- asks `opencode run` to generate release notes from commits since the previous release tag
- builds the application with `bun run build`
- finds current-version release assets under `src-tauri/target/release`
- creates and pushes the git tag if needed
- creates the GitHub release and uploads the assets

Options:
  --repo OWNER/REPO   Override the GitHub repo derived from `origin`
  --model MODEL       Override the opencode model
  -h, --help          Show this help message

Examples:
  ./release.sh
  ./release.sh --repo falleng0d/HandyPlus
  ./release.sh --model opencode/minimax-m2.5-free
EOF
}

die() {
  printf 'Error: %s\n' "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

infer_repo_from_origin() {
  local remote_url="$1"
  local repo=""

  case "$remote_url" in
    git@github.com:*)
      repo="${remote_url#git@github.com:}"
      ;;
    ssh://git@github.com/*)
      repo="${remote_url#ssh://git@github.com/}"
      ;;
    https://github.com/*)
      repo="${remote_url#https://github.com/}"
      ;;
    http://github.com/*)
      repo="${remote_url#http://github.com/}"
      ;;
  esac

  repo="${repo%.git}"
  [[ -n "$repo" ]] || return 1
  printf '%s\n' "$repo"
}

read_package_version() {
  jq -r '.version' package.json
}

read_cargo_package_version() {
  awk '
    BEGIN { in_package = 0 }
    /^[[:space:]]*\[package\][[:space:]]*$/ {
      in_package = 1
      next
    }
    /^[[:space:]]*\[/ {
      in_package = 0
    }
    in_package == 1 && /^[[:space:]]*version[[:space:]]*=/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
    }
  ' src-tauri/Cargo.toml
}

validate_clean_worktree() {
  local status_output
  status_output="$(git status --porcelain)"
  [[ -z "$status_output" ]] || die "Working tree must be clean before creating a release"
}

validate_version_consistency() {
  local package_version tauri_version nosign_version cargo_version

  package_version="$(read_package_version)"
  tauri_version="$(jq -r '.version' src-tauri/tauri.conf.json)"
  nosign_version="$(jq -r '.version' src-tauri/tauri.nosign.json)"
  cargo_version="$(read_cargo_package_version)"

  [[ "$package_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "package.json version must use major.middle.minor format"
  [[ "$tauri_version" == "$package_version" ]] || die "src-tauri/tauri.conf.json version does not match package.json"
  [[ "$nosign_version" == "$package_version" ]] || die "src-tauri/tauri.nosign.json version does not match package.json"
  [[ "$cargo_version" == "$package_version" || "$cargo_version" == "$package_version"-* ]] || {
    die "src-tauri/Cargo.toml package version must start with $package_version"
  }
}

latest_remote_semver_tags() {
  git ls-remote --tags origin \
    | awk '{print $2}' \
    | sed 's#refs/tags/##' \
    | sed '/\^{}$/d' \
    | grep -E '^v?[0-9]+\.[0-9]+\.[0-9]+$' \
    | sort -V
}

resolve_remote_tag_commit() {
  local tag="$1"

  git ls-remote --tags origin "refs/tags/$tag" "refs/tags/$tag^{}" \
    | awk '
        $2 ~ /\^\{\}$/ { peeled = $1 }
        END {
          if (peeled != "") {
            print peeled
          } else if (NR > 0) {
            print $1
          }
        }
      '
}

build_release_notes_prompt() {
  local version="$1"
  local release_tag="$2"
  local previous_tag="$3"
  local repo_root="$4"

  if [[ -n "$previous_tag" ]]; then
    cat <<EOF
You are preparing GitHub release notes for HandyPlus ${version}.

Repository root: ${repo_root}
Current release tag to create: ${release_tag}
Previous release tag: ${previous_tag}

Task:
- Explore every commit in the range ${previous_tag}..HEAD.
- Use git log, git show, and git diff as needed to understand the real user-visible changes.
- Focus on shipped behavior, bug fixes, UI improvements, and noteworthy maintenance work.
- Be accurate and do not invent changes.

Output requirements:
- Return ONLY GitHub-flavored Markdown for the release description.
- Do not include any preamble or closing commentary.
- Do not wrap the whole response in code fences.
- Keep it concise and useful for a GitHub release page.

Preferred structure:
## Summary
- 3 to 6 bullets

## Details
- 2 to 8 bullets

If a section would be empty, omit it.
EOF
  else
    cat <<EOF
You are preparing the first semver GitHub release notes for HandyPlus ${version}.

Repository root: ${repo_root}
Current release tag to create: ${release_tag}

Task:
- Explore the commit history reachable from HEAD.
- Use git log, git show, and git diff as needed to understand the real user-visible changes.
- Focus on shipped behavior, bug fixes, UI improvements, and noteworthy maintenance work.
- Be accurate and do not invent changes.

Output requirements:
- Return ONLY GitHub-flavored Markdown for the release description.
- Do not include any preamble or closing commentary.
- Do not wrap the whole response in code fences.
- Keep it concise and useful for a GitHub release page.

Preferred structure:
## Summary
- 3 to 6 bullets

## Details
- 2 to 8 bullets

If a section would be empty, omit it.
EOF
  fi
}

collect_release_assets() {
  local version="$1"
  local release_root="$2"
  local file

  [[ -d "$release_root" ]] || die "Release output directory not found: $release_root"

  while IFS= read -r -d '' file; do
    RELEASE_ASSETS+=("$file")
  done < <(
    find "$release_root/bundle" -type f \( \
      -name "*${version}*.msi" -o \
      -name "*${version}*.exe" -o \
      -name "*${version}*.deb" -o \
      -name "*${version}*.AppImage" -o \
      -name "*${version}*.rpm" -o \
      -name "*${version}*.dmg" -o \
      -name "*${version}*.zip" -o \
      -name "*${version}*.tar.gz" \
    \) -print0 2>/dev/null
  )

  if [[ -f "$release_root/latest.json" ]]; then
    RELEASE_ASSETS+=("$release_root/latest.json")
  fi

  if [[ -d "$release_root/bundle" ]]; then
    while IFS= read -r -d '' file; do
      RELEASE_ASSETS+=("$file")
    done < <(
      find "$release_root/bundle" -type f -name "*.sig" -print0 2>/dev/null
    )
  fi

  [[ ${#RELEASE_ASSETS[@]} -gt 0 ]] || {
    die "No release assets found for version $version under $release_root"
  }
}

repo_override=""
opencode_model="opencode/minimax-m2.5-free"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      shift
      [[ $# -gt 0 ]] || die "Missing value for --repo"
      repo_override="$1"
      ;;
    --model)
      shift
      [[ $# -gt 0 ]] || die "Missing value for --model"
      opencode_model="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "Unknown argument: $1"
      ;;
  esac
  shift
done

require_command git
require_command jq
require_command gh
require_command bun
require_command opencode
require_command mktemp

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || die "Not inside a git repository"
cd "$repo_root"

validate_clean_worktree
validate_version_consistency

version="$(read_package_version)"
head_commit="$(git rev-parse HEAD)"
origin_url="$(git remote get-url origin 2>/dev/null)" || die "Failed to resolve origin remote"

if [[ -n "$repo_override" ]]; then
  repo="$repo_override"
else
  repo="$(infer_repo_from_origin "$origin_url")" || die "Could not infer GitHub repo from origin: $origin_url"
fi

git fetch origin --tags --prune >/dev/null

mapfile -t semver_tags < <(latest_remote_semver_tags || true)

tag_prefix=""
if [[ ${#semver_tags[@]} -gt 0 ]]; then
  latest_existing_tag="${semver_tags[${#semver_tags[@]} - 1]}"
  if [[ "$latest_existing_tag" == v* ]]; then
    tag_prefix="v"
  fi
fi

release_tag="${tag_prefix}${version}"

previous_tag=""
if [[ ${#semver_tags[@]} -gt 0 ]]; then
  filtered_tags=()
  for tag in "${semver_tags[@]}"; do
    if [[ "$tag" != "$release_tag" ]]; then
      filtered_tags+=("$tag")
    fi
  done

  if [[ ${#filtered_tags[@]} -gt 0 ]]; then
    previous_tag="${filtered_tags[${#filtered_tags[@]} - 1]}"
  fi
fi

if gh release view "$release_tag" --repo "$repo" >/dev/null 2>&1; then
  die "Release $release_tag already exists on $repo"
fi

remote_tag_commit="$(resolve_remote_tag_commit "$release_tag")"
if [[ -n "$remote_tag_commit" && "$remote_tag_commit" != "$head_commit" ]]; then
  die "Remote tag $release_tag already exists on origin and does not point to HEAD"
fi

if git rev-parse -q --verify "refs/tags/$release_tag" >/dev/null; then
  local_tag_commit="$(git rev-list -n 1 "$release_tag")"
  [[ "$local_tag_commit" == "$head_commit" ]] || {
    die "Local tag $release_tag already exists and does not point to HEAD"
  }
fi

notes_file="$(mktemp)"
trap 'rm -f "$notes_file"' EXIT

release_notes_prompt="$(build_release_notes_prompt "$version" "$release_tag" "$previous_tag" "$repo_root")"
opencode run --dir "$repo_root" --model "$opencode_model" "$release_notes_prompt" > "$notes_file"

[[ -s "$notes_file" ]] || die "Generated release notes are empty"

bun run build

declare -a RELEASE_ASSETS=()
collect_release_assets "$version" "$repo_root/src-tauri/target/release"

if ! git rev-parse -q --verify "refs/tags/$release_tag" >/dev/null; then
  git tag -a "$release_tag" -m "Release $release_tag"
fi

if [[ -z "$remote_tag_commit" ]]; then
  git push origin "refs/tags/$release_tag"
fi

release_cmd=(
  gh release create "$release_tag"
  --repo "$repo"
  --title "$release_tag"
  --notes-file "$notes_file"
)

for asset in "${RELEASE_ASSETS[@]}"; do
  release_cmd+=("$asset")
done

"${release_cmd[@]}"
