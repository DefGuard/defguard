import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import { m } from '../../../../../../paraglide/messages';
import api from '../../../../../api/api';
import { Fold } from '../../../../../defguard-ui/components/Fold/Fold';
import { FoldButton } from '../../../../../defguard-ui/components/FoldButton/FoldButton';
import { Icon } from '../../../../../defguard-ui/components/Icon';
import { SizedBox } from '../../../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../defguard-ui/types';
import { useAddUserDeviceModal } from '../../store/useAddUserDeviceModal';
import { AddUserDeviceModalStep } from '../../types';
import clientImage from './assets/client.png';
import manualImage from './assets/manual.png';

export const AddDeviceModalStartStep = () => {
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const user = useAddUserDeviceModal((s) => s.user);

  const { mutate: startClientActivation, isPending } = useMutation({
    mutationFn: api.user.startClientActivation,
    onSuccess: ({ data }) => {
      useAddUserDeviceModal.setState({
        step: AddUserDeviceModalStep.ClientSetup,
        enrollment: {
          token: data.enrollment_token,
          url: data.enrollment_url,
        },
      });
    },
  });

  if (!user) return null;

  return (
    <div id="add-device-start-step">
      <div
        className="option"
        role="button"
        onClick={() => {
          if (isPending) return;
          startClientActivation({
            username: user.username,
            send_enrollment_notification: false,
          });
        }}
      >
        <div className="image">
          <img alt="option-image" src={clientImage} />
        </div>
        <div className="description">
          <p>{m.modal_add_user_device_start_client_title()}</p>
          <p>{m.modal_add_user_device_start_client_subtitle()}</p>
        </div>
        <Icon icon="arrow-small" rotationDirection="right" />
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <FoldButton
        open={advancedOpen}
        onChange={setAdvancedOpen}
        textClose={m.modal_add_user_device_hide_advanced()}
        textOpen={m.modal_add_user_device_show_advanced()}
      />
      <Fold open={advancedOpen}>
        <SizedBox height={ThemeSpacing.Md} />
        <div
          className="option"
          role="button"
          onClick={() => {
            useAddUserDeviceModal.setState({
              step: AddUserDeviceModalStep.ManualSetup,
            });
          }}
        >
          <div className="image">
            <img alt="option-image" src={manualImage} />
          </div>
          <div className="description">
            <p>{m.modal_add_user_device_start_manual_title()}</p>
            <p>{m.modal_add_user_device_start_manual_subtitle()}</p>
          </div>
          <Icon icon="arrow-small" rotationDirection="right" />
        </div>
      </Fold>
    </div>
  );
};
