const path = require('path');
const { config } = require('@swc/core/spack')

module.exports = config({
  entry: {
    'main': __dirname + '/site/src/main.js',
  },
  output: {
    path: path.join(process.env.BUILD_TARGET, "assets", "scripts"),
  },
  module: {},
});
