import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { saveAs } from 'file-saver';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCopy from '../../../../../../shared/components/svg/IconCopy';
import IconDownload from '../../../../../../shared/components/svg/IconDownload';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useAuthStore } from '../../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { MutationKeys } from '../../../../../../shared/mutations';

export const RecoveryCodesModal = () => {
  const { LL } = useI18nContext();
  const modalState = useModalStore((state) => state.recoveryCodesModal);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);

  return (
    <ModalWithTitle
      id="view-recovery-codes"
      title={LL.modals.recoveryCodes.title()}
      isOpen={modalState.visible}
      setIsOpen={(visible) => setModalState({ visible, codes: undefined })}
      disableClose={true}
      backdrop
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const codes = useModalStore((state) => state.recoveryCodesModal.codes);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);
  const {
    auth: {
      mfa: { enable },
    },
  } = useApi();
  const logOut = useAuthStore((state) => state.resetState);
  const { mutate, isPending } = useMutation({
    mutationKey: [MutationKeys.ENABLE_MFA],
    mutationFn: enable,
    onSuccess: () => {
      setModalState({ visible: false, codes: undefined });
      logOut();
    },
  });

  if (!codes) return null;

  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        {parse(LL.modals.recoveryCodes.infoMessage())}
      </MessageBox>
      <div className="codes" data-testid="recovery-codes">
        {codes.map((code) => (
          <p key={code}>{code}</p>
        ))}
      </div>
      <div className="actions">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconDownload />}
          text={LL.form.download()}
          onClick={() => {
            if (codes) {
              const blob = new Blob([codes.join('\n')], {
                type: 'text/plain;charset=utf-8',
              });
              saveAs(blob, 'recovery_codes.txt');
            }
          }}
        />
        <Button
          data-testid="copy-recovery"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconCopy />}
          text={LL.form.copy()}
          onClick={() => {
            if (codes) {
              void writeToClipboard(
                codes.join('\n'),
                LL.modals.recoveryCodes.messages.copied(),
              );
            }
          }}
        />
      </div>
      <div className="controls">
        <Button
          data-testid="accept-recovery"
          text={LL.modals.recoveryCodes.submit()}
          onClick={() => mutate()}
          styleVariant={ButtonStyleVariant.CONFIRM}
          size={ButtonSize.LARGE}
          loading={isPending}
        />
      </div>
    </>
  );
};
