#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./bump.sh [--major N] [--middle N] [--minor N]

Bumps one or more version parts from `package.json` and writes the new version to:
- package.json
- src-tauri/tauri.conf.json
- src-tauri/tauri.nosign.json
- src-tauri/Cargo.toml
- .cargo/config.toml (as HANDY_VERSION)

Examples:
  ./bump.sh --minor 1
  ./bump.sh --middle 1 --minor 4
  ./bump.sh --major 1 --middle 2 --minor 3
EOF
}

die() {
  printf 'Error: %s\n' "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

is_non_negative_integer() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

major_bump=0
middle_bump=0
minor_bump=0
specified_count=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --major|--middle|--minor)
      option="$1"
      shift
      [[ $# -gt 0 ]] || die "Missing value for $option"
      is_non_negative_integer "$1" || die "Value for $option must be a non-negative integer"

      case "$option" in
        --major)
          major_bump=$((major_bump + $1))
          ;;
        --middle)
          middle_bump=$((middle_bump + $1))
          ;;
        --minor)
          minor_bump=$((minor_bump + $1))
          ;;
      esac

      specified_count=$((specified_count + 1))
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

(( specified_count > 0 )) || die "Please specify at least one of --major, --middle, or --minor"

require_command jq
require_command mktemp

for file in package.json src-tauri/tauri.conf.json src-tauri/tauri.nosign.json src-tauri/Cargo.toml .cargo/config.toml; do
  [[ -f "$file" ]] || die "Required file not found: $file"
done

current_version="$(jq -r '.version' package.json)"
[[ "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || die "package.json version must use major.middle.minor format"

current_cargo_version="$(awk '
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
' src-tauri/Cargo.toml)"
[[ "$current_cargo_version" =~ ^([0-9]+\.[0-9]+\.[0-9]+)(.*)$ ]] || die "src-tauri/Cargo.toml package version must start with major.middle.minor"
cargo_version_suffix="${BASH_REMATCH[2]}"

IFS='.' read -r current_major current_middle current_minor <<<"$current_version"
new_version="$((current_major + major_bump)).$((current_middle + middle_bump)).$((current_minor + minor_bump))"

update_json_version() {
  local file="$1"
  local temp_file

  temp_file="$(mktemp)"
  jq --arg version "$new_version" '.version = $version' "$file" > "$temp_file"
  mv "$temp_file" "$file"
}

update_cargo_toml_version() {
  local file="$1"
  local temp_file
  local cargo_version="${new_version}${cargo_version_suffix}"

  temp_file="$(mktemp)"
  awk -v version="$cargo_version" '
    BEGIN { in_package = 0; updated = 0 }
    /^[[:space:]]*\[package\][[:space:]]*$/ {
      in_package = 1
      print
      next
    }
    /^[[:space:]]*\[/ {
      in_package = 0
    }
    in_package == 1 && updated == 0 && /^[[:space:]]*version[[:space:]]*=/ {
      print "version = \"" version "\""
      updated = 1
      next
    }
    { print }
    END {
      if (updated == 0) {
        exit 1
      }
    }
  ' "$file" > "$temp_file" || {
    rm -f "$temp_file"
    die "Failed to update package version in $file"
  }

  mv "$temp_file" "$file"
}

update_cargo_config_version() {
  local file="$1"
  local temp_file

  temp_file="$(mktemp)"

  if grep -Eq '^[[:space:]]*HANDY_VERSION[[:space:]]*=' "$file"; then
    awk -v version="$new_version" '
      BEGIN { updated = 0 }
      /^[[:space:]]*HANDY_VERSION[[:space:]]*=/ && updated == 0 {
        print "HANDY_VERSION = \"" version "\""
        updated = 1
        next
      }
      { print }
    ' "$file" > "$temp_file"
  elif grep -Eq '^[[:space:]]*\[env\][[:space:]]*$' "$file"; then
    awk -v version="$new_version" '
      BEGIN { inserted = 0 }
      {
        print
        if ($0 ~ /^[[:space:]]*\[env\][[:space:]]*$/ && inserted == 0) {
          print "HANDY_VERSION = \"" version "\""
          inserted = 1
        }
      }
    ' "$file" > "$temp_file"
  else
    cat "$file" > "$temp_file"
    if [[ -s "$temp_file" ]]; then
      printf '\n' >> "$temp_file"
    fi
    printf '[env]\nHANDY_VERSION = "%s"\n' "$new_version" >> "$temp_file"
  fi

  mv "$temp_file" "$file"
}

update_json_version package.json
update_json_version src-tauri/tauri.conf.json
update_json_version src-tauri/tauri.nosign.json
update_cargo_toml_version src-tauri/Cargo.toml
# update_cargo_config_version .cargo/config.toml

printf 'Updated version: %s -> %s\n' "$current_version" "$new_version"
