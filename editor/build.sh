#!/usr/bin/env bash

ARGS=""

if [ $NODE_ENV == "production" ]; then
  ARGS="--minify"
fi

npx esbuild \
  ${BUILD_SOURCE}/src/main.jsx \
  --outfile=${BUILD_TARGET}/assets/scripts/main.js \
  --define:process.env.NODE_ENV="'production'" \
  --jsx-factory=h \
  --jsx-fragment=Fragment \
  $ARGS \
  --bundle
