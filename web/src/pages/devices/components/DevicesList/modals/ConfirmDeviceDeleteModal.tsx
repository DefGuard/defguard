import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { useDeleteStandaloneDeviceModal } from '../../../hooks/useDeleteStandaloneDeviceModal';

export const ConfirmDeviceDeleteModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.deleteStandaloneDevice;
  const [visible, device] = useDeleteStandaloneDeviceModal(
    (s) => [s.visible, s.device],
    shallow,
  );
  const [close, reset] = useDeleteStandaloneDeviceModal(
    (s) => [s.close, s.reset],
    shallow,
  );

  const isOpen = visible && device !== undefined;

  return (
    <ConfirmModal
      isOpen={isOpen}
      title={localLL.title()}
      subTitle={localLL.content({
        name: (device?.name as string) ?? '',
      })}
      submitText={LL.common.controls.delete()}
      cancelText={LL.common.controls.cancel()}
      onSubmit={() => {
        console.warn('Delete device not implemented!');
      }}
      onClose={close}
      afterClose={reset}
    />
  );
};
