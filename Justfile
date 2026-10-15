set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

signing_identity := env_var_or_default("HANDY_SIGNING_IDENTITY", "HandyPlus Dev")
certificate_sha1 := env_var_or_default("HANDY_CERTIFICATE_SHA1", "8F8046C3F7528B1F860E80CA4CD212B9166F51D4")
signing_dir := env_var_or_default("HANDYPLUS_SIGNING_DIR", home_directory() / ".handyplus-signing")
app_source := "src-tauri/target/release/bundle/macos/HandyPlus.app"
app_destination := env_var_or_default("HANDY_APP_PATH", "/Applications/HandyPlus.app")
helper_label := "com.falleng0d.karabiner-input.helper.local"
installed_helper := "/Library/PrivilegedHelperTools/com.falleng0d.karabiner-input.helper"

export KARABINER_INPUT_LOCAL_CERT_SHA1 := certificate_sha1

default:
	@just --list

# Lint, format and type-check the frontend and backend.
check:
	bun check

# Run the Rust unit tests.
test:
	cd src-tauri; cargo test --lib

# Build unsigned installers.
build:
	bun run build

# Build installers signed with the HandyPlus certificate.
[windows]
build-signed:
	bun run build-signed

# Create the GitHub release for the current version (see release.sh --help).
[windows]
release *args:
	sh ./release.sh {{ args }}

# Add the signing certificate and key to the current user's certificate store.
[windows]
import-cert:
	certutil -f -user -p (Get-Content -Raw "{{ signing_dir }}/password.txt") -importpfx My "{{ signing_dir }}/handyplus-signing.p12" NoRoot

# Trust the certificate on this PC so signatures show as valid (Windows asks to confirm).
[windows]
trust-cert:
	certutil -user -addstore Root "src-tauri/signing/handyplus-dev.pem"
	certutil -user -addstore TrustedPublisher "src-tauri/signing/handyplus-dev.pem"

# Add the signing certificate to the login keychain and trust it (macOS asks for your password).
[macos]
import-cert:
	security import "{{ signing_dir }}/handyplus-signing.p12" -k "${HOME}/Library/Keychains/login.keychain-db" -P "$(cat "{{ signing_dir }}/password.txt")" -T /usr/bin/codesign
	security add-trusted-cert -r trustRoot -p codeSign -k "${HOME}/Library/Keychains/login.keychain-db" "src-tauri/signing/handyplus-dev.pem"
	just local-check-certificate

# Build and sign the local release app.
[macos]
local-build: local-check-certificate
	bun run tauri build --bundles app --config src-tauri/tauri.signed.json

# Confirm the configured certificate and private key are available.
[macos]
local-check-certificate:
	@actual="$(security find-certificate -c "{{ signing_identity }}" -Z | awk '/SHA-1/{print $3; exit}')"; \
		test "$actual" = "{{ certificate_sha1 }}" || { \
			echo "Expected {{ signing_identity }} certificate {{ certificate_sha1 }}, found ${actual:-none}" >&2; \
			exit 1; \
		}
	@security find-identity -v -p codesigning | rg -F '"{{ signing_identity }}"' >/dev/null

# Stop the installed app if it is running.
[macos]
local-stop:
	@osascript -e 'tell application id "com.pais.handy" to quit' 2>/dev/null || true; \
		for _ in {1..50}; do \
			pgrep -f '^{{ app_destination }}/Contents/MacOS/handy$' >/dev/null || exit 0; \
			sleep 0.1; \
		done; \
		echo "HandyPlus did not exit within 5 seconds" >&2; \
		exit 1

# Copy the latest signed release into /Applications.
[macos]
local-install-app: local-stop
	test -d "{{ app_source }}"
	ditto "{{ app_source }}" "{{ app_destination }}"
	codesign --verify --deep --strict --verbose=2 "{{ app_destination }}"

# Install or replace the local root helper. Prompts for administrator approval.
[macos]
local-install-helper:
	bash scripts/install-karabiner-helper-local.sh "{{ app_destination }}"

# Verify app/helper signatures, service state, plist, and IPC socket.
[macos]
local-verify:
	codesign --verify --deep --strict --verbose=2 "{{ app_destination }}"
	codesign -R='identifier "com.pais.handy" and certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ app_destination }}/Contents/MacOS/handy"
	codesign -R='certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ app_destination }}/Contents/MacOS/karabiner-input-helper"
	codesign -R='certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ installed_helper }}"
	plutil -lint "{{ app_destination }}/Contents/Library/LaunchDaemons/com.falleng0d.karabiner-input.helper.plist"
	launchctl print "system/{{ helper_label }}" | rg 'state = running'
	test -S /var/run/karabiner-input.sock
	test "$(stat -f '%Su:%Sp' /var/run/karabiner-input.sock)" = "${USER}:srw-------"

# Show the latest HandyPlus application log entries.
[macos]
local-logs:
	tail -n 200 "${HOME}/Library/Logs/com.pais.handy/handy.log"

# Follow HandyPlus application logs while reproducing a problem.
[macos]
local-logs-follow:
	tail -f "${HOME}/Library/Logs/com.pais.handy/handy.log"

# Show recent privileged helper logs.
[macos]
local-helper-logs:
	tail -n 200 /var/log/karabiner-input-helper.log

# Follow privileged helper logs while reproducing a problem.
[macos]
local-helper-logs-follow:
	tail -f /var/log/karabiner-input-helper.log

# Print service, process, socket, signature, and recent error diagnostics.
[macos]
local-diagnose:
	@echo '=== Launch daemon ==='
	@launchctl print "system/{{ helper_label }}" || true
	@echo '=== Processes ==='
	@pgrep -alf 'HandyPlus.app/Contents/MacOS/handy|karabiner-input-helper' || true
	@echo '=== IPC socket ==='
	@ls -la /var/run/karabiner-input.sock || true
	@echo '=== Installed signatures ==='
	@codesign --verify --deep --strict --verbose=2 "{{ app_destination }}" || true
	@codesign --verify --strict --verbose=2 "{{ installed_helper }}" || true
	@echo '=== Recent HandyPlus errors ==='
	@rg -i 'Failed to paste|Using paste method: Karabiner|Karabiner input helper|helper rejected|unsupported input' "${HOME}/Library/Logs/com.pais.handy/handy.log" | tail -n 100 || true
	@echo '=== Recent helper logs ==='
	@tail -n 100 /var/log/karabiner-input-helper.log 2>/dev/null || echo 'No helper log yet; rerun just local-install-helper to enable it.'

# Relaunch the installed app.
[macos]
local-restart: local-stop
	open "{{ app_destination }}"

# Build, install, verify, and restart everything needed for local iteration.
[macos]
local-all: local-build
	just local-install-app
	just local-install-helper
	just local-verify
	just local-restart

# Remove only the local-development launch daemon and helper.
[macos]
local-uninstall-helper:
	sudo launchctl bootout "system/{{ helper_label }}" 2>/dev/null || true
	sudo rm -f "/Library/LaunchDaemons/{{ helper_label }}.plist" "{{ installed_helper }}" /var/run/karabiner-input.sock /var/log/karabiner-input-helper.log
