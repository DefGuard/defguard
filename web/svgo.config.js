module.exports = {
  multipass: true, // boolean. false by default
  datauri: 'unenc', // 'base64' (default), 'enc' or 'unenc'.
  js2svg: {
    indent: 2, // string with spaces or number of spaces. 4 by default
    pretty: true, // boolean, false by default
  },
  plugins: [
    // set of built-in plugins enabled by default
    {
      name: 'preset-default',
      params: {
        overrides: {
          inlineStyles: false,
          mergeStyles: false,
        },
      },
    },
    // enable built-in plugins by name
    'prefixIds',
    // or by expanded notation which allows to configure plugin
  ],
};
