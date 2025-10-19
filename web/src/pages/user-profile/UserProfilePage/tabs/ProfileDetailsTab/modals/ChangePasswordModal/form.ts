import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';

const PasswordErrorCode = {
  Number: 'password_form_check_number',
  Special: 'password_form_check_special',
  Lowercase: 'password_form_check_lowercase',
  Uppercase: 'password_form_check_uppercase',
  Minimum: 'password_form_check_minimum',
} as const;

const errorCodes = Object.values(PasswordErrorCode);

export type PasswordErrorCodeValue =
  (typeof PasswordErrorCode)[keyof typeof PasswordErrorCode];

const errorIsCustomCode = (value: string): value is PasswordErrorCodeValue => {
  return (errorCodes as readonly string[]).includes(value);
};

export const mapPasswordFieldError = (errorValue: string): string => {
  if (errorIsCustomCode(errorValue)) {
    return m.password_form_special_error();
  }
  return errorValue;
};

const hasNumber = /[0-9]/;

const hasUppercase = /[A-Z]/;

const hasLowercase = /[a-z]/;

const hasSpecialChar = /[^a-zA-Z0-9]/;

const createSchema = (isAdmin: boolean) =>
  z
    .object({
      current: z
        .string()
        .trim()
        .refine(
          (current) => {
            if (isAdmin) {
              return true;
            }
            return current.length > 0;
          },
          {
            error: m.form_error_required(),
          },
        ),
      password: z.string().trim().min(1, m.form_error_required()),
      repeat: z
        .string()
        .trim()
        .refine(
          (repeatValue) => {
            if (isAdmin) {
              return true;
            }
            return repeatValue.length > 0;
          },
          {
            error: m.form_error_required(),
          },
        ),
    })
    .superRefine(({ password, repeat }, ctx) => {
      if (password.length < 8) {
        ctx.addIssue({
          message: PasswordErrorCode.Minimum,
          code: 'custom',
          path: ['password'],
          continue: true,
        });
      }
      if (!hasNumber.test(password)) {
        ctx.addIssue({
          message: PasswordErrorCode.Number,
          code: 'custom',
          path: ['password'],
          continue: true,
        });
      }
      if (!hasUppercase.test(password)) {
        ctx.addIssue({
          message: PasswordErrorCode.Uppercase,
          code: 'custom',
          path: ['password'],
          continue: true,
        });
      }
      if (!hasLowercase.test(password)) {
        ctx.addIssue({
          message: PasswordErrorCode.Lowercase,
          code: 'custom',
          continue: true,
          path: ['password'],
        });
      }
      if (!hasSpecialChar.test(password)) {
        ctx.addIssue({
          message: PasswordErrorCode.Special,
          code: 'custom',
          continue: true,
          path: ['password'],
        });
      }
      if (repeat.length && repeat !== password && !isAdmin) {
        ctx.addIssue({
          message: m.password_form_check_repeat_match(),
          code: 'custom',
          path: ['repeat'],
          continue: true,
        });
      }
    });

export const userChangePasswordSchema = createSchema(false);

export const adminChangePasswordSchema = createSchema(true);

type UserChangePasswordFormFields = z.infer<typeof userChangePasswordSchema>;

type AdminChangePasswordFormFields = z.infer<typeof adminChangePasswordSchema>;

export const userChangePasswordDefaultValues: UserChangePasswordFormFields = {
  current: '',
  password: '',
  repeat: '',
};

export const adminChangePasswordDefaultValues: AdminChangePasswordFormFields = {
  current: '',
  password: '',
  repeat: '',
};
