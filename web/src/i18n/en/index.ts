import type { BaseTranslation } from '../i18n-types';

const en: BaseTranslation = {
  // TODO: your translations go here
  HI: 'Hi {name:string}! Please leave a star if you like this project: https://github.com/ivanhofer/typesafe-i18n',
  form: {
    submit: 'Submit',
    cancel: 'Cancel',
    close: 'Close',
    placeholders: {
      password: 'Password',
      username: 'Username',
    },
    error: {
      usernameTaken: 'Username is already in use',
      invalidKey: 'Key is invalid.',
      invalid: 'Field is invalid.',
      required: 'Field is required.',
      maximumLength: 'Maximum length exceeded.',
      minimumLength: 'Minimum length not reached.',
      noSpecialChars: 'No special characters are allowed.',
      oneDigit: 'One digit required.',
      oneSpecial: 'Special character required.',
      oneUppercase: 'One uppercase character required.',
      oneLowercase: 'One lowercase character required.',
    },
  },
};

export default en;
