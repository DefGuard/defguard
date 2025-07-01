import './style.scss';

import clsx from 'clsx';
import { useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { RadioButton } from '../../../../../../shared/defguard-ui/components/Layout/RadioButton/Radiobutton';
import { ActivityLogStreamType } from '../../../../../../shared/types';
import { activityLogStreamTypeToLabel } from '../../utils/activityLogStreamToLabel';
import { useLogstashHttpStreamCEModalStore } from '../LogStashHttpStreamCEModal/store';
import { useVectorHttpStreamCEModal } from '../VectorHttpStreamCEModal/store';
import { useCreateActivityLogStreamModalStore } from './store';

export const CreateActivityLogStreamModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.activityLogStreamSettings.modals.selectDestination;
  const [close, reset] = useCreateActivityLogStreamModalStore(
    (s) => [s.close, s.reset],
    shallow,
  );
  const isOpen = useCreateActivityLogStreamModalStore((s) => s.visible);

  return (
    <ModalWithTitle
      title={localLL.title()}
      id="create-activity-log-stream-modal"
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      afterClose={() => {
        reset();
      }}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const availableTypes: ActivityLogStreamType[] = ['vector_http', 'logstash_http'];

const ModalContent = () => {
  const { LL } = useI18nContext();

  const closeModal = useCreateActivityLogStreamModalStore((s) => s.close, shallow);
  const openCreateLogstash = useLogstashHttpStreamCEModalStore((s) => s.open, shallow);
  const openCreateVector = useVectorHttpStreamCEModal((s) => s.open, shallow);

  const [currentStreamType, setStreamType] =
    useState<ActivityLogStreamType>('vector_http');

  return (
    <>
      <div className="activity-log-stream-types">
        {availableTypes.map((streamType) => {
          const active = streamType === currentStreamType;
          return (
            <div
              className={clsx('stream-type', {
                active,
              })}
              key={streamType}
              onClick={() => {
                setStreamType(streamType);
              }}
            >
              <RadioButton active={active} />
              <p className="label">{activityLogStreamTypeToLabel(streamType)}</p>
            </div>
          );
        })}
      </div>
      <div className="controls">
        <Button
          onClick={() => {
            closeModal();
          }}
          text={LL.common.controls.close()}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.next()}
          onClick={() => {
            switch (currentStreamType) {
              case 'vector_http':
                openCreateVector();
                break;
              case 'logstash_http':
                openCreateLogstash();
                break;
            }
            closeModal();
          }}
        />
      </div>
    </>
  );
};
