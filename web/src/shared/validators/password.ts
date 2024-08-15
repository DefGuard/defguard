import { z } from 'zod';

import { TranslationFunctions } from '../../i18n/i18n-types';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
  patternSafePasswordCharacters,
} from '../patterns';

export const passwordValidator = (LL: TranslationFunctions) =>
  z
    .string()
    .min(1, LL.form.error.required())
    .min(8, LL.form.error.minimumLength())
    .max(128, LL.form.error.maximumLength())
    .regex(patternAtLeastOneDigit, LL.form.error.oneDigit())
    .regex(patternAtLeastOneSpecialChar, LL.form.error.oneSpecial())
    .regex(patternAtLeastOneUpperCaseChar, LL.form.error.oneUppercase())
    .regex(patternAtLeastOneLowerCaseChar, LL.form.error.oneLowercase())
    .regex(patternSafePasswordCharacters, LL.form.error.forbiddenCharacter());
