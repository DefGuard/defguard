import { createFormHook, createFormHookContexts } from '@tanstack/react-form';
import { FormSelectMultiple } from './components/FormSelectMultiple/FormSelectMultiple';
import { FormCheckbox } from './defguard-ui/components/form/FormCheckbox/FormCheckbox';
import { FormInput } from './defguard-ui/components/form/FormInput/FormInput';
import { FormRadio } from './defguard-ui/components/form/FormRadio/FormRadio';
import { FormSubmitButton } from './defguard-ui/components/form/FormSubmitButton/FormSubmitButton';
import { FormSuggestedIPInput } from './defguard-ui/components/form/FormSuggestedIPInput/FormSuggestedIPInput';
import { FormToggle } from './defguard-ui/components/form/FormToggle/FormToggle';

export const { fieldContext, formContext, useFieldContext, useFormContext } =
  createFormHookContexts();

export const { useAppForm, withFieldGroup, withForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    FormInput,
    FormCheckbox,
    FormRadio,
    FormToggle,
    FormSuggestedIPInput,
    FormSelectMultiple,
  },
  formComponents: {
    FormSubmitButton,
  },
});
