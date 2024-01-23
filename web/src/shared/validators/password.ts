import * as yup from 'yup';

import { TranslationFunctions } from '../../i18n/i18n-types';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
  patternSafePasswordCharacters,
} from '../patterns';

export const passwordValidator = (LL: TranslationFunctions) =>
  yup
    .string()
    .min(8, LL.form.error.minimumLength())
    .max(128, LL.form.error.maximumLength())
    .matches(patternAtLeastOneDigit, LL.form.error.oneDigit())
    .matches(patternAtLeastOneSpecialChar, LL.form.error.oneSpecial())
    .matches(patternAtLeastOneUpperCaseChar, LL.form.error.oneUppercase())
    .matches(patternAtLeastOneLowerCaseChar, LL.form.error.oneLowercase())
    .matches(patternSafePasswordCharacters, LL.form.error.forbiddenCharacter())
    .required(LL.form.error.required());
