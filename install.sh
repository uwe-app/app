set -e;

fatal() {
  echo "$1" 2>&1;
  exit 1;
}

BASE="${HOME}/.uwe";
BIN="${BASE}/bin";
UVM="https://release.uwe.app/latest/uvm";

command -v curl > /dev/null || fatal "Curl is required to use the install.sh script";

mkdir -p "${BIN}"

(cd "${BIN}" && curl -OL --progress-bar "${UVM}" && chmod +x ./uvm && ./uvm)
