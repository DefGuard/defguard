import './style.scss';

import { useMutation } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { saveAs } from 'file-saver';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/types';
import { MessageBox } from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/components/layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { IconCopy, IconDownload } from '../../../../../../shared/components/svg';
import { useAuthStore } from '../../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
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
  const { LL } = useI18nContext();
  const codes = useModalStore((state) => state.recoveryCodesModal.codes);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);
  const {
    auth: {
      mfa: { enable },
    },
  } = useApi();
  const logOut = useAuthStore((state) => state.resetState);
  const { mutate, isLoading } = useMutation([MutationKeys.ENABLE_MFA], enable, {
    onSuccess: () => {
      setModalState({ visible: false, codes: undefined });
      logOut();
    },
  });
  const toaster = useToaster();

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
              clipboard
                .write(codes.join('\n'))
                .then(() => {
                  toaster.success(LL.modals.recoveryCodes.messages.copied());
                })
                .catch((err) => {
                  toaster.error(LL.messages.clipboardError());
                  console.error(err);
                });
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
          loading={isLoading}
        />
      </div>
    </>
  );
};
