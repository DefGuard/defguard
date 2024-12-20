import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
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
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
import { StandaloneDeviceModalForm } from '../../../components/StandaloneDeviceModalForm';
import { StandaloneDeviceModalFormMode } from '../../../components/types';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalStep,
  WGConfigGenChoice,
} from '../../types';

export const SetupCliStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.setup;
  const [formLoading, setFormLoading] = useState(false);
  const queryClient = useQueryClient();
  const [setState, close, next, submitSubject] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.close, s.changeStep, s.submitSubject],
    shallow,
  );

  const toast = useToaster();

  const {
    standaloneDevice: { createCliDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: createCliDevice,
    onSuccess: () => {
      toast.success(LL.modals.addStandaloneDevice.toasts.deviceCreated());
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      });
    },
    onError: (e) => {
      toast.error(LL.modals.addStandaloneDevice.toasts.creationFailed());
      console.error(e);
    },
  });

  const [initIp, locationOptions] = useAddStandaloneDeviceModal(
    (s) => [s.initAvailableIp, s.networkOptions],
    shallow,
  );

  const defaultValues = useMemo(() => {
    if (initIp && locationOptions) {
      const res: AddStandaloneDeviceFormFields = {
        assigned_ip: initIp,
        generationChoice: WGConfigGenChoice.AUTO,
        location_id: locationOptions[0].value,
        name: '',
        wireguard_pubkey: '',
        description: '',
      };
      return res;
    }
    return undefined;
  }, [initIp, locationOptions]);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      const response = await mutateAsync({
        assigned_ip: values.assigned_ip,
        location_id: values.location_id,
        name: values.name,
        description: values.description,
      });
      setState({ enrollResponse: response });
      next(AddStandaloneDeviceModalStep.FINISH_CLI);
    },
    [mutateAsync, next, setState],
  );

  if (initIp === undefined || defaultValues === undefined) return null;

  return (
    <div className="setup-cli-step">
      <MessageBox
        type={MessageBoxType.INFO}
        message={localLL.stepMessage()}
        dismissId="add-standalone-device-cli-setup-step-header"
      />
      <StandaloneDeviceModalForm
        locationOptions={locationOptions}
        defaults={defaultValues}
        onLoadingChange={setFormLoading}
        onSubmit={handleSubmit}
        mode={StandaloneDeviceModalFormMode.CREATE_CLI}
        submitSubject={submitSubject}
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
