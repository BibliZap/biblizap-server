#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
SCRIPT_NAME=$(basename -- "$0")

BINARY_SOURCE="$SCRIPT_DIR/target/release/biblizap-monitor"
BINARY_TARGET="/usr/bin/biblizap-monitor"
CONFIG_DIR="/etc/biblizap-monitor"
CONFIG_FILE="$CONFIG_DIR/biblizap-monitor.toml"
SERVICE_SOURCE="$SCRIPT_DIR/deploy/biblizap-monitor.service"
SERVICE_TARGET="/etc/systemd/system/biblizap-monitor.service"
SERVICE_NAME="biblizap-monitor.service"
SERVICE_USER="biblizap-monitor"
SERVICE_GROUP="biblizap-monitor"
RECONFIGURE=false

usage() {
	cat <<EOF
Usage: $SCRIPT_NAME [--binary PATH] [--reconfigure]

Install or upgrade the BibliZap external monitor and its systemd service.

Options:
	--binary PATH    Release binary to install
	                 (default: ./target/release/biblizap-monitor)
	--reconfigure    Replace the existing configuration after confirmation
	-h, --help       Show this help
EOF
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--binary)
			if [ "$#" -lt 2 ]; then
				echo "--binary requires a path" >&2
				exit 1
			fi
			BINARY_SOURCE="$2"
			shift 2
			;;
		--reconfigure)
			RECONFIGURE=true
			shift
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "Unknown argument: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

if [ "$(id -u)" -ne 0 ]; then
	echo "This installer must be run as root (or via sudo)." >&2
	exit 1
fi

if ! command -v systemctl >/dev/null 2>&1; then
	echo "systemd is required to install the monitor service." >&2
	exit 1
fi

if [ ! -f "$BINARY_SOURCE" ]; then
	echo "Monitor binary not found at '$BINARY_SOURCE'." >&2
	echo "Build it first with: cargo build --release -p biblizap-monitor" >&2
	exit 1
fi

if [ ! -f "$SERVICE_SOURCE" ]; then
	echo "Systemd unit not found at '$SERVICE_SOURCE'." >&2
	exit 1
fi

prompt_required() {
	local variable_name=$1
	local prompt=$2
	local default_value=${3:-}
	local value

	while true; do
		if [ -n "$default_value" ]; then
			read -r -p "$prompt [$default_value]: " value
			value=${value:-$default_value}
		else
			read -r -p "$prompt: " value
		fi
		if [ -n "$value" ]; then
			printf -v "$variable_name" '%s' "$value"
			return
		fi
		echo "A value is required." >&2
	done
}

prompt_positive_integer() {
	local variable_name=$1
	local prompt=$2
	local default_value=$3
	local value

	while true; do
		read -r -p "$prompt [$default_value]: " value
		value=${value:-$default_value}
		if [[ "$value" =~ ^[1-9][0-9]*$ ]]; then
			printf -v "$variable_name" '%s' "$value"
			return
		fi
		echo "Enter a positive integer." >&2
	done
}

prompt_port() {
	local variable_name=$1
	local prompt=$2
	local default_value=$3
	local value

	while true; do
		read -r -p "$prompt [$default_value]: " value
		value=${value:-$default_value}
		if [[ "$value" =~ ^[1-9][0-9]{0,4}$ ]] && [ "$value" -le 65535 ]; then
			printf -v "$variable_name" '%s' "$value"
			return
		fi
		echo "Enter a port between 1 and 65535." >&2
	done
}

