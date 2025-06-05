import './style.scss';

import { useMutation, useQuery } from '@tanstack/react-query';
import { orderBy, range } from 'lodash-es';
import { useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ListCellText } from '../../../../shared/components/Layout/ListCellText/ListCellText';
import { ListHeader } from '../../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../../shared/components/Layout/ListHeader/types';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { NoData } from '../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { ListSortDirection } from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import queryClient from '../../../../shared/query-client';
import { ActivityStream } from '../../../../shared/types';
import { CreateAuditStreamModal } from './modals/CreateAuditStreamModal/CreateAuditStreamModal';
import { useCreateAuditStreamModalStore } from './modals/CreateAuditStreamModal/store';
import { LogStashHttpStreamCEModal } from './modals/LogStashHttpStreamCEModal/LogStashHttpStreamCEModal';
import { useVectorHttpStreamCEModal } from './modals/VectorHttpStreamCEModal/store';
import { VectorHttpStreamCEModal } from './modals/VectorHttpStreamCEModal/VectorHttpStreamCEModal';
import {
  activityStreamToLabel,
  activityStreamTypeToLabel,
} from './utils/auditStreamToLabel';

export const ActivityStreamSettings = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.auditStreamSettings;

  return (
    <>
      <section id="audit-stream-settings">
        <header>
          <h2>{localLL.title()}</h2>
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
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.auditStreamSettings;

  const {
    activityStream: { getActivityStreams },
  } = useApi();

  const openCreateModal = useCreateAuditStreamModalStore((s) => s.open, shallow);

  const { data: auditStreams, isLoading: streamsLoading } = useQuery({
    queryFn: getActivityStreams,
    queryKey: ['activity_stream'],
    placeholderData: (perv) => perv,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
    select: (data) => orderBy(data, (row) => row.name.toLowerCase(), ['asc']),
  });

  const [activeSortKey] = useState<keyof ActivityStream>('name');
  const [sortDirection, setSortDirection] = useState<ListSortDirection>(
    ListSortDirection.ASC,
  );

  const listHeaders = useMemo(
    (): ListHeaderColumnConfig<ActivityStream>[] => [
      {
        key: 'name',
        enabled: true,
        sortKey: 'name',
        label: localLL.list.headers.name(),
      },
      {
        key: 'destination',
        enabled: false,
        sortKey: 'stream_type',
        label: localLL.list.headers.destination(),
      },
      {
        key: 'edit',
        enabled: false,
        label: LL.common.controls.edit(),
      },
    ],
    [LL.common.controls, localLL.list.headers],
  );

  return (
    <div className="audit-stream-list">
      <div className="controls">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.addNew()}
          icon={<SvgIconPlus />}
          className="add"
          onClick={() => {
            openCreateModal();
          }}
        />
      </div>
      <div className="list-header">
        <ListHeader
          headers={listHeaders}
          activeKey={activeSortKey}
          sortDirection={sortDirection}
          onChange={(_, direction) => {
            setSortDirection(direction);
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
          <NoData
            customMessage={LL.settingsPage.auditStreamSettings.list.noData()}
            messagePosition="center"
          />
        )}
      </div>
    </div>
  );
};

type ListItemsProps = {
  stream: ActivityStream;
};

const ListItem = ({ stream }: ListItemsProps) => {
  return (
    <div className="audit-stream list-item">
      <div className="cell name">
        <ListCellText text={stream.name} />
      </div>
      <div className="cell destination">
        <ListCellText text={activityStreamTypeToLabel(stream.stream_type)} />
      </div>
      <div className="cell edit">
        <EditListItem stream={stream} />
      </div>
    </div>
  );
};

type EditProps = {
  stream: ActivityStream;
};

const EditListItem = ({ stream }: EditProps) => {
  const openVectorHttpStreamModal = useVectorHttpStreamCEModal((s) => s.open, shallow);
  const { LL } = useI18nContext();
  const toast = useToaster();
  const {
    activityStream: { deleteActivityStream },
  } = useApi();

  const { mutate: deleteStreamMutation, isPending: isDeleting } = useMutation({
    mutationFn: deleteActivityStream,
    onSuccess: () => {
      toast.success(
        LL.settingsPage.auditStreamSettings.messages.destinationCrud.delete({
          destination: activityStreamToLabel(stream),
        }),
      );
      void queryClient.invalidateQueries({
        queryKey: ['activity_stream'],
      });
    },
    onError: (e) => {
      toast.error(LL.messages.error());
      console.error(e);
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
