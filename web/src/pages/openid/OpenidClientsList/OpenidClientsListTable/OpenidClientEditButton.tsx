import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { useNavigate } from 'react-router';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import OptionsPopover from '../../../../shared/components/layout/OptionsPopover/OptionsPopover';
import SvgIconDelete from '../../../../shared/components/svg/IconDelete';
import SvgIconEditAlt from '../../../../shared/components/svg/IconEditAlt';
import { deviceBreakpoints } from '../../../../shared/constants';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useOpenidClientStore } from '../../../../shared/hooks/store/useOpenidClientStore';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { OpenidClient } from '../../../../shared/types';

interface Props {
  client: OpenidClient;
}

const OpenidClientEditButton: React.FC<Props> = ({ client }) => {
  const [isEditOpen, setEditOpen] = useState(false);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const setOpenidClientViewEditMode = useOpenidClientStore(
    (state) => state.setEditMode
  );
  const setDeleteOpenidClientModal = useModalStore(
    (state) => state.setDeleteOpenidClientModal,
    shallow
  );
  const [isDeleteOpen, setDeleteOpen] = useState(false);
  const [referenceElement, setReferenceElement] =
    useState<HTMLButtonElement | null>(null);
  const {
    openid: { deleteOpenidClient, changeOpenidClientState },
  } = useApi();
  const queryClient = useQueryClient();
  const deleteClientMutation = useMutation(
    (client: OpenidClient) => deleteOpenidClient(client.id),
    {
      onSuccess: () => {
        setEditOpen(false);
        setDeleteOpen(false);
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: () => {
        setDeleteOpen(false);
      },
    }
  );
  const handleDeleteClick = () => {
    if (breakpoint !== 'mobile') {
      setEditOpen(false);
      setDeleteOpen(true);
    } else {
      setEditOpen(false);
      setDeleteOpenidClientModal({
        visible: true,
        client: client,
        onSuccess: onDeleteClientSuccess,
      });
    }
  };
  const onDeleteClientSuccess = () => {
    queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
  };

  const [isEnableOpen, setEnableModalOpen] = useState(false);

  const changeClientStateMutation = useMutation(
    (client: OpenidClient) =>
      changeOpenidClientState({
        clientId: client.client_id,
        enabled: !client.enabled,
      }),
    {
      onSuccess: () => {
        setEnableModalOpen(false);
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: () => {
        setEnableModalOpen(false);
      },
    }
  );
  const setEnableOpenidClientModal = useModalStore(
    (state) => state.setEnableOpenidClientModal,
    shallow
  );
  const handleEnableClick = () => {
    if (breakpoint !== 'mobile') {
      setEditOpen(false);
      setEnableModalOpen(true);
    } else {
      setEditOpen(false);
      setEnableOpenidClientModal({
        visible: true,
        client: client,
        onSuccess: onDeleteClientSuccess,
      });
    }
  };
  return (
    <>
      <button
        type="button"
        ref={setReferenceElement}
        onClick={() => setEditOpen(true)}
        className="client-edit"
      >
        <SvgIconEditAlt />
      </button>
      {referenceElement ? (
        <OptionsPopover
          items={[
            <button
              key="edit"
              onClick={() => {
                setOpenidClientViewEditMode(true);
                navigate(`${client.id}`);
              }}
            >
              Edit
            </button>,
            <button key="state-webhook" onClick={handleEnableClick}>
              {client.enabled ? 'Disable' : 'Enable'}
            </button>,
            <button
              key="delete"
              className="warning"
              onClick={handleDeleteClick}
            >
              Delete
            </button>,
          ]}
          referenceElement={referenceElement}
          isOpen={isEditOpen}
          setIsOpen={setEditOpen}
          popperOptions={{ placement: 'left' }}
        />
      ) : null}
      {isDeleteOpen && breakpoint !== 'mobile' ? (
        <div className="delete-client-overlay">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.STANDARD}
            onClick={() => setDeleteOpen(false)}
            text="Cancel"
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM_WARNING}
            text="Delete client"
            icon={<SvgIconDelete />}
            onClick={() => deleteClientMutation.mutate(client)}
          />
        </div>
      ) : null}
      {isEnableOpen && breakpoint !== 'mobile' ? (
        <div className="delete-client-overlay">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.STANDARD}
            onClick={() => setEnableModalOpen(false)}
            text="Cancel"
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={
              client.enabled
                ? ButtonStyleVariant.WARNING
                : ButtonStyleVariant.PRIMARY
            }
            text={client.enabled ? 'Disable' : 'Enable'}
            onClick={() => changeClientStateMutation.mutate(client)}
          />
        </div>
      ) : null}
    </>
  );
};

export default OpenidClientEditButton;
