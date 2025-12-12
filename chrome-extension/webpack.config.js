const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  mode: 'production',
  entry: {
    'background': './background.js',
    'popup': './popup.js',
    'content': './content.js',
    'content-clipboard': './content-clipboard.js',
    'config-manager': './config-manager.js',
    'dialogs': './ui/dialogs.js'
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].js',
    clean: true
  },
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'manifest.json', to: 'manifest.json' },
        { from: 'popup.html', to: 'popup.html' },
        { from: 'popup.css', to: 'popup.css' },
        { from: 'ui/dialogs.css', to: 'ui/dialogs.css' },
        { from: 'config/special-domains-default.json', to: 'config/special-domains-default.json' },
        { from: 'images', to: 'images' }
      ]
    })
  ],
  resolve: {
    extensions: ['.js']
  }
};

