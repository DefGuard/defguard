import './style.scss';

import clsx from 'clsx';
import { useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { RadioButton } from '../../../../../../shared/defguard-ui/components/Layout/RadioButton/Radiobutton';
import { ActivityStreamType } from '../../../../../../shared/types';
import { activityStreamTypeToLabel } from '../../utils/activityStreamToLabel';
import { useLogstashHttpStreamCEModalStore } from '../LogStashHttpStreamCEModal/store';
import { useVectorHttpStreamCEModal } from '../VectorHttpStreamCEModal/store';
import { useCreateActivityStreamModalStore } from './store';

export const CreateActivityStreamModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.auditStreamSettings.modals.selectDestination;
  const [close, reset] = useCreateActivityStreamModalStore(
    (s) => [s.close, s.reset],
    shallow,
  );
  const isOpen = useCreateActivityStreamModalStore((s) => s.visible);

  return (
    <ModalWithTitle
      title={localLL.title()}
      id="create-audit-stream-modal"
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

const availableTypes: ActivityStreamType[] = ['vector_http', 'logstash_http'];

const ModalContent = () => {
  const { LL } = useI18nContext();

  const closeModal = useCreateActivityStreamModalStore((s) => s.close, shallow);
  const openCreateLogstash = useLogstashHttpStreamCEModalStore((s) => s.open, shallow);
  const openCreateVector = useVectorHttpStreamCEModal((s) => s.open, shallow);

  const [currentStreamType, setStreamType] = useState<ActivityStreamType>('vector_http');

  return (
    <>
      <div className="audit-stream-types">
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
              <p className="label">{activityStreamTypeToLabel(streamType)}</p>
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
