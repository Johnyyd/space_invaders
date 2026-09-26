#!/bin/bash
# Wrapper to launch the Python-based Zero-Latency Terminal Client
cd "$(dirname "$0")" || exit 1
python3 play.py "$@"
