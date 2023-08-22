#!/usr/bin/env bash
pip install mkdocs mkdocs-material
mkdocs gh-deploy --force --no-history -f ../mkdocs.yml
