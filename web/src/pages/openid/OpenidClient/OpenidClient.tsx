import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { AnimatePresence, motion, Variants } from 'framer-motion';
import React, { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Subject } from 'rxjs';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import IconButton from '../../../shared/components/layout/IconButton/IconButton';
import OptionsPopover from '../../../shared/components/layout/OptionsPopover/OptionsPopover';
import Tabs, { Tab } from '../../../shared/components/layout/Tabs/Tabs';
import SvgIconCheckmarkWhite from '../../../shared/components/svg/IconCheckmarkWhite';
import SvgIconEdit from '../../../shared/components/svg/IconEdit';
import SvgIconEditAlt from '../../../shared/components/svg/IconEditAlt';
import { deviceBreakpoints } from '../../../shared/constants';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import { useOpenidClientStore } from '../../../shared/hooks/store/useOpenidClientStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { OpenidClient } from '../../../shared/types';
import OpenidClientForm from '../OpenidClientEdit/OpenidClientForm/OpenidClientForm';
import OpenidClientDetail from './OpenidCLientDetails/OpenidClientDetails';

interface Props {
  clientData?: OpenidClient;
}

const OpenIDClient: React.FC<Props> = ({ clientData }) => {
  const navigate = useNavigate();
  const { id } = useParams();
  const [editMode, setEditMode] = useOpenidClientStore(
    (state) => [state.editMode, state.setEditMode],
    shallow
  );
  const setDeleteClientModal = useModalStore(
    (state) => state.setDeleteOpenidClientModal,
    shallow
  );
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const setNavigationOpenidClient = useNavigationStore(
    (state) => state.setNavigationOpenidClient
  );
  const {
    openid: { getOpenidClient },
  } = useApi();

  const [saveEditSubject, setSaveEditSubject] = useState<
    Subject<unknown> | undefined
  >();

  const { data } = useQuery(
    [QueryKeys.FETCH_CLIENTS, id],
    () => {
      if (id) {
        return getOpenidClient(id);
      }
    },
    {
      enabled: typeof id === 'string',
      onSuccess: (data) => {
        if (data) {
          setNavigationOpenidClient(data);
        }
      },
    }
  );

  const client = useMemo(() => (data ? data : clientData), [data, clientData]);

  useEffect(() => {
    if (!saveEditSubject) {
      setSaveEditSubject(new Subject());
    }
  }, [saveEditSubject]);

  const getTabs: Tab[] = useMemo(
    (): Tab[] => [
      {
        title: 'App details',
        node: client ? <OpenidClientDetail client={client} /> : null,
      },
    ],
    [client]
  );

  const getHeaderText = useMemo(() => {
    if (editMode) {
      return 'Edit app';
    }
    if (client) {
      return client.name;
    }
    return '';
  }, [editMode, client]);

  if (!client || !saveEditSubject) return null;

  return (
    <section id="client-profile">
      <AnimatePresence mode="wait">
        <motion.header
          initial="hidden"
          animate="show"
          exit="hidden"
          variants={standardVariants}
          key={editMode ? 'edit-mode' : 'showcase-mode'}
        >
          {breakpoint === 'mobile' ? (
            <>
              <div className={'mobile-controls admin'}>
                {editMode ? (
                  <div className="edit-controls">
                    <Button
                      size={ButtonSize.SMALL}
                      styleVariant={ButtonStyleVariant.STANDARD}
                      text="Cancel"
                      onClick={() => setEditMode(false)}
                    />
                    <Button
                      size={ButtonSize.SMALL}
                      styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
                      icon={<SvgIconCheckmarkWhite />}
                      text="Save changes"
                      onClick={() => saveEditSubject.next({})}
                    />
                    <AdminCommandsButton client={client} />
                  </div>
                ) : null}
                {editMode ? null : (
                  <IconButton
                    className="edit-button"
                    onClick={() => setEditMode(true)}
                  >
                    <SvgIconEdit />
                  </IconButton>
                )}
              </div>
            </>
          ) : null}
          {breakpoint !== 'mobile' ? (
            <>
              <motion.h1>{getHeaderText}</motion.h1>
              {!editMode ? (
                <motion.div className="edit-button-container">
                  <Button
                    className="small"
                    onClick={() => setEditMode(true)}
                    size={ButtonSize.SMALL}
                    icon={<SvgIconEdit />}
                    text="Edit app"
                  />
                </motion.div>
              ) : null}
              {editMode ? (
                <motion.div className="controls">
                  <motion.div className="admin-controls">
                    <Button
                      size={ButtonSize.SMALL}
                      styleVariant={ButtonStyleVariant.WARNING}
                      text="Delete app"
                      onClick={() =>
                        setDeleteClientModal({
                          visible: true,
                          client: client,
                          onSuccess: () =>
                            navigate('/admin/openid', { replace: true }),
                        })
                      }
                    />
                  </motion.div>
                  <motion.div className="edit-controls">
                    <Button
                      size={ButtonSize.SMALL}
                      styleVariant={ButtonStyleVariant.STANDARD}
                      text="Cancel"
                      onClick={() => setEditMode(false)}
                    />
                    <Button
                      size={ButtonSize.SMALL}
                      styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
                      icon={<SvgIconCheckmarkWhite />}
                      text="Save changes"
                      onClick={() => saveEditSubject.next({})}
                    />
                  </motion.div>
                </motion.div>
              ) : null}
            </>
          ) : null}
        </motion.header>
      </AnimatePresence>
      <div className="content">
        <AnimatePresence mode="wait">
          {editMode ? (
            <motion.div
              className="container-with-title edit-client-data"
              key="profile-edit"
              initial="hidden"
              animate="show"
              exit="hidden"
              variants={standardVariants}
            >
              <header>
                <h3 className="title">App details</h3>
              </header>
              <motion.div className="container-basic">
                <OpenidClientForm
                  client={client}
                  saveSubject={saveEditSubject}
                />
              </motion.div>
            </motion.div>
          ) : null}
          {!editMode ? (
            <Tabs
              tabs={getTabs}
              motionUnderlineID="OpenidClientTabsUnderline"
              key="profile-tabs"
              initial="hidden"
              animate="show"
              exit="hidden"
              variants={standardVariants}
            />
          ) : null}
        </AnimatePresence>
      </div>
    </section>
  );
};

