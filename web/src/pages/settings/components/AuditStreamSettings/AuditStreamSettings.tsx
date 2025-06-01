import './style.scss';

import { useMutation, useQuery } from '@tanstack/react-query';
import { range } from 'lodash-es';
import Skeleton from 'react-loading-skeleton';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { NoData } from '../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../shared/hooks/useApi';
import queryClient from '../../../../shared/query-client';
import { AuditStream } from '../../../../shared/types';
import { CreateAuditStreamModal } from './modals/CreateAuditStreamModal/CreateAuditStreamModal';
import { useCreateAuditStreamModalStore } from './modals/CreateAuditStreamModal/store';
import { LogStashHttpStreamCEModal } from './modals/LogStashHttpStreamCEModal/LogStashHttpStreamCEModal';
import { useVectorHttpStreamCEModal } from './modals/VectorHttpStreamCEModal/store';
import { VectorHttpStreamCEModal } from './modals/VectorHttpStreamCEModal/VectorHttpStreamCEModal';
import { auditStreamToLabel } from './utils/auditStreamToLabel';

export const AuditStreamSettings = () => {
  return (
    <>
      <section id="audit-stream-settings">
        <header>
          <h2>Audit Logs Streaming</h2>
        </header>
        <AuditStreamList />
      </section>
      <CreateAuditStreamModal />
      <VectorHttpStreamCEModal />
      <LogStashHttpStreamCEModal />
    </>
  );
};

const AuditStreamList = () => {
  const {
    auditStream: { getAuditStreams },
  } = useApi();

  const openCreateModal = useCreateAuditStreamModalStore((s) => s.open, shallow);

  const { data: auditStreams, isLoading: streamsLoading } = useQuery({
    queryFn: getAuditStreams,
    queryKey: ['audit_stream'],
    placeholderData: (perv) => perv,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
  });

  return (
    <div className="audit-stream-list">
      <div className="controls">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Add new"
          icon={<SvgIconPlus />}
          className="add"
          onClick={() => {
            openCreateModal();
          }}
        />
      </div>
      <div className="list">
        {!isPresent(auditStreams) && streamsLoading && (
          <div className="skeletons">
            {range(6).map((index) => (
              <Skeleton key={index} />
            ))}
          </div>
        )}
        {isPresent(auditStreams) && (
          <ul>
            {auditStreams.map((stream) => (
              <li key={stream.id}>
                <ListItem stream={stream} />
              </li>
            ))}
          </ul>
        )}
        {isPresent(auditStreams) && auditStreams.length === 0 && (
          <NoData customMessage="No audit streams" messagePosition="center" />
        )}
      </div>
    </div>
  );
};

type ListItemsProps = {
  stream: AuditStream;
};

const ListItem = ({ stream }: ListItemsProps) => {
  return (
    <div className="audit-stream list-item">
      <div className="cell name">
        <p>{stream.name ?? auditStreamToLabel(stream)}</p>
      </div>
      <div className="cell edit">
        <EditListItem stream={stream} />
      </div>
    </div>
  );
};

type EditProps = {
  stream: AuditStream;
};

const EditListItem = ({ stream }: EditProps) => {
  const openVectorHttpStreamModal = useVectorHttpStreamCEModal((s) => s.open, shallow);
  const { LL } = useI18nContext();
  const {
    auditStream: { deleteAuditStream },
  } = useApi();

  const { mutate: deleteStreamMutation, isPending: isDeleting } = useMutation({
    mutationFn: deleteAuditStream,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ['audit_stream'],
      });
    },
  });

  return (
    <EditButton>
      <EditButtonOption
        text={LL.common.controls.edit()}
        onClick={() => {
          openVectorHttpStreamModal(stream);
        }}
        disabled={isDeleting}
      />
      <EditButtonOption
        text={LL.common.controls.delete()}
        styleVariant={EditButtonOptionStyleVariant.WARNING}
        onClick={() => {
          deleteStreamMutation(stream.id);
        }}
        disabled={isDeleting}
      />
    </EditButton>
  );
};
