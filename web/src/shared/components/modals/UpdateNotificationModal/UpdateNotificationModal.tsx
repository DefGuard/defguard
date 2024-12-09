// eslint-disable-next-line simple-import-sort/imports
import { shallow } from 'zustand/shallow';

import { Modal } from '../../../defguard-ui/components/Layout/modals/Modal/Modal';
import { useUpdatesStore } from '../../../hooks/store/useUpdatesStore';
import { UpdateNotificationModalIcons } from './components/UpdateNotificationModalIcons';
import { Button } from '../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../defguard-ui/components/Layout/Button/types';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { RenderMarkdown } from '../../Layout/RenderMarkdown/RenderMarkdown';
import './style.scss';
import dayjs from 'dayjs';

export const UpdateNotificationModal = () => {
  const isOpen = useUpdatesStore((s) => s.modalVisible);
  const close = useUpdatesStore((s) => s.closeModal, shallow);

  return (
    <Modal
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      className="updates-modal"
      id="updates-modal"
      disableClose
    >
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.updatesNotification;
  const data = useUpdatesStore((s) => s.update);
  const setStore = useUpdatesStore((s) => s.setStore, shallow);
  if (!data) return null;
  return (
    <div className="content-wrapper">
      <div className="top">
        <div className="header">
          <UpdateNotificationModalIcons variant="update" />
          <p className="title">{localLL.header.title()}</p>
        </div>
        <div className="info">
          <p className="version">
            {localLL.header.newVersion({
              version: data.version,
            })}
          </p>
          {data.critical && (
            <div className="badge">
              <UpdateNotificationModalIcons variant="alert" />
              <span>{localLL.header.criticalBadge()}</span>
            </div>
          )}
        </div>
      </div>
      <div className="bottom">
        <div className="content">
          <RenderMarkdown content={data.notes} />
        </div>
        <div className="controls">
          <Button
            className="close"
            styleVariant={ButtonStyleVariant.STANDARD}
            size={ButtonSize.LARGE}
            text={LL.common.controls.dismiss()}
            onClick={() => {
              setStore({
                modalVisible: false,
                dismissal: {
                  dismissedAt: dayjs.utc().toISOString(),
                  version: data.version,
                },
              });
            }}
          />
          <a href={data.releaseLink} target="_blank" rel="noreferrer noopener">
            <Button
              styleVariant={ButtonStyleVariant.PRIMARY}
              size={ButtonSize.LARGE}
              text={localLL.controls.visitRelease()}
              // if not given it prevent's default
              onClick={() => {}}
            />
          </a>
        </div>
      </div>
    </div>
  );
};