export default OpenIDClient;

const standardVariants: Variants = {
  hidden: {
    opacity: 0,
    transition: {
      duration: 0.2,
    },
  },
  show: {
    opacity: 1,
    transition: {
      duration: 0.2,
    },
  },
};

interface AdminCommandsButtonProps {
  client: OpenidClient;
}

const AdminCommandsButton: React.FC<AdminCommandsButtonProps> = ({
  client,
}) => {
  const navigate = useNavigate();
  const [menuOpen, setMenuOpen] = useState(false);
  const [refElement, setRefElement] = useState<HTMLButtonElement | null>(null);
  const setDeleteClientModal = useModalStore(
    (state) => state.setDeleteOpenidClientModal,
    shallow
  );

  const getOptionsItems = useMemo(() => {
    const res = [
      <button
        className="warning"
        key="delete-client"
        onClick={() => {
          setDeleteClientModal({
            visible: true,
            client: client,
            onSuccess: () => navigate('/admin/openid', { replace: true }),
          });
          setMenuOpen(false);
        }}
      >
        Delete app
      </button>,
    ];
    return res;
  }, [navigate, setDeleteClientModal, client]);

  return (
    <>
      <IconButton ref={setRefElement} className="admin-button">
        <SvgIconEditAlt />
      </IconButton>
      {refElement ? (
        <OptionsPopover
          referenceElement={refElement}
          items={getOptionsItems}
          isOpen={menuOpen}
          setIsOpen={setMenuOpen}
          popperOptions={{ placement: 'left' }}
        />
      ) : null}
    </>
  );
};
