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
import { useAppStore } from '../../../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../../../shared/hooks/store/useAuthStore';
import { useEnterpriseUpgradeStore } from '../../../../../../shared/hooks/store/useEnterpriseUpgradeStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
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

export const SetupCliStep = () => {
  const [{ reservedDeviceNames }] = useDevicesPage();
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.setup;
  const [formLoading, setFormLoading] = useState(false);
  const queryClient = useQueryClient();
  const [setState, close, next, submitSubject] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.close, s.changeStep, s.submitSubject],
    shallow,
  );
  const currentUserId = useAuthStore((s) => s.user?.id);

  const toast = useToaster();

  const {
    standaloneDevice: { createCliDevice },
  } = useApi();

  const showUpgradeToast = useEnterpriseUpgradeStore((s) => s.show);
  const { getAppInfo } = useApi();
  const setAppStore = useAppStore((s) => s.setState, shallow);

  const { mutateAsync } = useMutation({
    mutationFn: createCliDevice,
    onSuccess: () => {
      toast.success(LL.modals.addStandaloneDevice.toasts.deviceCreated());
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_USER_PROFILE, currentUserId],
        [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      ]);
      void getAppInfo().then((response) => {
        setAppStore({
          appInfo: response,
        });
        if (response.license_info.any_limit_exceeded) {
          showUpgradeToast();
        }
      });
    },
    onError: (e) => {
      toast.error(LL.modals.addStandaloneDevice.toasts.creationFailed());
      console.error(e);
    },
  });

  const [initIpResponse, locationOptions] = useAddStandaloneDeviceModal(
    (s) => [s.initLocationIpResponse, s.networkOptions],
    shallow,
  );

  const defaultValues = useMemo(() => {
    if (initIpResponse && locationOptions) {
      const res: AddStandaloneDeviceFormFields = {
        modifiableIpPart: initIpResponse.modifiable_part,
        generationChoice: WGConfigGenChoice.AUTO,
        location_id: locationOptions[0].value,
        name: '',
        wireguard_pubkey: '',
        description: '',
      };
      return res;
    }
    return undefined;
  }, [initIpResponse, locationOptions]);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      const response = await mutateAsync({
        assigned_ips: values.modifiableIpPart,
        location_id: values.location_id,
        name: values.name,
        description: values.description,
      });
      setState({ enrollResponse: response });
      next(AddStandaloneDeviceModalStep.FINISH_CLI);
    },
    [mutateAsync, next, setState],
  );

  if (initIpResponse === undefined || defaultValues === undefined) return null;

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
        reservedNames={reservedDeviceNames}
        initialIpRecommendation={initIpResponse}
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
