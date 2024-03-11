#!/bin/bash

# Usage: ./push_script.sh "Commit message"


if [ -z "$1" ]; then
    echo "Please provide commit message."
    exit 1
fi


git add .
git commit -m "$1"
git push 

latest_commit_hash=$(git rev-parse HEAD)
git tag -f latest "$latest_commit_hash"
git push --force --tags

