#!/usr/bin/env sh

#echo "Running the book ${BUILD_SOURCE}";
#echo "Running the book ${BUILD_TARGET}";

npx esbuild \
  ${BUILD_SOURCE}/src/main.jsx \
  --outfile=${BUILD_TARGET}/assets/scripts/main.js \
  --define:process.env.NODE_ENV="'production'" \
  --jsx-factory=h \
  --jsx-fragment=Fragment \
  --bundle
