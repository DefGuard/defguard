import type { BaseTranslation } from '../i18n-types';

const en: BaseTranslation = {
  navigation: {
    bar: {
      overview: 'Overview',
      users: 'Users',
      provisioners: 'Provisioners',
      webhooks: 'Webhooks',
      openId: 'OpenID Apps',
      myProfile: 'My Profile',
      settings: 'Settings',
      logOut: 'Log out',
    },
    mobileTitles: {
      users: 'Users',
      settings: 'Defguard Global Settings',
      user: 'User Profile',
      provisioners: 'Provisioners',
      webhooks: 'Webhooks',
      openId: 'OpenId Apps',
      overview: 'Network Overview',
      networkSettings: 'Network Settings',
    },
    copyright: 'Copyright \u00A9 2023',
    version: 'Application version: {version: string}',
  },
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
