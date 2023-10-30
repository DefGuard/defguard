import { useMutation } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { SMTPError } from '../../../../../shared/types';

type Props = {
  isOpen: boolean;
  onOpenChange: (v: boolean) => void;
};

export const SendSupportDataModal = ({ isOpen, onOpenChange }: Props) => {
  const { LL } = useI18nContext();
  const {
    mail: { sendSupportMail },
  } = useApi();
  const toaster = useToaster();

  const { mutate: sendMail, isLoading: mailLoading } = useMutation([], sendSupportMail, {
    onSuccess: () => {
      toaster.success(LL.supportPage.debugDataCard.mailSent());
      onOpenChange(false);
    },
    onError: (err: SMTPError) => {
      toaster.error(
        `${LL.supportPage.debugDataCard.mailError()}`,
        `${err.response?.data.error}`,
      );
      console.error(err);
    },
  });

  return (
    <ConfirmModal
      type={ConfirmModalType.NORMAL}
      loading={mailLoading}
      title={LL.supportPage.modals.confirmDataSend.title()}
      subTitle={LL.supportPage.modals.confirmDataSend.subTitle()}
      submitText={LL.supportPage.modals.confirmDataSend.submit()}
      isOpen={isOpen}
      onClose={() => onOpenChange(false)}
      onSubmit={() => {
        sendMail();
      }}
    />
  );
};
