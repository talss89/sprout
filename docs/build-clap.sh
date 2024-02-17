#! /bin/bash

cd "$(dirname "$0")"
cd ../
(echo "---" && echo "title: Command Reference" && echo "description: Sprout command reference" && echo "---") | (cat && cargo run --features markdown-docs -- | sed -e '1,/## `sprout`/d') > ./docs/src/content/docs/reference/command-reference.mdx