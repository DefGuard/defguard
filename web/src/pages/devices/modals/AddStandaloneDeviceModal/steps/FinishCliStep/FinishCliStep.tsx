import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { StandaloneDeviceModalEnrollmentContent } from '../../../components/StandaloneDeviceModalEnrollmentContent/StandaloneDeviceModalEnrollmentContent';
import { useAddStandaloneDeviceModal } from '../../store';

export const FinishCliStep = () => {
  const { LL } = useI18nContext();
  const [closeModal] = useAddStandaloneDeviceModal((s) => [s.close], shallow);
  const enroll = useAddStandaloneDeviceModal((s) => s.enrollResponse);

  if (!enroll) return null;
  return (
    <div className="finish-cli-step">
      <StandaloneDeviceModalEnrollmentContent enrollmentData={enroll} />
      <div className="controls solo">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.close()}
          onClick={() => {
            closeModal();
          }}
        />
      </div>
    </div>
  );
};
