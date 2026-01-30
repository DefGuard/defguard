import { createFormHook, createFormHookContexts } from '@tanstack/react-form';
import { FormSelectMultiple } from './components/FormSelectMultiple/FormSelectMultiple';
import { FormUploadField } from './components/FormUploadField/FormUploadField';
import { FormCheckbox } from './defguard-ui/components/form/FormCheckbox/FormCheckbox';
import { FormCheckboxGroup } from './defguard-ui/components/form/FormCheckboxGroup/FormCheckboxGroup';
import { FormInput } from './defguard-ui/components/form/FormInput/FormInput';
import { FormInteractiveBlock } from './defguard-ui/components/form/FormInteractiveBlock/FormInteractiveBlock';
import { FormRadio } from './defguard-ui/components/form/FormRadio/FormRadio';
import { FormSelect } from './defguard-ui/components/form/FormSelect/FormSelect';
import { FormSubmitButton } from './defguard-ui/components/form/FormSubmitButton/FormSubmitButton';
import { FormSuggestedIPInput } from './defguard-ui/components/form/FormSuggestedIPInput/FormSuggestedIPInput';
import { FormTextarea } from './defguard-ui/components/form/FormTextarea/FormTextarea';
import { FormToggle } from './defguard-ui/components/form/FormToggle/FormToggle';

export const { fieldContext, formContext, useFieldContext, useFormContext } =
  createFormHookContexts();

export const { useAppForm, withFieldGroup, withForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    FormTextarea,
    FormInput,
    FormSelect,
    FormCheckbox,
    FormRadio,
    FormToggle,
    FormSuggestedIPInput,
    FormSelectMultiple,
    FormInteractiveBlock,
    FormUploadField,
    FormCheckboxGroup,
  },
  formComponents: {
    FormSubmitButton,
  },
});
