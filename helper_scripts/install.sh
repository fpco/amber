/bin/sh

set -eu -o pipefail

version_prefix=''
tool='fpco/amber'

tags_url="https://github.com/${tool}/tags"
tags="$(wget -O- "${tags_url}" | grep -o "/${tool}/releases/tag/[^\"]*")"
[ -z "${version_prefix}" ] && tag="$(echo "${tags}" | head -1)" || tag="$(echo "${tags}" | grep "/${version_prefix}" | head -1)"
target="https://github.com/$(wget -O- "https://github.com${tag}" | grep -o "/${tool}[^\"]*linux[^\"]*")"

wget -O /tmp/amber "${target}"
chmod a+x /tmp/amber

echo 'Amber has been downloaded to /tmp/amber.'
