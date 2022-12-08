import { yupResolver } from '@hookform/resolvers/yup';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../shared/components/Form/FormToggle/FormToggle';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ToggleOption } from '../../../../../../shared/components/layout/Toggle/Toggle';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { patternValidWireguardKey } from '../../../../../../shared/patterns';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';

enum ChoiceEnum {
  AUTO_CONFIG = 1,
  MANUAL_CONFIG = 2,
}
interface FormValues {
  name: string;
  choice: ChoiceEnum;
  publicKey?: string;
}

const toggleOptions: ToggleOption<number>[] = [
  {
    text: 'Generate key pair',
    value: ChoiceEnum.AUTO_CONFIG,
  },
  {
    text: 'Use my own public key',
    value: ChoiceEnum.MANUAL_CONFIG,
  },
];

const schema = yup
  .object()
  .shape({
    choice: yup.number().required(),
    name: yup
      .string()
      .min(4, 'Min. 4 characters.')
      .required('Name is required.'),
    publicKey: yup.string().when('choice', {
      is: ChoiceEnum.MANUAL_CONFIG,
      then: (s) =>
        s
          .min(44, 'Key is invalid.')
          .max(44, 'Key is invalid.')
          .required('Key is required.')
          .matches(patternValidWireguardKey, 'Key is invalid.'),
      otherwise: (s) => s.optional(),
    }),
  })
  .required();

export const SetupStep = () => {
  const toaster = useToaster();
  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<FormValues>({
    defaultValues: {
      name: '',
      choice: ChoiceEnum.AUTO_CONFIG,
      publicKey: '',
    },
    resolver: yupResolver(schema),
  });

  const validSubmitHandler: SubmitHandler<FormValues> = (values) => {
    console.table(values);
    toaster.success('Form Valid.');
    if (values.choice === ChoiceEnum.AUTO_CONFIG) {
      const keys = generateWGKeys();
      console.table(keys);
    }
  };

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'choice' });

  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        <p>
          You need to configure WireguardVPN on your device, please visit{' '}
          <a href="">documentation</a> if you don&apos;t know how to do it.
        </p>
      </MessageBox>
      <form onSubmit={handleSubmit(validSubmitHandler)}>
        <FormInput
          outerLabel="Device Name"
          controller={{ control, name: 'name' }}
        />
        <FormToggle
          options={toggleOptions}
          controller={{ control, name: 'choice' }}
        />
        <FormInput
          outerLabel="Provide Your Public Key"
          controller={{ control, name: 'publicKey' }}
          disabled={choiceValue === ChoiceEnum.AUTO_CONFIG}
        />
        <div className="controls">
          <Button
            className="cancel"
            type="button"
            text="Cancel"
            styleVariant={ButtonStyleVariant.STANDARD}
            size={ButtonSize.BIG}
          />
          <Button
            type="submit"
            text="Generate Config"
            styleVariant={ButtonStyleVariant.PRIMARY}
            size={ButtonSize.BIG}
            disabled={!isValid}
          />
        </div>
      </form>
    </>
  );
};
