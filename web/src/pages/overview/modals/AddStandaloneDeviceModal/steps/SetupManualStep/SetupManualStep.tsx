import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../../shared/queries';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { StandaloneDeviceModalForm } from '../../components/StandaloneDeviceModalForm';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalChoice,
  AddStandaloneDeviceModalStep,
  WGConfigGenChoice,
} from '../../types';

export const SetupManualStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.setup;
  const [formLoading, setFormLoading] = useState(false);
  const [setState, next, submitSubject, close] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.changeStep, s.submitSubject, s.close],
    shallow,
  );
  const initialIp = useAddStandaloneDeviceModal((s) => s.initAvailableIp);

  const queryClient = useQueryClient();

  const {
    standaloneDevice: { createDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: createDevice,
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      });
    },
  });

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      let pub = values.wireguard_pubkey;
      if (values.generationChoice === WGConfigGenChoice.AUTO) {
        const keys = generateWGKeys();
        pub = keys.publicKey;
        setState({
          genKeys: keys,
        });
      }
      const response = await mutateAsync({
        assigned_ip: values.assigned_ip,
        location_id: values.location_id,
        name: values.name,
        description: values.description,
        wireguard_pubkey: pub,
      });
      setState({
        genChoice: values.generationChoice,
        manualResponse: response,
      });
      next(AddStandaloneDeviceModalStep.FINISH_MANUAL);
    },
    [mutateAsync, next, setState],
  );

  if (initialIp === undefined) return null;

  return (
    <div className="setup-manual">
      <StandaloneDeviceModalForm
        onSubmit={handleSubmit}
        onLoadingChange={setFormLoading}
        initialAssignedIp={initialIp}
        mode={AddStandaloneDeviceModalChoice.MANUAL}
      />
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
          onClick={() => close()}
          size={ButtonSize.LARGE}
          type="button"
        />
        <Button
          loading={formLoading}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={localLL.form.submit()}
          onClick={() => {
            submitSubject.next();
          }}
          type="submit"
        />
      </div>
    </div>
  );
};
