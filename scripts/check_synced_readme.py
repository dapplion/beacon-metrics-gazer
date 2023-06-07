#!/bin/bash
set -e

FILE="./README.md"

if [ -z "$(git diff $FILE)" ]; then 
  echo "Working directory clean"
else 
  echo "README file not clean"
  exit 1
fi

./scripts/sync_readme.py

if [ -z "$(git diff $FILE)" ]; then 
  echo "README is up to date"
else 
  git diff $FILE
  echo "README not synced"
  exit 1
fi



