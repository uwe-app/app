#!/usr/bin/env bash

ARGS=""

# WARN: esbuld --minify option is not stable enough yet,
# WARN: it will break the release build if you enable it!

#if [ $NODE_ENV == "production" ]; then
  #ARGS="--minify"
#fi

npx esbuild \
  ${BUILD_SOURCE}/src/main.jsx \
  --outfile=${BUILD_TARGET}/assets/scripts/main.js \
  --define:process.env.NODE_ENV="'production'" \
  --jsx-factory=h \
  --jsx-fragment=Fragment \
  $ARGS \
  --bundle
