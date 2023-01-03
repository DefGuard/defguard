import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined, orderBy } from 'lodash-es';
import { useEffect, useState } from 'react';
import { useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { EditButton } from '../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../shared/components/layout/EditButton/EditButtonOption';
import LoaderSpinner from '../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import NoData from '../../../shared/components/layout/NoData/NoData';
import PageContainer from '../../../shared/components/layout/PageContainer/PageContainer';
import { Search } from '../../../shared/components/layout/Search/Search';
import {
  Select,
  SelectOption,
} from '../../../shared/components/layout/Select/Select';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
  VirtualizedList,
} from '../../../shared/components/layout/VirtualizedList/VirtualizedList';
import {
  IconCheckmarkGreen,
  IconDeactivated,
} from '../../../shared/components/svg';
import SvgIconPlusWhite from '../../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../../shared/constants';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { OpenidClient } from '../../../shared/types';
import { OpenIdClientModal } from '../modals/OpenIdClientModal/OpenIdClientModal';

export const OpenidClientsListPage = () => {
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [selectedFilter, setSelectedFilter] = useState(selectOptions[0]);
  const [deleteClientModalOpen, setDeleteClientModalOpen] = useState(false);
  const [deleteClient, setDeleteClient] = useState<OpenidClient | undefined>(
    undefined
  );
  const [searchValue, setSearchValue] = useState('');
  const {
    openid: { getOpenidClients, changeOpenidClientState, deleteOpenidClient },
    license: { getLicense },
  } = useApi();
  const setOpenIdClientModalState = useModalStore(
    (state) => state.setOpenIdClientModal
  );

  const { data: license } = useQuery([QueryKeys.FETCH_LICENSE], getLicense);

  const { mutate: deleteClientMutation, isLoading: deleteClientLoading } =
    useMutation([MutationKeys.DELETE_OPENID_CLIENT], deleteOpenidClient, {
      onSuccess: () => {
        toaster.success('Client removed.');
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
        setDeleteClientModalOpen(false);
      },
      onError: (err) => {
        toaster.error('Error has occurred.');
        setDeleteClientModalOpen(false);
        console.error(err);
      },
    });

  const { mutate: editClientStatusMutation } = useMutation(
    (client: OpenidClient) =>
      changeOpenidClientState({
        clientId: client.client_id,
        enabled: !client.enabled,
      }),
    {
      onSuccess: (_, client) => {
        if (client.enabled) {
          toaster.success('Client disabled.');
        } else {
          toaster.success('Client enabled.');
        }
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: (err) => {
        toaster.error('Error occurred.');
        console.error(err);
      },
    }
  );

  const hasAccess = useMemo(() => {
    return license?.openid || license?.enterprise;
  }, [license]);

  const { data: clients, isLoading } = useQuery(
    [QueryKeys.FETCH_CLIENTS],
    getOpenidClients,
    { enabled: hasAccess, refetchOnWindowFocus: false, refetchInterval: 15000 }
  );

  const filteredClients = useMemo(() => {
    if (!clients || (clients && clients.length === 0)) return [];
    let res = orderBy(clients, ['name'], ['asc']);
    res = res.filter((c) =>
      c.name.toLowerCase().includes(searchValue.toLowerCase())
    );
    switch (selectedFilter.value) {
      case FilterOption.ALL:
        break;
      case FilterOption.ENABLED:
        res = res.filter((c) => c.enabled);
        break;
      case FilterOption.DISABLED:
        res = res.filter((c) => !c.enabled);
        break;
    }
    return res;
  }, [clients, searchValue, selectedFilter.value]);

  const listHeaders = useMemo(() => {
    const res: ListHeader[] = [
      {
        key: 'name',
        text: 'Name',
        active: true,
        sortDirection: ListSortDirection.ASC,
      },
      {
        key: 'status',
        text: 'Status',
      },
      {
        key: 'actions',
        text: 'Actions',
        sortable: false,
      },
    ];
    return res;
  }, []);

  const listCells = useMemo(() => {
    const res: ListRowCell<OpenidClient>[] = [
      {
        key: 'name',
        render: (client) => <span>{client.name}</span>,
        onClick: (client) =>
          setOpenIdClientModalState({ visible: true, client, viewMode: true }),
      },
      {
        key: 'status',
        render: (client) =>
          client.enabled ? (
            <>
              <IconCheckmarkGreen /> <span>Enabled</span>
            </>
          ) : (
            <>
              <IconDeactivated /> <span>Disabled</span>
            </>
          ),
      },
      {
        key: 'actions',
        render: (client) => (
          <EditButton>
            <EditButtonOption
              text="Edit"
              onClick={() =>
                setOpenIdClientModalState({
                  visible: true,
                  viewMode: false,
                  client,
                })
              }
            />
            <EditButtonOption
              text={client.enabled ? 'Disable' : 'Enable'}
              onClick={() => editClientStatusMutation(client)}
            />
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              text="Delete"
              onClick={() => {
                setDeleteClient(client);
                setDeleteClientModalOpen(true);
              }}
            />
          </EditButton>
        ),
      },
    ];
    return res;
  }, [editClientStatusMutation, setOpenIdClientModalState]);

  const getListPadding = useMemo(() => {
    if (breakpoint === 'desktop') {
      return {
        left: 60,
        right: 60,
      };
    }
    return {
      left: 20,
      right: 20,
    };
  }, [breakpoint]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilter.value !== FilterOption.ALL) {
      setSelectedFilter(selectOptions[0]);
    }
  }, [breakpoint, selectedFilter.value]);

  return (
    <PageContainer id="openid-clients-list">
      <header>
        <h1>OpenID Apps</h1>
        <Search
          placeholder="Find app"
          className="clients-search"
          initialValue={searchValue}
          debounceTiming={500}
          onDebounce={(value) => setSearchValue(value)}
        />
      </header>
      <section className="actions">
        <div className="clients-count">
          <span>All apps</span>
          <div className="count" data-test="clients-count">
            <span>{clients && clients.length > 0 ? clients.length : 0}</span>
          </div>
        </div>
        <div className="controls">
          {breakpoint === 'desktop' && (
            <Select
              options={selectOptions}
              selected={selectedFilter}
              onChange={(o) => {
                if (o && !Array.isArray(o)) {
                  setSelectedFilter(o);
                }
              }}
            />
          )}
          <Button
            className="add-client"
            onClick={() =>
              setOpenIdClientModalState({
                visible: true,
                client: undefined,
                viewMode: false,
              })
            }
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconPlusWhite />}
            text="Add new"
            disabled={!hasAccess}
          />
        </div>
      </section>
      {!hasAccess && (
        <NoData customMessage="You don't have a license for this feature." />
      )}
      {(isLoading || isUndefined(clients)) && hasAccess && (
        <div className="list-loader">
          <LoaderSpinner size={180} />
        </div>
      )}
      {!isLoading && hasAccess && filteredClients.length === 0 && (
        <NoData customMessage="No results found." />
      )}
      {!isLoading && hasAccess && filteredClients?.length > 0 && (
        <VirtualizedList
          className="clients-list"
          data={filteredClients}
          headers={listHeaders}
          cells={listCells}
          rowSize={70}
          padding={getListPadding}
          headerPadding={{
            left: 50,
            right: 15,
          }}
        />
      )}
      <OpenIdClientModal />
      <ConfirmModal
        type={ConfirmModalType.WARNING}
        title="Delete client"
        submitText="Delete"
        subTitle={`Are you sure you want to delete ${deleteClient?.name}`}
        onSubmit={() => {
          if (!isUndefined(deleteClient)) {
            deleteClientMutation(deleteClient.client_id);
          }
        }}
        loading={deleteClientLoading}
        isOpen={deleteClientModalOpen}
        setIsOpen={setDeleteClientModalOpen}
      />
    </PageContainer>
  );
};

enum FilterOption {
  ALL = 1,
  ENABLED = 2,
  DISABLED = 3,
}

const selectOptions: SelectOption<FilterOption>[] = [
  {
    key: 1,
    label: 'All',
    value: FilterOption.ALL,
  },
  {
    key: 3,
    label: 'Enabled',
    value: FilterOption.ENABLED,
  },
  {
    key: 2,
    label: 'Disabled',
    value: FilterOption.DISABLED,
  },
];
