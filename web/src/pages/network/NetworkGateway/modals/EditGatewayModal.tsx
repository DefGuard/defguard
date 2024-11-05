import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { trimObjectStrings } from '../../../../shared/utils/trimObjectStrings';
import { useEditGatewayModal } from './hooks/useEditGatewayModal';

interface Inputs {
  url: string;
}

export const EditGatewayModal = () => {
  const { LL } = useI18nContext();
  const {
    network: {
      gateway: { editGateway },
    },
  } = useApi();
  const [gateway, visible] = useEditGatewayModal(
    (state) => [state.gateway, state.visible],
    shallow,
  );

  const [close] = useEditGatewayModal((state) => [state.close], shallow);

  const zodSchema = useMemo(
    () =>
      z.object({
        url: z.string().url(),
      }),
    [],
  );

  const defaultValues = useMemo(() => {
    return {
      url: gateway?.url,
    };
  }, [gateway]);

  const {
    control,
    handleSubmit,
    setValue,
    getValues,
    reset,
    formState: { isValid },
  } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
    mode: 'all',
    defaultValues: defaultValues,
  });

  useEffect(() => {
    reset(defaultValues);
  }, [reset, defaultValues]);

  const onInvalidSubmit: SubmitErrorHandler<Inputs> = (values) => {
    const invalidFields = Object.keys(values) as (keyof Partial<Inputs>)[];
    const invalidFieldsValues = getValues(invalidFields);
    invalidFields.forEach((key, index) => {
      setValue(key, invalidFieldsValues[index], {
        shouldTouch: true,
        shouldValidate: true,
      });
    });
  };

  const queryClient = useQueryClient();
  const toaster = useToaster();

  const { mutate, isLoading: addGatewayLoading } = useMutation(
    [MutationKeys.ADD_GATEWAY],
    editGateway,
    {
      onSuccess: (_data, _variables) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_ALL_GATEWAYS]);
        toaster.success('Gateway changed successfully');
        close();
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    if (!gateway) return;
    values = trimObjectStrings(values);
    mutate({
      url: values.url,
      gatewayId: gateway.id,
    });
  };

  return (
    <ModalWithTitle
      id="add-gateway-modal"
      backdrop
      title={'Edit Gateway'}
      onClose={close}
      isOpen={visible}
    >
      <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
        <FormInput
          label={'Gateway URL'}
          controller={{ control, name: 'url' }}
          disabled={false}
          required
        />
        <div className="controls">
          <Button
            className="big primary"
            type="submit"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={'Save'}
            disabled={!isValid}
            loading={addGatewayLoading}
          />
          <Button
            size={ButtonSize.LARGE}
            text={LL.form.cancel()}
            className="cancel"
            onClick={() => close()}
            tabIndex={4}
            type="button"
            disabled={addGatewayLoading}
          />
        </div>
      </form>
    </ModalWithTitle>
  );
};
