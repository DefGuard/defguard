import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../../shared/queries';
import { StandaloneDeviceModalForm } from '../../components/StandaloneDeviceModalForm';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalChoice,
} from '../../types';

export const SetupCliStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.setup;
  const [formLoading, setFormLoading] = useState(false);
  const queryClient = useQueryClient();
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [setState, close, next, submitSubject] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.close, s.changeStep, s.submitSubject],
    shallow,
  );

  const {
    standaloneDevice: { createDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: createDevice,
    onSuccess: (resp) => {
      console.table(resp);
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      });
      // next(AddStandaloneDeviceModalStep.FINISH_CLI);
    },
  });

  const initIp = useAddStandaloneDeviceModal((s) => s.initAvailableIp);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      console.table(values);
      await mutateAsync({
        assigned_ip: values.assigned_ip,
        location_id: values.location_id,
        name: values.name,
      });
    },
    [mutateAsync],
  );

  if (initIp === undefined) return null;

  return (
    <div className="setup-cli-step">
      <MessageBox
        type={MessageBoxType.INFO}
        message={
          // eslint-disable-next-line max-len
          'Here you can add definitions or generate configurations for devices that can connect to your VPN. Only locations without Multi-Factor Authentication are available here, as MFA is only supported in Defguard Desktop Client for now.'
        }
        dismissId="add-standalone-device-cli-setup-step-header"
      />
      <StandaloneDeviceModalForm
        initialAssignedIp={initIp}
        mode={AddStandaloneDeviceModalChoice.CLI}
        onLoadingChange={setFormLoading}
        onSubmit={handleSubmit}
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
