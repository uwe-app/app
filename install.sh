set -e;

fatal() {
  echo "$1" 2>&1;
  exit 1;
}

BASE="${HOME}/.uwe";
BIN="${BASE}/bin";

OS="`uname`"
case $OS in
  'Linux')
    OS='linux'
    ;;
  'Darwin') 
    OS='macos'
    ;;
  'FreeBSD')
    fatal "Operating system FreeBSD is not supported, sorry."
    ;;
  'WindowsNT')
    fatal "Operating system Windows is not supported, sorry."
    ;;
  'SunOS')
    fatal "Operating system Solaris is not supported, sorry."
    ;;
  'AIX') ;;
  *) ;;
esac

command -v curl > /dev/null || fatal "Curl is required to use the install.sh script";
mkdir -p "${BIN}" || fatal "Could not create bin directory ${BIN}";

URL="https://release.uwe.app/latest/${OS}/uvm";
echo "Download ${URL}";

(cd "${BIN}" && curl -OL --fail --progress-bar "${URL}" || fatal "Download failed")
(cd "${BIN}" && chmod +x ./uvm update && ./uvm update)
