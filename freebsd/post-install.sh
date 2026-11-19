#!/bin/sh
CONFIG=/etc/defguard/core.conf

if [ ! -f "${CONFIG}" ]; then
    cp "${CONFIG}.sample" "${CONFIG}"
fi
