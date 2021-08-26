#!/bin/sh

set -eu -o pipefail

chk_tools () {
    ok=true
    for i in $@; do
        if ! which "${i}" > /dev/null 2>&1; then
            ok=false
            echo "Tool ${i} does not exist in env PATH. Please install it." >&2
        fi
    done
    ${ok}
}

chk_envs () {
    ok=true
    for i in $@; do
        eval "x=\"\${${i}-~~~}\""
        if [ "${x}" = '~~~' ]; then
            ok=false
            echo "Environment variable ${i} needs to be set." >&2
        fi
    done
    ${ok}
}

get_envs () {
    ok=true
    for i in $@; do
        if ! (chk_envs "${i}" > /dev/null 2>&1 ||
            eval "$(amber print | grep " ${i}=")"); then
            ok=false
            echo "Environment variable ${i} needs to be set, or in Amber secrets." >&2
        fi
    done
    ${ok}
}

### main

# Due to awscli takes value of secret in commandline, and some secrets are in environments, both are easily to be observed. This script must be run in a trusted environment.

chk_tools 'amber' 'jq' 'aws'
chk_envs 'AMBER_SECRET' 'AWS_REGION'
get_envs 'AWS_ACCESS_KEY_ID' 'AWS_SECRET_ACCESS_KEY'

size="$(amber print --style json | jq -r '. | length')"

for i in $(seq "${size}"); do
    i="$(( i - 1 ))"
    aws secretsmanager create-secret --name "$(amber print --style json | jq -r ".[${i}].key")" --secret-string "$(amber print --style json | jq -r ".[${i}].value")"
done
