import './style.scss';

import { useCallback, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { StandaloneDeviceModalForm } from '../../components/StandaloneDeviceModalForm';
import { useAddStandaloneDeviceModal } from '../../store';
import {
  AddStandaloneDeviceFormFields,
  AddStandaloneDeviceModalChoice,
  AddStandaloneDeviceModalStep,
} from '../../types';

export const SetupManualStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.setup;
  const [formLoading, setFormLoading] = useState(false);
  const [setState, next, submitSubject] = useAddStandaloneDeviceModal(
    (s) => [s.setStore, s.changeStep, s.submitSubject],
    shallow,
  );
  const initialIp = useAddStandaloneDeviceModal((s) => s.initAvailableIp);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      console.table(values);
      next(AddStandaloneDeviceModalStep.FINISH_MANUAL);
    },
    [next],
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