toml_escape() {
	REPLY=${1//\\/\\\\}
	REPLY=${REPLY//\"/\\\"}
}

write_configuration() {
	if [ ! -t 0 ]; then
		echo "Interactive input is required to create the configuration." >&2
		exit 1
	fi

	echo "Configuring the BibliZap monitor. Press Enter to accept defaults."
	while true; do
		prompt_required HEALTH_URL "Public HTTPS health URL"
		if [[ "$HEALTH_URL" == https://* ]]; then
			break
		fi
		echo "The health URL must begin with https://" >&2
	done
	prompt_positive_integer CHECK_INTERVAL "Check interval in seconds" "30"
	prompt_positive_integer REQUEST_TIMEOUT "HTTP timeout in seconds" "10"
	prompt_positive_integer FAILURE_THRESHOLD "Failures before alerting" "2"
	prompt_required SMTP_HOST "SMTP host"
	prompt_port SMTP_PORT "SMTP port" "587"
	prompt_required SMTP_USERNAME "SMTP username"
	while true; do
		read -r -s -p "SMTP password: " SMTP_PASSWORD
		echo
		if [ -n "$SMTP_PASSWORD" ]; then
			break
		fi
		echo "A password is required." >&2
	done
	while true; do
		prompt_required SMTP_TLS_MODE "SMTP TLS mode (starttls or implicit)" "starttls"
		case "$SMTP_TLS_MODE" in
			starttls|implicit) break ;;
			*) echo "Use 'starttls' or 'implicit'." >&2 ;;
		esac
	done
	prompt_positive_integer SMTP_TIMEOUT "SMTP timeout in seconds" "10"
	prompt_required ALERT_FROM "Alert sender address"
	prompt_required ALERT_TO "Alert recipient address"

	toml_escape "$HEALTH_URL"; HEALTH_URL_TOML=$REPLY
	toml_escape "$SMTP_HOST"; SMTP_HOST_TOML=$REPLY
	toml_escape "$SMTP_USERNAME"; SMTP_USERNAME_TOML=$REPLY
	toml_escape "$SMTP_PASSWORD"; SMTP_PASSWORD_TOML=$REPLY
	toml_escape "$ALERT_FROM"; ALERT_FROM_TOML=$REPLY
	toml_escape "$ALERT_TO"; ALERT_TO_TOML=$REPLY

	local temporary_config
	temporary_config=$(mktemp)
	trap 'rm -f "$temporary_config"' RETURN
	cat > "$temporary_config" <<EOF
[health]
url = "$HEALTH_URL_TOML"
interval_seconds = $CHECK_INTERVAL
timeout_seconds = $REQUEST_TIMEOUT
failure_threshold = $FAILURE_THRESHOLD

[state]
file = "/var/lib/biblizap-monitor/state"

[smtp]
host = "$SMTP_HOST_TOML"
port = $SMTP_PORT
username = "$SMTP_USERNAME_TOML"
password = "$SMTP_PASSWORD_TOML"
tls_mode = "$SMTP_TLS_MODE"
timeout_seconds = $SMTP_TIMEOUT

[alerts]
from = "$ALERT_FROM_TOML"
to = ["$ALERT_TO_TOML"]
EOF
	install -m 0600 -o "$SERVICE_USER" -g "$SERVICE_GROUP" \
		"$temporary_config" "$CONFIG_FILE"
	trap - RETURN
	rm -f "$temporary_config"
	echo "Wrote protected configuration to $CONFIG_FILE."
}

echo "Creating system user and group if needed..."
if ! getent group "$SERVICE_GROUP" >/dev/null; then
	groupadd --system "$SERVICE_GROUP"
fi
if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
	useradd --system --no-create-home --shell /usr/sbin/nologin \
		--gid "$SERVICE_GROUP" "$SERVICE_USER"
fi

install -m 0755 "$BINARY_SOURCE" "$BINARY_TARGET"
echo "Installed monitor binary to $BINARY_TARGET."

install -d -m 0700 -o "$SERVICE_USER" -g "$SERVICE_GROUP" "$CONFIG_DIR"

if [ -f "$CONFIG_FILE" ] && [ "$RECONFIGURE" = false ]; then
	echo "Keeping existing configuration at $CONFIG_FILE."
else
	if [ -f "$CONFIG_FILE" ]; then
		read -r -p "Replace existing configuration at $CONFIG_FILE? [y/N] " answer
		if [[ ! "$answer" =~ ^[Yy]$ ]]; then
			echo "Keeping existing configuration."
		else
			install -m 0600 -o "$SERVICE_USER" -g "$SERVICE_GROUP" \
				"$CONFIG_FILE" "$CONFIG_FILE.backup"
			echo "Backed up the existing configuration to $CONFIG_FILE.backup."
			write_configuration
		fi
	else
		write_configuration
	fi
fi

install -m 0644 "$SERVICE_SOURCE" "$SERVICE_TARGET"
echo "Installed systemd unit to $SERVICE_TARGET."

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"
sleep 1

if systemctl is-active --quiet "$SERVICE_NAME"; then
	echo "BibliZap monitor installed and running."
	echo "Logs: journalctl -u $SERVICE_NAME -f"
else
	echo "The monitor did not remain active. Current status:" >&2
	systemctl status --no-pager "$SERVICE_NAME" >&2 || true
	exit 1
fi
