import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../../shared/queries';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { invalidateMultipleQueries } from '../../../../../../shared/utils/invalidateMultipleQueries';
import { useDevicesPage } from '../../../../hooks/useDevicesPage';
import { StandaloneDeviceModalForm } from '../../../components/StandaloneDeviceModalForm/StandaloneDeviceModalForm';
import { StandaloneDeviceModalFormMode } from '../../../components/types';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalStep,
  WGConfigGenChoice,
} from '../../types';

export const SetupManualStep = () => {
  const { LL } = useI18nContext();
  const [formLoading, setFormLoading] = useState(false);
  const [setState, next, submitSubject, close] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.changeStep, s.submitSubject, s.close],
    shallow,
  );
  const [initialIpResponse, locationOptions] = useAddStandaloneDeviceModal(
    (s) => [s.initLocationIpResponse, s.networkOptions],
    shallow,
  );

  const queryClient = useQueryClient();

  const currentUserId = useAuthStore((s) => s.user?.id);

  const [{ reservedDeviceNames }] = useDevicesPage();

  const {
    standaloneDevice: { createManualDevice: createDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: createDevice,
    onSuccess: () => {
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_USER_PROFILE, currentUserId],
        [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      ]);
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
        assigned_ip: values.modifiableIpPart,
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

  const defaultFormValues = useMemo(() => {
    if (locationOptions && initialIpResponse) {
      const res: AddStandaloneDeviceFormFields = {
        modifiableIpPart: initialIpResponse.modifiable_part,
        generationChoice: WGConfigGenChoice.AUTO,
        location_id: locationOptions[0].value,
        name: '',
        wireguard_pubkey: '',
        description: '',
      };
      return res;
    }
    return undefined;
  }, [initialIpResponse, locationOptions]);

  if (initialIpResponse === undefined || defaultFormValues === undefined) return null;

  return (
    <div className="setup-manual">
      <StandaloneDeviceModalForm
        defaults={defaultFormValues}
        locationOptions={locationOptions}
        mode={StandaloneDeviceModalFormMode.CREATE_MANUAL}
        submitSubject={submitSubject}
        onSubmit={handleSubmit}
        onLoadingChange={setFormLoading}
        reservedNames={reservedDeviceNames}
        initialIpRecommendation={initialIpResponse}
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
          text={LL.modals.addStandaloneDevice.form.submit()}
          onClick={() => {
            submitSubject.next();
          }}
          type="submit"
        />
      </div>
    </div>
  );
};
