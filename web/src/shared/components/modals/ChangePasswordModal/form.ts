import z from 'zod';
import { m } from '../../../../paraglide/messages';

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

export const mapPasswordFieldError = (
  errorValue: string,
  displayCustomError: boolean = false,
): string => {
  if (errorIsCustomCode(errorValue)) {
    if (displayCustomError) {
      return m[errorValue]();
    }
    return m.password_form_special_error();
  }
  return errorValue;
};

const hasNumber = /[0-9]/;

const hasUppercase = /[A-Z]/;

const hasLowercase = /[a-z]/;

const hasSpecialChar = /[^a-zA-Z0-9]/;

export const refinePasswordField = (password: string): string[] => {
  const issues: string[] = [];
  if (password.length < 8) {
    issues.push(PasswordErrorCode.Minimum);
  }
  if (!hasNumber.test(password)) {
    issues.push(PasswordErrorCode.Number);
  }
  if (!hasUppercase.test(password)) {
    issues.push(PasswordErrorCode.Uppercase);
  }
  if (!hasLowercase.test(password)) {
    issues.push(PasswordErrorCode.Lowercase);
  }
  if (!hasSpecialChar.test(password)) {
    issues.push(PasswordErrorCode.Special);
  }
  return issues;
};

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
      const passwordIssues = refinePasswordField(password);
      for (const issue of passwordIssues) {
        ctx.addIssue({
          message: issue,
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
