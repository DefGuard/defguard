import './style.scss';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useProfileDetailsWarningModal } from './hooks/useProfileDetailsWarningModal';

export const ProfileDetailsWarningModals = () => {
  const warningModals = useProfileDetailsWarningModal((state) => state);
  const { LL } = useI18nContext();
  const localLL = LL.userPage.userDetails.warningModals;

  return (
    <>
      <ModalWithTitle
        className="change-warning-modal"
        backdrop
        isOpen={warningModals.usernameChange.open}
        onClose={() => warningModals.close('usernameChange')}
        title="Warning"
      >
        <p>{localLL.content.usernameChange()}</p>
        <div className="buttons">
          <Button
            onClick={() => warningModals.accept('usernameChange')}
            text={localLL.buttons.proceed()}
            styleVariant={ButtonStyleVariant.DELETE}
          />
          <Button
            onClick={() => warningModals.close('usernameChange')}
            text={localLL.buttons.cancel()}
          />
        </div>
      </ModalWithTitle>
      <ModalWithTitle
        className="change-warning-modal"
        backdrop
        isOpen={warningModals.emailChange.open}
        onClose={() => warningModals.close('emailChange')}
        title="Warning"
      >
        <p>{localLL.content.emailChange()}</p>
        <div className="buttons">
          <Button
            onClick={() => warningModals.accept('emailChange')}
            text={localLL.buttons.proceed()}
            styleVariant={ButtonStyleVariant.DELETE}
          />
          <Button
            onClick={() => warningModals.close('emailChange')}
            text={localLL.buttons.cancel()}
          />
        </div>
      </ModalWithTitle>
    </>
  );
};
