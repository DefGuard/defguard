import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import React from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';
import * as yup from 'yup';

import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import SvgIconArrowGrayLeft from '../../../shared/components/svg/IconArrowGrayLeft';

interface Inputs {
  username: string;
  email: string;
  confirmEmail: string;
}

const Register: React.FC = () => {
  const { t } = useTranslation('en');
  const schema = yup
    .object({
      username: yup
        .string()
        .required(t('auth.register.form.required.username')),
      email: yup.string().required(t('auth.register.form.required.email')),
      confirmEmail: yup
        .string()
        .required(t('auth.register.form.required.confirmEmail')),
    })
    .required();
  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const onSubmit: SubmitHandler<Inputs> = (data) => console.table(data);
  const navigate = useNavigate();
  return (
    <section id="register-container">
      <div className="login-link">
        <div onClick={() => navigate('../login')}>
          <button className="icon-button nav">
            <SvgIconArrowGrayLeft />
          </button>
          <p>{t('auth.register.login-link')}</p>
        </div>
      </div>
      <section className="content">
        <h1>{t('auth.register.template.header')}</h1>
        <form onSubmit={handleSubmit(onSubmit)}>
          <div className="errors-container"></div>
          <FormInput
            controller={{ control, name: 'username' }}
            required
            innerLabel
            placeholder={t('auth.register.form.placeholder.username')}
          />
          <FormInput
            controller={{ control, name: 'email' }}
            required
            innerLabel
            placeholder={t('auth.register.form.placeholder.email')}
          />
          <FormInput
            controller={{ control, name: 'confirmEmail' }}
            required
            innerLabel
            placeholder={t('auth.register.form.placeholder.confirmEmail')}
          />
          <Button
            type="submit"
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={t('auth.register.form.template.submit')}
            onClick={() => navigate('/wizard')}
          />
        </form>
      </section>
    </section>
  );
};

export default Register;
