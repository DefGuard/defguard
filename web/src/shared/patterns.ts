/* eslint-disable no-useless-escape */

export const patternNoSpecialChars = /^\w+$/;

export const patternDigitOrLowercase = /^[0-9a-z]+$/g;

export const patternValidEmail =
  // eslint-disable-next-line max-len
  /[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?/g;

export const patternAtLeastOneUpperCaseChar = /(?=.*?[A-Z])/g;

export const patternAtLeastOneLowerCaseChar = /(?=.*?[a-z])/g;

export const patternAtLeastOneDigit = /(?=.*?[0-9])/g;

export const patternStartsWithDigit = /^\d/;

export const patternAtLeastOneSpecialChar = /(?=.*?[#?!@$%^&*-])/g;

export const patternValidPhoneNumber =
  /^(\+?\d{1,3}\s?)?(\(\d{1,3}\)|\d{1,3})[-\s]?\d{1,4}[-\s]?\d{1,4}?$/;

export const patternValidWireguardKey =
  /^[A-Za-z0-9+/]{42}[A|E|I|M|Q|U|Y|c|g|k|o|s|w|4|8|0]=$/;

export const patternBaseUrl = /:\/\/(.[^/]+)/;

export const patternNumbersOnly = new RegExp('^[0-9]+$');

// https://gist.github.com/dperini/729294
export const patternValidUrl = new RegExp(
  '^' +
    // protocol identifier (optional)
    // short syntax // still required
    '(?:(?:(?:https?):)?\\/\\/)' +
    // user:pass BasicAuth (optional)
    '(?:\\S+(?::\\S*)?@)?' +
    '(?:' +
    // IP address dotted notation octets
    // excludes loopback network 0.0.0.0
    // excludes reserved space >= 224.0.0.0
    // excludes network & broadcast addresses
    // (first & last IP address of each class)
    '(?:[1-9]\\d?|1\\d\\d|2[01]\\d|22[0-3])' +
    '(?:\\.(?:1?\\d{1,2}|2[0-4]\\d|25[0-5])){2}' +
    '(?:\\.(?:[1-9]\\d?|1\\d\\d|2[0-4]\\d|25[0-4]))' +
    '|' +
    // host & domain names, may end with dot
    // can be replaced by a shortest alternative
    // (?![-_])(?:[-\\w\\u00a1-\\uffff]{0,63}[^-_]\\.)+
    '(?:' +
    '(?:' +
    '[a-z0-9\\u00a1-\\uffff]' +
    '[a-z0-9\\u00a1-\\uffff_-]{0,62}' +
    ')?' +
    '[a-z0-9\\u00a1-\\uffff]\\.' +
    ')+' +
    // TLD identifier name, may end with dot
    '(?:[a-z\\u00a1-\\uffff]{2,}\\.?)' +
    ')' +
    // port number (optional)
    '(?::\\d{2,5})?' +
    // resource path (optional)
    '(?:[/?#]\\S*)?' +
    '$',
  'i',
);

export const patternValidDomain =
  /^(?:(?:(?:[a-zA-z\-]+)\:\/{1,3})?(?:[a-zA-Z0-9])(?:[a-zA-Z0-9\-\.]){1,61}(?:\.[a-zA-Z]{2,})+|\[(?:(?:(?:[a-fA-F0-9]){1,4})(?::(?:[a-fA-F0-9]){1,4}){7}|::1|::)\]|(?:(?:[0-9]{1,3})(?:\.[0-9]{1,3}){3}))(?:\:[0-9]{1,5})?$/;

export const patternValidIp =
  /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
