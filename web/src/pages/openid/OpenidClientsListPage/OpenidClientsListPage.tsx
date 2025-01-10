import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined, orderBy } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import { PageContainer } from '../../../shared/components/Layout/PageContainer/PageContainer';
import IconCheckmarkGreen from '../../../shared/components/svg/IconCheckmarkGreen';
import IconDeactivated from '../../../shared/components/svg/IconDeactivated';
import SvgIconPlusWhite from '../../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { EditButton } from '../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../shared/defguard-ui/components/Layout/EditButton/types';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { ConfirmModal } from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { NoData } from '../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Search } from '../../../shared/defguard-ui/components/Layout/Search/Search';
import { Select } from '../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../shared/defguard-ui/components/Layout/Select/types';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
} from '../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { useClipboard } from '../../../shared/hooks/useClipboard';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { OpenidClient } from '../../../shared/types';
import { OpenIdClientModal } from '../modals/OpenIdClientModal/OpenIdClientModal';

export const OpenidClientsListPage = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [deleteClientModalOpen, setDeleteClientModalOpen] = useState(false);
  const [deleteClient, setDeleteClient] = useState<OpenidClient | undefined>(undefined);
  const [searchValue, setSearchValue] = useState('');
  const {
    openid: { getOpenidClients, changeOpenidClientState, deleteOpenidClient },
  } = useApi();
  const setOpenIdClientModalState = useModalStore((state) => state.setOpenIdClientModal);

  const selectOptions = useMemo(
    (): SelectOption<FilterOption>[] => [
      {
        key: 1,
        label: LL.openidOverview.filterLabels.all(),
        value: FilterOption.ALL,
      },
      {
        key: 3,
        label: LL.openidOverview.filterLabels.enabled(),
        value: FilterOption.ENABLED,
      },
      {
        key: 2,
        label: LL.openidOverview.filterLabels.disabled(),
        value: FilterOption.DISABLED,
      },
    ],
    [LL],
  );

  const [selectedFilter, setSelectedFilter] = useState(FilterOption.ALL);

  const { mutate: deleteClientMutation, isPending: deleteClientLoading } = useMutation({
    mutationKey: [MutationKeys.DELETE_OPENID_CLIENT],
    mutationFn: deleteOpenidClient,
    onSuccess: () => {
      toaster.success(LL.openidOverview.deleteApp.messages.success());
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_CLIENTS],
      });
      setDeleteClientModalOpen(false);
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      setDeleteClientModalOpen(false);
      console.error(err);
    },
  });

  const { mutate: editClientStatusMutation } = useMutation({
    mutationFn: (client: OpenidClient) =>
      changeOpenidClientState({
        clientId: client.client_id,
        enabled: !client.enabled,
      }),
    onSuccess: (_, client) => {
      if (client.enabled) {
        toaster.success(LL.openidOverview.disableApp.messages.success());
      } else {
        toaster.success(LL.openidOverview.enableApp.messages.success());
      }
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_CLIENTS],
      });
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { data: clients, isLoading } = useQuery({
    queryKey: [QueryKeys.FETCH_CLIENTS],
    queryFn: getOpenidClients,
    refetchOnWindowFocus: false,
    refetchInterval: 15000,
  });

  const filteredClients = useMemo(() => {
    if (!clients || (clients && clients.length === 0)) return [];
    let res = orderBy(clients, ['name'], ['asc']);
    res = res.filter((c) => c.name.toLowerCase().includes(searchValue.toLowerCase()));
    switch (selectedFilter) {
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
  }, [clients, searchValue, selectedFilter]);

  const listHeaders = useMemo(() => {
    const res: ListHeader[] = [
      {
        key: 'name',
        text: LL.openidOverview.list.headers.name(),
        active: true,
        sortDirection: ListSortDirection.ASC,
      },
      {
        key: 'status',
        text: LL.openidOverview.list.headers.status(),
      },
      {
        key: 'actions',
        text: LL.openidOverview.list.headers.actions(),
        sortable: false,
      },
    ];
    return res;
  }, [LL.openidOverview.list.headers]);

  const listCells = useMemo(() => {
    const res: ListRowCell<OpenidClient>[] = [
      {
        key: 'name',
        render: (client) => <span>{client.name}</span>,
      },
      {
        key: 'status',
        render: (client) =>
          client.enabled ? (
            <>
              <IconCheckmarkGreen />{' '}
              <span>{LL.openidOverview.list.status.enabled()}</span>
            </>
          ) : (
            <>
              <IconDeactivated /> <span>{LL.openidOverview.list.status.disabled()}</span>
            </>
          ),
      },
      {
        key: 'actions',
        render: (client) => (
          <EditButton data-testid={`edit-openid-client-${client.id}`}>
            <EditButtonOption
              text={LL.openidOverview.list.editButton.edit()}
              onClick={() =>
                setOpenIdClientModalState({
                  visible: true,
                  viewMode: false,
                  client,
                })
              }
            />
            <EditButtonOption
              text={
                client.enabled
                  ? LL.openidOverview.list.editButton.disable()
                  : LL.openidOverview.list.editButton.enable()
              }
              onClick={() => editClientStatusMutation(client)}
            />
            <EditButtonOption
              data-testid="copy-openid-client-id"
              text={LL.openidOverview.list.editButton.copy()}
              onClick={() => {
                void writeToClipboard(
                  client.client_id,
                  LL.openidOverview.messages.copySuccess(),
                );
              }}
            />
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              text={LL.openidOverview.list.editButton.delete()}
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
  }, [
    writeToClipboard,
    LL.openidOverview.list.editButton,
    LL.openidOverview.list.status,
    LL.openidOverview.messages,
    editClientStatusMutation,
    setOpenIdClientModalState,
  ]);

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

  const renderSelectedFilter = useCallback(
    (selected: FilterOption): SelectSelectedValue => {
      const option = selectOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exists");
      return {
        key: selected,
        displayValue: option.label,
      };
    },
    [selectOptions],
  );

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilter !== FilterOption.ALL) {
      setSelectedFilter(FilterOption.ALL);
    }
  }, [breakpoint, selectOptions, selectedFilter]);

  return (
    <PageContainer id="openid-clients-list">
      <header>
        <h1>{LL.openidOverview.pageTitle()}</h1>
        <Search
          placeholder={LL.openidOverview.search.placeholder()}
          className="clients-search"
          initialValue={searchValue}
          debounceTiming={500}
          onDebounce={(value) => setSearchValue(value)}
        />
      </header>
      <section className="actions">
        <div className="clients-count">
          <span>{LL.openidOverview.clientCount()}</span>
          <div className="count" data-testid="clients-count">
            <span>{clients && clients.length > 0 ? clients.length : 0}</span>
          </div>
        </div>
        <div className="controls">
          {breakpoint === 'desktop' && (
            <Select
              renderSelected={renderSelectedFilter}
              options={selectOptions}
              selected={selectedFilter}
              onChangeSingle={(res) => setSelectedFilter(res)}
            />
          )}
          <Button
            data-testid="add-openid-client"
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
            text={breakpoint === 'desktop' ? LL.openidOverview.addNewApp() : undefined}
          />
        </div>
      </section>
      {(isLoading || isUndefined(clients)) && (
        <div className="list-loader">
          <LoaderSpinner size={180} />
        </div>
      )}
      {!isLoading && filteredClients.length === 0 && (
        <NoData customMessage={LL.openidOverview.messages.noClientsFound()} />
      )}
      {!isLoading && filteredClients?.length > 0 && (
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
          onDefaultRowClick={(client) =>
            setOpenIdClientModalState({
              visible: true,
              client,
              viewMode: true,
            })
          }
        />
      )}
      <OpenIdClientModal />
      <ConfirmModal
        type={ConfirmModalType.WARNING}
        title={LL.openidOverview.deleteApp.title()}
        submitText={LL.openidOverview.deleteApp.submit()}
        subTitle={LL.openidOverview.deleteApp.message({
          appName: deleteClient?.name || '',
        })}
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
