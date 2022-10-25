import { yupResolver } from '@hookform/resolvers/yup';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';

import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';

interface Inputs {
  code: string;
}

const schema = yup
  .object()
  .shape({
    code: yup.string().required('Code is required.'),
  })
  .required();

//TOTP method
export const MFACode = () => {
  const navigate = useNavigate();
  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      code: '',
    },
  });

  const handleValidSubmit: SubmitHandler<Inputs> = (values) => {
    console.table(values);
  };
  return (
    <>
      <p>Use code from your authentication app and click button to proceed</p>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'code' }}
          required
          placeholder="Enter Authenticator code"
        />
        <Button
          text="Use authenticator code"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
        />
      </form>
      <nav>
        <span>or</span>
        <Button
          text="Use security key instead"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('../key')}
        />
        <Button
          text="Use your wallet instead"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('../wallet')}
        />
      </nav>
    </>
  );
};
