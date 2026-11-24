import './style.scss';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { DeviceOverview } from '../../../../shared/api/types';
import { CodeRow } from '../../../../shared/components/CodeRow/CodeRow';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';
import type { MenuItemsGroup } from '../../../../shared/defguard-ui/components/Menu/types';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenDeviceVerificationModal } from '../../../../shared/hooks/modalControls/types';
import { useTimer } from '../../../../shared/hooks/useTimer';

const ScreenVariant = {
  Default: 'default',
  Block: 'block',
  Delete: 'delete',
} as const;

type ScreenVariantValue = (typeof ScreenVariant)[keyof typeof ScreenVariant];

const modalName = ModalName.DeviceVerification;

const fallbackCodePeriod = 60;

const cancelAndClose = (deviceId: number) => {
  // a 404 only means the verification already expired
  api.device.cancelDeviceVerification(deviceId).catch(() => undefined);
  closeModal(modalName);
};

export const DeviceVerificationModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenDeviceVerificationModal | null>(null);
  const [screen, setScreen] = useState<ScreenVariantValue>(ScreenVariant.Default);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalName, (data) => {
      setModalData(data);
      setScreen(ScreenVariant.Default);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalName, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  const title = useMemo(() => {
    switch (screen) {
      case ScreenVariant.Block:
        return m.devices_modal_block_single_title();
      case ScreenVariant.Delete:
        return m.devices_modal_delete_single_title();
      default:
        return m.devices_modal_verification_title();
    }
  }, [screen]);

  return (
    <Modal
      id="device-verification-modal"
      size="small"
      title={title}
      isOpen={isOpen}
      onClose={() => {
        if (isPresent(modalData)) cancelAndClose(modalData.device.id);
      }}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && (
        <div className="content">
          {screen === ScreenVariant.Default && (
            <DefaultScreen device={modalData.device} onScreenChange={setScreen} />
          )}
          {screen === ScreenVariant.Block && (
            <ConfirmScreen
              content={m.devices_modal_verification_block_content()}
              submitText={m.devices_action_block()}
              testId="device-verification-block"
              action={() => api.device.bulkBlockDevices([modalData.device.id])}
              successMessage={m.devices_block_success()}
              errorMessage={m.devices_block_error()}
              onBack={() => setScreen(ScreenVariant.Default)}
            />
          )}
          {screen === ScreenVariant.Delete && (
            <ConfirmScreen
              content={m.devices_modal_verification_delete_content({
                name: modalData.device.name,
              })}
              submitText={m.controls_delete()}
              testId="device-verification-delete"
              action={() => api.device.bulkDeleteDevices([modalData.device.id])}
              successMessage={m.devices_delete_success()}
              errorMessage={m.devices_delete_error()}
              onBack={() => setScreen(ScreenVariant.Default)}
            />
          )}
        </div>
      )}
    </Modal>
  );
};

type DefaultScreenProps = {
  device: DeviceOverview;
  onScreenChange: (screen: ScreenVariantValue) => void;
};

const DefaultScreen = ({ device, onScreenChange }: DefaultScreenProps) => {
  const { secondsLeft, start: startTimer } = useTimer();

  const { data, isError } = useQuery({
    // outside the 'device' prefix so device invalidations don't start a new verification
    queryKey: ['device-verification', device.id],
    queryFn: () => api.device.startDeviceVerification(device.id),
    select: (response) => response.data,
    staleTime: 0,
    gcTime: 0,
    refetchInterval: (query) =>
      (query.state.data?.data.code_expires_in ?? fallbackCodePeriod) * 1000,
  });

  const { mutate: verify, isPending: isVerifying } = useMutation({
    mutationFn: api.device.bulkVerifyDevices,
    meta: { invalidate: ['device'] },
    onSuccess: () => {
      Snackbar.default(m.devices_verify_success());
      closeModal(modalName);
    },
    onError: () => Snackbar.error(m.devices_verify_error()),
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: startTimer is recreated on every render
  useEffect(() => {
    if (isPresent(data)) startTimer(data.code_expires_in);
  }, [data]);

  useEffect(() => {
    if (isError) {
      Snackbar.error(m.devices_modal_verification_load_error());
      closeModal(modalName);
    }
  }, [isError]);

  const digits = useMemo(
    () => (isPresent(data) ? [...data.code].map(Number) : []),
    [data],
  );

  const menuItems = useMemo(
    (): MenuItemsGroup[] => [
      {
        items: [
          {
            text: m.devices_modal_verification_leave_unverified(),
            icon: 'pending',
            testId: 'device-verification-leave-unverified',
            onClick: () => cancelAndClose(device.id),
          },
        ],
      },
      {
        items: [
          {
            text: m.devices_modal_verification_block(),
            icon: 'cancel',
            testId: 'device-verification-block-screen',
            onClick: () => onScreenChange(ScreenVariant.Block),
          },
          {
            text: m.devices_modal_verification_delete(),
            icon: 'delete',
            variant: 'danger',
            testId: 'device-verification-delete-screen',
            onClick: () => onScreenChange(ScreenVariant.Delete),
          },
        ],
      },
    ],
    [device.id, onScreenChange],
  );

  return (
    <>
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.devices_modal_verification_content()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      {isPresent(data) && <CodeRow values={digits} testId="device-verification-code" />}
      {!isPresent(data) && (
        <div className="loader">
          <LoaderSpinner size={52} variant="primary" />
        </div>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="line"></div>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="refresh">
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.devices_modal_verification_refresh()}
        </AppText>
        <AppText font={TextStyle.TBodySm600} color={ThemeVariable.FgDefault}>
          {m.devices_modal_verification_seconds({ seconds: secondsLeft })}
        </AppText>
      </div>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="line"></div>
      <Controls>
        <ButtonMenu
          variant="outlined"
          text={m.devices_modal_verification_actions()}
          iconRight="arrow-small"
          iconRightRotation="down"
          testId="device-verification-actions"
          disabled={isVerifying}
          menuItems={menuItems}
        />
        <div className="right">
          <Button
            variant="secondary"
            text={m.controls_cancel()}
            testId="device-verification-cancel"
            disabled={isVerifying}
            onClick={() => cancelAndClose(device.id)}
          />
          <Button
            text={m.devices_action_verify()}
            testId="device-verification-verify"
            loading={isVerifying}
            disabled={!isPresent(data)}
            onClick={() => verify([device.id])}
          />
        </div>
      </Controls>
    </>
  );
};

type ConfirmScreenProps = {
  content: string;
  submitText: string;
  testId: string;
  action: () => Promise<unknown>;
  successMessage: string;
  errorMessage: string;
  onBack: () => void;
};

const ConfirmScreen = ({
  content,
  submitText,
  testId,
  action,
  successMessage,
  errorMessage,
  onBack,
}: ConfirmScreenProps) => {
  const { mutate, isPending } = useMutation({
    mutationFn: action,
    meta: { invalidate: ['device'] },
    onSuccess: () => {
      Snackbar.default(successMessage);
      closeModal(modalName);
    },
    onError: () => Snackbar.error(errorMessage),
  });

  return (
    <>
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {content}
      </AppText>
      <Controls>
        <div className="right">
          <Button
            variant="secondary"
            text={m.controls_cancel()}
            testId={`${testId}-back`}
            disabled={isPending}
            onClick={onBack}
          />
          <Button
            variant="critical"
            text={submitText}
            testId={`${testId}-submit`}
            loading={isPending}
            onClick={() => mutate()}
          />
        </div>
      </Controls>
    </>
  );
};
