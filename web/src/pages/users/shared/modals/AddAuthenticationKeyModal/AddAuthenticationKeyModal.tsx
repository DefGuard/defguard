import './style.scss';

import classNames from 'classnames';
import { useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconAuthenticationKey from '../../../../../shared/components/svg/IconAuthenticationKey';
import SvgIconNavYubikey from '../../../../../shared/components/svg/IconNavYubikey';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { AuthenticationKeyType } from '../../../../../shared/types';
import { AddAuthenticationKeyForm } from './components/AddAuthenticationKeyForm/AddAuthenticationKeyForm';
import { AddAuthenticationKeyYubikey } from './components/AddAuthenticationKeyYubikey/AddAuthenticationKeyYubikey';
import { AuthenticationKeyModalKeyType } from './types';
import { useAddAuthorizationKeyModal } from './useAddAuthorizationKeyModal';

export const AddAuthenticationKeyModal = () => {
  const { LL } = useI18nContext();

  const [close, reset] = useAddAuthorizationKeyModal((s) => [s.close, s.reset], shallow);

  const isProvisioning = useAddAuthorizationKeyModal((s) => s.provisioningInProgress);

  const isOpen = useAddAuthorizationKeyModal((s) => s.visible);

  return (
    <ModalWithTitle
      id="add-authentication-key-modal"
      backdrop
      title={LL.userPage.authenticationKeys.addModal.header()}
      onClose={close}
      afterClose={reset}
      isOpen={isOpen}
      disableClose={isProvisioning}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const locaLL = LL.userPage.authenticationKeys.addModal;
  const initSelection = useAddAuthorizationKeyModal((s) => s.selectedMode);
  const [selectedType, setSelectedType] =
    useState<AuthenticationKeyModalKeyType>(initSelection);

  const mappedFormKeyType = useMemo(() => {
    switch (selectedType) {
      case 'gpg':
        return AuthenticationKeyType.GPG;
      case 'ssh':
        return AuthenticationKeyType.SSH;
      case 'yubikey':
        return undefined;
    }
  }, [selectedType]);

  return (
    <>
      <div className="type-selection">
        <Label>{locaLL.keyType()}</Label>
        <div className="buttons">
          <Button
            text="SSH"
            icon={<IconAuthenticationKey />}
            styleVariant={
              selectedType === 'ssh'
                ? ButtonStyleVariant.PRIMARY
                : ButtonStyleVariant.LINK
            }
            onClick={() => {
              setSelectedType('ssh');
            }}
            className={classNames({
              active: selectedType === 'ssh',
            })}
          />
          <Button
            text="GPG"
            icon={<IconAuthenticationKey />}
            styleVariant={
              selectedType === 'gpg'
                ? ButtonStyleVariant.PRIMARY
                : ButtonStyleVariant.LINK
            }
            onClick={() => {
              setSelectedType('gpg');
            }}
            className={classNames({
              active: selectedType === 'gpg',
            })}
          />
          <Button
            text="YubiKey"
            icon={<SvgIconNavYubikey />}
            styleVariant={
              selectedType === 'yubikey'
                ? ButtonStyleVariant.PRIMARY
                : ButtonStyleVariant.LINK
            }
            onClick={() => {
              setSelectedType('yubikey');
            }}
            className={classNames({
              active: selectedType === 'yubikey',
            })}
          />
        </div>
      </div>
      {mappedFormKeyType && <AddAuthenticationKeyForm keyType={mappedFormKeyType} />}
      {selectedType === 'yubikey' && <AddAuthenticationKeyYubikey />}
    </>
  );
};
