#!/usr/bin/env bash
pip install mkdocs mkdocs-material mkdocs-git-revision-date-localized-plugin pymdown-extensions
mkdocs gh-deploy --force --no-history -f ../mkdocs.yml
