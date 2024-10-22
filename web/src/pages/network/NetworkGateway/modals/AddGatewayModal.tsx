import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
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
import { useNetworkPageStore } from '../../hooks/useNetworkPageStore';
import { useAddGatewayModal } from './hooks/useAddGatewayModal';

interface Inputs {
  url: string;
}

export const AddGatewayModal = () => {
  const { LL } = useI18nContext();
  const {
    network: {
      gateway: { addGateway },
    },
  } = useApi();
  const [visible] = useAddGatewayModal(
    (state) => [state.visible],
    shallow,
  );
  const [reset, close] = useAddGatewayModal(
    (state) => [state.reset, state.close],
    shallow,
  );
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);

  const zodSchema = useMemo(
    () =>
      z.object({
        url: z.string().url(),
      }),
    [],
  );

  const {
    control,
    handleSubmit,
    setValue,
    getValues,
    formState: { isValid },
  } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
    mode: 'all',
    defaultValues: {
      url: '',
    },
  });

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
    addGateway,
    {
      onSuccess: (_data, _variables) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_ALL_GATEWAYS]);
        toaster.success('Gateway added successfully');
        close();
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    values = trimObjectStrings(values);
    mutate({
      url: values.url,
      networkId: selectedNetworkId,
    });
  };

  return (
    <ModalWithTitle
      id="add-gateway-modal"
      backdrop
      title={'Add Gateway'}
      onClose={close}
      afterClose={reset}
      isOpen={visible}
    >
      <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
        <FormInput
          label={'Gateway URL'}
          controller={{ control, name: 'url' }}
          disabled={false}
          required
          placeholder='e.g. http://127.0.0.1:50066/'
        />
        <div className="controls">
          <Button
            className="big primary"
            type="submit"
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={'Add Gateway'}
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
