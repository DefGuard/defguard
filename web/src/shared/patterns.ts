export const patternNoSpecialChars = /^\w+$/;

export const patternDigitOrLowercase = /^[0-9a-z]+$/g;

export const patternValidEmail =
  /[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?/g;

export const patternAtLeastOneUpperCaseChar = /(?=.*?[A-Z])/g;

export const patternAtLeastOneLowerCaseChar = /(?=.*?[a-z])/g;

export const patternAtLeastOneDigit = /(?=.*?[0-9])/g;

export const patternAtLeastOneSpecialChar = /(?=.*?[#?!@$%^&*-])/g;

export const patternValidPhoneNumber =
  /^\s*(?:\+?(\d{1,3}))?([-. (]*(\d{3})[-. )]*)?((\d{3})[-. ]*(\d{2,4})(?:[-.x ]*(\d+))?)\s*$/g;

export const patternValidWireguardKey =
  /^[A-Za-z0-9+/]{42}[A|E|I|M|Q|U|Y|c|g|k|o|s|w|4|8|0]=$/;

export const patternBaseUrl = /:\/\/(.[^/]+)/;

export const patternValidUrl = /^http:\/\/\w+(\.\w+)*(:[0-9]+)?\/?(\/[.\w]*)*$/;

