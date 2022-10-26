import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import IconButton from '../../../../../shared/components/layout/IconButton/IconButton';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import NoData from '../../../../../shared/components/layout/NoData/NoData';
import { RowBox } from '../../../../../shared/components/layout/RowBox/RowBox';
import SvgIconKey from '../../../../../shared/components/svg/IconKey';
import SvgIconTrash from '../../../../../shared/components/svg/IconTrash';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../shared/mutations';
import { SecurityKey } from '../../../../../shared/types';
import { convertFromStringToBuffer } from '../../../../../shared/utils/convertFromStringToBuffer';
import { toaster } from '../../../../../shared/utils/toaster';

export const ManageWebAuthNKeysModal = () => {
  const user = useUserProfileV2Store((state) => state.user);
  const modalState = useModalStore((state) => state.manageWebAuthNKeysModal);
  const setModalState = useModalStore((state) => state.setState);
  const [waitingForSecurityKey, setWaitingForSecurityKey] = useState(false);

  const {
    auth: {
      mfa: {
        webauthn: {
          register: { start, finish },
        },
      },
    },
  } = useApi();

  const { mutate: registerKeyFinish, isLoading: registerKeyFinishLoading } =
    useMutation([MutationKeys.REGISTER_SECURITY_KEY_FINISH], finish, {
      onSuccess: () => {
        toaster.success('Security key added.');
      },
    });

  const { mutate: registerKeyStart, isLoading: registerKeyStartLoading } =
    useMutation([MutationKeys.REGISTER_SECURITY_KEY_START], start, {
      onSuccess: async (data) => {
        // eslint-disable-next-line prettier/prettier
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        if (data.publicKey) {
          data.publicKey.challenge = convertFromStringToBuffer(
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            data.publicKey.challenge
          );
          data.publicKey.user.id = convertFromStringToBuffer(
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            data.publicKey.user.id
          );
        }
        setWaitingForSecurityKey(true);
        const keyResponse = await navigator.credentials
          .create(data)
          .catch((e) => {
            console.error(e);
            return null;
          });
        setWaitingForSecurityKey(false);
        if (keyResponse) {
          registerKeyFinish(keyResponse);
        } else {
          toaster.error('Failed to get key response, please try again.');
          setModalState({ manageWebAuthNKeysModal: { visible: false } });
        }
      },
    });

  return (
    <ModalWithTitle
      backdrop
      title="Security keys"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalState({ manageWebAuthNKeysModal: { visible: visibility } })
      }
    >
      {user?.security_keys.map((key) => (
        <KeyRow key={key.id} data={key} />
      ))}
      {user?.security_keys.length === 0 ? <NoData /> : null}
      <Button
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
        loading={
          registerKeyStartLoading ||
          registerKeyFinishLoading ||
          waitingForSecurityKey
        }
        onClick={() => registerKeyStart()}
        text="Register new security key"
      />
    </ModalWithTitle>
  );
};

interface KeyRowProps {
  data: SecurityKey;
}
const KeyRow = ({ data }: KeyRowProps) => {
  return (
    <RowBox className="security-key">
      <SvgIconKey />
      <p>{data.name}</p>
      <IconButton onClick={() => console.log('delete key')}>
        <SvgIconTrash />
      </IconButton>
    </RowBox>
  );
};
